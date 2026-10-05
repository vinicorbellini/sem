//! ABAP front end: lowers an ABAP source into [`FileFacts`], plus ABAP's
//! data for the shared stages: names are case-insensitive (folded to lower
//! case), a global class, interface, function module or report is visible
//! repo-wide while a local class, form or module stays in its object (the
//! files of one class or function group), classes are types whose methods
//! live in an impl and whose `INHERITING FROM` base is searched for what
//! they lack, a method calls its own class's methods with no receiver
//! written, and `CALL FUNCTION 'X'` and `PERFORM f` call by name. Receivers
//! are not typed yet: a call through a variable, a parameter or an
//! attribute stays unresolved, and there is no same-name guessing.
//!
//! Lowered from statements, not from the syntax tree. The grammar has no
//! node for `FORM` or `MODULE`, and its error recovery loses whole `METHOD`
//! blocks (see `abap_fallback`), so the source is cut into statements by
//! the same cutter, with comments and literals blanked by
//! `strip_abap_content`. The blanking keeps every byte where it was, so an
//! offset into the stripped code is one into the source, and a `CALL
//! FUNCTION` literal is read back from the source at the same place.

use std::path::Path as FsPath;

use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};

use super::ir::*;
use super::lang::{BuiltinRet, ClosureArg, Lang, Layout};
use crate::parser::graph::strip_abap_content;
use crate::parser::plugins::code::abap_fallback::{statements, Statement};
use crate::parser::plugins::code::abap_name::parse_abapgit_name;

pub struct Abap;

pub static ABAP: Abap = Abap;

impl Lang for Abap {
    fn lower(&self, tree: &tree_sitter::Tree, src: &str) -> FileFacts {
        let _ = tree; // statements, not the tree: see the module doc
        lower(src)
    }

    fn layout(&self, root: &FsPath, files: &[(&str, &FileFacts)]) -> Layout {
        let _ = root;
        layout(files)
    }

    fn builtin_method(&self, _recv: &str, _method: &str) -> Option<BuiltinRet> {
        None
    }

    fn builtin_field(&self, _ty: &str, _field: &str) -> Option<BuiltinRet> {
        None
    }

    fn deref(&self, _ty: &str) -> Option<BuiltinRet> {
        None
    }

    fn variant_payload(&self, _ctor: &str, _field: &str) -> Option<BuiltinRet> {
        None
    }

    fn closure_param(&self, _method: &str, _pos: usize, _k: usize) -> Option<ClosureArg> {
        None
    }

    fn elem(&self, _container: &str) -> Option<BuiltinRet> {
        None
    }

    fn case_insensitive(&self) -> bool {
        true
    }

    fn implicit_self(&self) -> bool {
        true
    }

    fn replaces_bow(&self) -> bool {
        // not until receivers are typed: the bag-of-words resolver's unique
        // names still bind the calls through them (`abap_scoped_target`)
        false
    }

    fn fn_entity_types(&self) -> &'static [&'static str] {
        // a report's own statements are its body
        &["function", "method", "form", "module", "report"]
    }

    fn self_value(&self) -> &'static str {
        "me"
    }

    fn self_type(&self) -> &'static str {
        "" // no name for the enclosing class's type
    }
}

// ---------------------------------------------------------------------------
// Lowering
// ---------------------------------------------------------------------------

pub fn lower(src: &str) -> FileFacts {
    let code = strip_abap_content(src);
    let mut cx = Lower {
        src,
        code: &code,
        f: FileFacts::default(),
        interner: HashMap::default(),
        lines: std::iter::once(0)
            .chain(src.match_indices('\n').map(|(i, _)| i + 1))
            .collect(),
        classes: HashMap::default(),
        class: None,
        section: Section::None,
        body: None,
        report: None,
        macros: HashSet::default(),
        in_macro: false,
        in_interface: false,
    };
    cx.f.exprs.push(Expr::Unknown);
    // scope 0 holds the global names, scope 1 the object's local ones
    cx.f.scopes.push(ScopeDecl {
        name: None,
        parent: None,
        out_of_line: false,
    });
    cx.f.scopes.push(ScopeDecl {
        name: None,
        parent: Some(0),
        out_of_line: false,
    });
    for statement in statements(&code) {
        cx.statement(&statement);
    }
    cx.close_body(code.len());
    let mut f = cx.f;
    f.locals.sort_by_key(|l| (l.func, l.at));
    f
}

/// A class of this file, across its `DEFINITION` and `IMPLEMENTATION`.
struct ClassCx {
    /// Its type, when this file holds its definition.
    ty: Option<u32>,
    /// Its methods' impl, once this file implements it.
    imp: Option<u32>,
    /// The scope its own name lives in: 0 for a global class, 1 for a local one.
    outer: u32,
    /// Its own block: its attributes, and its methods' sites.
    scope: u32,
    /// Attributes (`DATA`, `CLASS-DATA`, `CONSTANTS`) its definition declares.
    attrs: HashSet<Name>,
    /// Static methods (`CLASS-METHODS`) its definition declares.
    statics: HashSet<Name>,
}

/// Which half of a `CLASS` the statements are in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Definition,
    Implementation,
}

/// The method, form, function module or module being lowered.
#[derive(Clone, Copy)]
struct Body {
    func: u32,
    scope: u32,
    method: bool,
    /// Its first local, in `f.locals`: the ones its end closes.
    locals: usize,
}

struct Lower<'a> {
    src: &'a str,
    /// `src` with comments and literals blanked, byte for byte.
    code: &'a str,
    f: FileFacts,
    interner: HashMap<Name, Sym>,
    /// Byte offset of each line's start.
    lines: Vec<usize>,
    classes: HashMap<Name, ClassCx>,
    class: Option<Name>,
    section: Section,
    body: Option<Body>,
    /// The `REPORT`, whose body is every statement outside a procedure.
    report: Option<u32>,
    /// `DEFINE` names: a statement starting with one is a macro call.
    macros: HashSet<Name>,
    in_macro: bool,
    in_interface: bool,
}

/// A chain of member accesses being read (`a=>b->c-d`), before it is known
/// whether its last link is called.
enum Link {
    /// `name` or `class=>member`.
    Path(Vec<Name>),
    /// `recv->name` or `recv-name`: `name`'s offset too.
    Member(ExprId, Name, usize),
    /// A value: a call's result, a parenthesized expression.
    Value(ExprId),
}

/// Constructor operators: the type after one is not called (`CONV i( x )`).
/// `NEW` is not one: it calls the class's constructor.
const OPERATORS: &[&str] = &[
    "value",
    "conv",
    "cast",
    "ref",
    "corresponding",
    "exact",
    "cond",
    "switch",
    "reduce",
    "filter",
];

fn fold(s: &str) -> Name {
    s.to_ascii_lowercase().into()
}

fn name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl<'a> Lower<'a> {
    fn row(&self, at: usize) -> u32 {
        (self.lines.partition_point(|&l| l <= at) - 1) as u32
    }

    fn sym(&mut self, s: &str) -> Sym {
        if let Some(&id) = self.interner.get(s) {
            return id;
        }
        let id = Sym(self.f.syms.len() as u32);
        self.f.syms.push(s.into());
        self.interner.insert(s.into(), id);
        id
    }

    fn node(&mut self, e: Expr) -> ExprId {
        if e == Expr::Unknown {
            return ExprId::UNKNOWN;
        }
        self.f.exprs.push(e);
        ExprId(self.f.exprs.len() as u32 - 1)
    }

    fn path_expr(&mut self, segs: &[Name]) -> ExprId {
        let start = self.f.path_pool.len() as u32;
        for s in segs {
            let id = self.sym(s);
            self.f.path_pool.push(id);
        }
        self.node(Expr::Path(Span {
            start,
            len: segs.len() as u32,
        }))
    }

    /// One statement: a declaration opens or closes a class, a procedure or
    /// a macro, and anything else in a procedure or a program is read for
    /// calls.
    fn statement(&mut self, st: &Statement) {
        let code = self.code;
        let words: Vec<&str> = st.tokens.iter().map(|t| t.text(code)).collect();
        let Some(head) = words.first().map(|w| w.to_ascii_uppercase()) else {
            return;
        };
        let start = st.tokens[0].start_byte;
        let end = before_period(st);
        if self.in_macro {
            self.in_macro = head != "END-OF-DEFINITION";
            return;
        }
        let word = |i: usize| words.get(i).copied().filter(|w| !matches!(*w, ":" | ","));
        match head.as_str() {
            "DEFINE" => {
                if let Some(name) = word(1) {
                    self.macros.insert(fold(name));
                }
                self.in_macro = true;
            }
            "REPORT" | "PROGRAM" => {
                if let Some(name) = word(1) {
                    self.report = Some(self.free_fn(name, start, 0));
                }
            }
            "CLASS" => self.class(st, &words),
            "ENDCLASS" => {
                self.class = None;
                self.section = Section::None;
            }
            "INTERFACE" => self.interface(&words, start),
            "ENDINTERFACE" => self.in_interface = false,
            "METHOD" if self.section == Section::Implementation => {
                if let Some(name) = word(1) {
                    self.method(name, start);
                }
            }
            "FORM" | "MODULE" | "FUNCTION" => {
                if let Some(name) = word(1) {
                    self.close_body(start);
                    // a function module is global; a form or module is its object's
                    let scope = u32::from(head != "FUNCTION");
                    let func = self.free_fn(name, start, scope);
                    self.body = Some(Body {
                        func,
                        scope,
                        method: false,
                        locals: self.f.locals.len(),
                    });
                }
            }
            "ENDMETHOD" | "ENDFORM" | "ENDMODULE" | "ENDFUNCTION" => self.close_body(start),
            _ if self.in_interface => {}
            _ if self.section == Section::Definition => self.declaration(&head, st),
            _ if self.section == Section::Implementation && self.body.is_none() => {}
            _ if self.macros.contains(&*fold(&words[0])) => {} // a macro call
            "PERFORM" => self.perform(st, &words),
            "CALL" if word(1).is_some_and(|w| w.eq_ignore_ascii_case("FUNCTION")) => {
                self.call_function(st);
            }
            "CALL" if word(1).is_some_and(|w| w.eq_ignore_ascii_case("METHOD")) => {
                if let Some(t) = st.tokens.get(2) {
                    self.expressions(t.start_byte, end, true);
                }
            }
            _ => {
                self.locals(&head, st);
                self.expressions(start, end, false);
            }
        }
    }

    /// `CLASS x DEFINITION ...` declares a type, `CLASS x IMPLEMENTATION`
    /// opens its methods' impl. `DEFERRED`, `LOAD` and `LOCAL FRIENDS`
    /// declare nothing.
    fn class(&mut self, st: &Statement, words: &[&str]) {
        let (Some(name), Some(kind)) = (words.get(1), words.get(2)) else {
            return;
        };
        let name = fold(name);
        let rest: Vec<String> = words[3..].iter().map(|w| w.to_ascii_uppercase()).collect();
        let row = self.row(st.tokens[0].start_byte);
        if kind.eq_ignore_ascii_case("DEFINITION") {
            if rest.iter().any(|w| w == "DEFERRED" || w == "LOAD")
                || rest.first().is_some_and(|w| w == "LOCAL")
            {
                return;
            }
            // `PUBLIC`, not `CREATE PUBLIC`: a global class
            let public =
                (0..rest.len()).any(|i| rest[i] == "PUBLIC" && (i == 0 || rest[i - 1] != "CREATE"));
            let base = rest
                .windows(2)
                .position(|w| w[0] == "INHERITING" && w[1] == "FROM")
                .and_then(|i| words.get(3 + i + 2));
            let outer = u32::from(!public);
            let embeds = base
                .map(|b| named(&fold(b)))
                .into_iter()
                .collect::<Vec<_>>();
            let ty = self.f.types.len() as u32;
            self.f.types.push(TypeDecl {
                name: name.clone(),
                row,
                scope: outer,
                generics: Vec::new(),
                kind: TypeKind::Struct,
                fields: Vec::new(),
                variants: Vec::new(),
                embeds,
                field_inits: Vec::new(),
            });
            self.class_cx(&name, outer).ty = Some(ty);
            self.section = Section::Definition;
        } else if kind.eq_ignore_ascii_case("IMPLEMENTATION") {
            // a class implemented with no definition in the file is local
            let cx = self.class_cx(&name, 1);
            if cx.imp.is_none() {
                let outer = cx.outer;
                let imp = self.f.impls.len() as u32;
                self.f.impls.push(ImplDecl {
                    scope: outer,
                    generics: Vec::new(),
                    self_ty: named(&name),
                    trait_: None,
                    assoc_types: Vec::new(),
                });
                self.class_cx(&name, outer).imp = Some(imp);
            }
            self.section = Section::Implementation;
        } else {
            return;
        }
        self.class = Some(name);
    }

    /// The class `name` of this file, made on first sight, in scope `outer`.
    fn class_cx(&mut self, name: &Name, outer: u32) -> &mut ClassCx {
        if !self.classes.contains_key(name) {
            let scope = self.f.scopes.len() as u32;
            self.f.scopes.push(ScopeDecl {
                name: None,
                parent: Some(outer),
                out_of_line: false,
            });
            self.classes.insert(
                name.clone(),
                ClassCx {
                    ty: None,
                    imp: None,
                    outer,
                    scope,
                    attrs: HashSet::default(),
                    statics: HashSet::default(),
                },
            );
        }
        self.classes.get_mut(name).unwrap()
    }

    /// `INTERFACE x [PUBLIC].` declares a trait; its members are left to
    /// dispatch, which does not model them yet.
    fn interface(&mut self, words: &[&str], start: usize) {
        let Some(name) = words.get(1) else { return };
        let rest: Vec<String> = words[2..].iter().map(|w| w.to_ascii_uppercase()).collect();
        if rest.iter().any(|w| w == "DEFERRED" || w == "LOAD") {
            return;
        }
        self.f.traits.push(TraitDecl {
            name: fold(name),
            row: self.row(start),
            scope: u32::from(!rest.iter().any(|w| w == "PUBLIC")),
            generics: Vec::new(),
            supertraits: Vec::new(),
            assoc: Vec::new(),
        });
        self.in_interface = true;
    }

    /// A declaration in a class's definition: its attributes, and which of
    /// its methods are static.
    fn declaration(&mut self, head: &str, st: &Statement) {
        let Some(class) = self.class.clone() else {
            return;
        };
        let attrs = matches!(head, "DATA" | "CLASS-DATA" | "CONSTANTS");
        if !attrs && head != "CLASS-METHODS" {
            return;
        }
        let scope = self.classes[&class].scope;
        for (name, at) in self.declared(st) {
            if attrs {
                let row = self.row(at);
                self.f.values.push(ValueDecl {
                    name: name.clone(),
                    row,
                    scope,
                    ty: None,
                    init: None,
                });
                self.classes.get_mut(&class).unwrap().attrs.insert(name);
            } else {
                self.classes.get_mut(&class).unwrap().statics.insert(name);
            }
        }
    }

    /// The names a declaration statement declares, with their offsets: the
    /// word after the keyword, and in a chain (`DATA: a ..., b ....`) the
    /// word after each comma. A structure (`BEGIN OF s ... END OF s`) is
    /// `s`, not its components.
    fn declared(&self, st: &Statement) -> Vec<(Name, usize)> {
        let code = self.code;
        let mut out = Vec::new();
        let mut depth = 0usize;
        let mut next = true; // the next word names something
        let mut tokens = st.tokens.iter().skip(1);
        while let Some(t) = tokens.next() {
            let w = t.text(code);
            if matches!(w, ":" | ",") {
                next = true;
                continue;
            }
            if !next {
                continue;
            }
            next = false;
            match w.to_ascii_uppercase().as_str() {
                "BEGIN" => {
                    tokens.next(); // OF
                    if let Some(s) = tokens.next() {
                        if depth == 0 {
                            out.push((fold(s.text(code)), s.start_byte));
                        }
                    }
                    depth += 1;
                }
                "END" => depth = depth.saturating_sub(1),
                _ if depth == 0 => out.push((fold(w), t.start_byte)),
                _ => {}
            }
        }
        out
    }

    fn free_fn(&mut self, name: &str, start: usize, scope: u32) -> u32 {
        let row = self.row(start);
        self.f.fns.push(FnDecl {
            name: fold(name),
            row,
            scope,
            owner: Owner::Free,
            generics: Vec::new(),
            params: Vec::new(),
            has_self: false,
            enclosing: None,
            ret: None,
        });
        self.f.fns.len() as u32 - 1
    }

    /// `METHOD m.` in a class's implementation.
    fn method(&mut self, name: &str, start: usize) {
        let Some(class) = self.class.clone() else {
            return;
        };
        self.close_body(start);
        let name = fold(name);
        let cx = &self.classes[&class];
        let (scope, imp, has_self) = (cx.scope, cx.imp, !cx.statics.contains(&name));
        let Some(imp) = imp else { return };
        let row = self.row(start);
        self.f.fns.push(FnDecl {
            name,
            row,
            scope,
            owner: Owner::Impl(imp),
            generics: Vec::new(),
            params: Vec::new(),
            has_self,
            enclosing: None,
            ret: None,
        });
        self.body = Some(Body {
            func: self.f.fns.len() as u32 - 1,
            scope,
            method: true,
            locals: self.f.locals.len(),
        });
    }

    /// End the procedure being lowered at byte `end`: its locals go out of
    /// scope there.
    fn close_body(&mut self, end: usize) {
        if let Some(body) = self.body.take() {
            for l in &mut self.f.locals[body.locals..] {
                l.until = end as u32;
            }
        }
    }

    /// Where a statement's sites are: `(function, scope)`. Outside every
    /// procedure, a program's statements are its report's body.
    fn site_cx(&self) -> (Option<u32>, u32) {
        match self.body {
            Some(b) => (Some(b.func), b.scope),
            None => (self.report, 1),
        }
    }

    /// Locals a statement in a procedure declares: `DATA x`, `FIELD-SYMBOLS
    /// <x>` and the inline `DATA(x)`, `FINAL(x)`, `FIELD-SYMBOL(<x>)`.
    /// Untyped: they only shadow the class's attributes.
    fn locals(&mut self, head: &str, st: &Statement) {
        let Some(body) = self.body else { return };
        let code = self.code;
        let mut names: Vec<Name> = Vec::new();
        if matches!(head, "DATA" | "STATICS" | "CONSTANTS" | "FIELD-SYMBOLS") {
            names.extend(self.declared(st).into_iter().map(|(n, _)| n));
        }
        for t in &st.tokens {
            let w = t.text(code).trim_start_matches('@');
            let lower = w.to_ascii_lowercase();
            for prefix in ["data(", "final(", "field-symbol("] {
                if let Some(rest) = lower.strip_prefix(prefix) {
                    if let Some((name, _)) = rest.split_once(')') {
                        names.push(name.into());
                    }
                }
            }
        }
        let at = st.tokens[0].start_byte as u32;
        for n in names {
            let name = self.sym(&n);
            self.f.locals.push(Local {
                func: body.func,
                name,
                at,
                until: u32::MAX,
                ty: None,
                init: None,
            });
        }
    }

    /// A keyword call's site. `PERFORM` and `CALL FUNCTION` never name a
    /// method, so inside one their site has no function, which keeps the
    /// implicit `me` from answering it: its edge comes from the method
    /// entity around its row instead.
    fn keyword_site(&mut self, name: &str, at: usize) {
        let (func, scope) = self.site_cx();
        let func = func.filter(|_| !self.body.is_some_and(|b| b.method));
        let callee = self.path_expr(&[fold(name)]);
        let expr = self.node(Expr::Call(callee));
        let row = self.row(at);
        self.f.sites.push(Site {
            func,
            scope,
            row,
            at: at as u32,
            kind: SiteKind::Call,
            expr,
        });
    }

    /// `PERFORM f ...`, `PERFORM: f, g.`: a form of this object. A form of
    /// another program (`IN PROGRAM p`, `f(p)`) or a dynamic name (`(f)`)
    /// is left to includes and dynamic calls.
    fn perform(&mut self, st: &Statement, words: &[&str]) {
        let upper: Vec<String> = words.iter().map(|w| w.to_ascii_uppercase()).collect();
        if upper.windows(2).any(|w| w[0] == "IN" && w[1] == "PROGRAM") {
            return;
        }
        let code = self.code;
        let chained = words.get(1) == Some(&":");
        let mut next = true;
        for t in &st.tokens[1..] {
            let w = t.text(code);
            if matches!(w, ":" | ",") {
                next = chained;
                continue;
            }
            if next && !w.contains('(') {
                self.keyword_site(w, t.start_byte);
            }
            next = false;
        }
        if let Some(t) = st.tokens.get(2) {
            self.expressions(t.start_byte, before_period(st), false);
        }
    }

    /// `CALL FUNCTION 'X' ...`: the function module named by the literal,
    /// read from the source since the stripped code has blanked it. A name
    /// in a variable is dynamic and left alone.
    fn call_function(&mut self, st: &Statement) {
        let end = before_period(st);
        let after = st.tokens[1].end_byte;
        let until = st.tokens.get(2).map_or(end, |t| t.start_byte);
        let gap = &self.src[after..until];
        let lit = gap.trim_start();
        let at = after + (gap.len() - lit.len());
        let lit = lit.trim_end();
        if lit.len() > 2
            && (lit.starts_with('\'') && lit.ends_with('\'')
                || lit.starts_with('`') && lit.ends_with('`'))
        {
            self.keyword_site(&lit[1..lit.len() - 1], at + 1);
        }
        if let Some(t) = st.tokens.get(2) {
            self.expressions(t.start_byte, end, false);
        }
    }

    /// The calls in `code[from..to]`: every name directly followed by `(`
    /// and a blank or `)` is called (`m( )`, `lo->m( x )`, `zcl_x=>m( )`),
    /// unlike an offset (`s+1(2)`) or an inline declaration (`DATA(x)`).
    /// With `call_method`, the first chain is called though it has no
    /// parentheses (`CALL METHOD lo->m EXPORTING ...`).
    fn expressions(&mut self, from: usize, to: usize, call_method: bool) {
        let b = self.code.as_bytes();
        let mut parens: Vec<Option<ExprId>> = Vec::new();
        let mut operator = false;
        let mut forced = call_method;
        let mut i = from;
        while i < to {
            let c = b[i];
            if c.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            // only a name right at the start is `CALL METHOD`'s
            let first = std::mem::take(&mut forced);
            if c == b'(' {
                parens.push(None);
                operator = false;
                i += 1;
                continue;
            }
            if c == b')' {
                // `)->m( )`: a call on the parenthesized value
                let value = parens.pop().flatten().unwrap_or(ExprId::UNKNOWN);
                i = self.chain(Link::Value(value), i + 1, to, &mut parens, false, false);
                operator = false;
                continue;
            }
            let fresh = i == from || !name_byte(b[i - 1]);
            let Some((name, end)) = fresh.then(|| self.name(i, to)).flatten() else {
                operator = false;
                i += 1;
                continue;
            };
            let suppress = std::mem::take(&mut operator);
            let (link, next) = self.head(name.clone(), i, end, to);
            i = self.chain(link, next, to, &mut parens, suppress, first);
            operator = i == end
                && OPERATORS.contains(&&*name)
                && b.get(i).is_some_and(|c| c.is_ascii_whitespace());
        }
    }

    /// The first link of a chain, `class=>member` or a name at byte `at`,
    /// and where it ends. A bare use of one of the enclosing class's
    /// attributes is a reference site.
    fn head(&mut self, name: Name, at: usize, end: usize, to: usize) -> (Link, usize) {
        if let Some((member, end)) = self.arrow(end, to, b"=>") {
            return (Link::Path(vec![name, member]), end);
        }
        let attr = self.body.is_some_and(|b| b.method)
            && self
                .class
                .as_ref()
                .is_some_and(|c| self.classes[c].attrs.contains(&name));
        let called = self.code.as_bytes().get(end) == Some(&b'(');
        if !attr || called {
            return (Link::Path(vec![name]), end);
        }
        let (func, scope) = self.site_cx();
        let expr = self.path_expr(&[name]);
        let row = self.row(at);
        self.f.sites.push(Site {
            func,
            scope,
            row,
            at: at as u32,
            kind: SiteKind::Ref,
            expr,
        });
        (Link::Value(expr), end)
    }

    /// Read the rest of a chain from byte `i`: `->m`, `=>m` and `-c` links,
    /// up to a call's `(` (pushed on `parens`, so the chain goes on after
    /// its `)`) or the chain's end. Returns where it stopped.
    fn chain(
        &mut self,
        mut link: Link,
        mut i: usize,
        to: usize,
        parens: &mut Vec<Option<ExprId>>,
        suppress: bool,
        call_method: bool,
    ) -> usize {
        let b = self.code.as_bytes();
        loop {
            if i < to && b[i] == b'(' {
                let called = b
                    .get(i + 1)
                    .is_some_and(|c| c.is_ascii_whitespace() || *c == b')');
                let call = if called && !suppress {
                    self.call(link, i)
                } else {
                    None
                };
                parens.push(call);
                return i + 1;
            }
            if let Some((member, end)) = self
                .arrow(i, to, b"->")
                .or_else(|| self.arrow(i, to, b"=>"))
            {
                let recv = self.value(link);
                link = Link::Member(recv, member, i + 2);
                i = end;
                continue;
            }
            // a structure component, `s-c`; not `a - b` nor `->`
            if i + 1 < to && b[i] == b'-' && b[i + 1] != b'>' {
                if let Some((member, end)) = self.name(i + 1, to) {
                    let recv = self.value(link);
                    link = Link::Member(recv, member, i + 1);
                    i = end;
                    continue;
                }
            }
            break;
        }
        // `CALL METHOD lo->m EXPORTING ...`, not `CALL METHOD lo->(name)`
        if call_method && (i >= to || b[i].is_ascii_whitespace()) {
            self.call(link, i);
        }
        i
    }

    /// `->name` or `=>name` at byte `i`: the name and where it ends.
    fn arrow(&self, i: usize, to: usize, arrow: &[u8]) -> Option<(Name, usize)> {
        let b = self.code.as_bytes();
        if i + 2 >= to || &b[i..i + 2] != arrow {
            return None;
        }
        self.name(i + 2, to)
    }

    /// A name starting at byte `i`, folded, and where it ends: an
    /// identifier, a namespaced one (`/ns/x`), a field symbol (`<fs>`), an
    /// interface component (`zif_x~m`), or a name escaped with `!`.
    fn name(&self, mut i: usize, to: usize) -> Option<(Name, usize)> {
        let b = self.code.as_bytes();
        if b.get(i) == Some(&b'!') {
            i += 1;
        }
        let ident = |from: usize| {
            let mut j = from;
            while j < to && name_byte(b[j]) {
                j += 1;
            }
            j
        };
        let start = i;
        let mut end = match b.get(i)? {
            // `<fs>`
            b'<' => {
                let j = ident(i + 1);
                (j > i + 1 && b.get(j) == Some(&b'>')).then_some(j + 1)?
            }
            // `/ns/name`
            b'/' => {
                let ns = ident(i + 1);
                (ns > i + 1 && b.get(ns) == Some(&b'/')).then_some(())?;
                let j = ident(ns + 1);
                (j > ns + 1).then_some(j)?
            }
            c if c.is_ascii_alphabetic() || *c == b'_' => ident(i),
            _ => return None,
        };
        // `zif_x~m`: one name
        if b.get(end) == Some(&b'~') {
            let j = ident(end + 1);
            if j > end + 1 {
                end = j;
            }
        }
        Some((fold(&self.code[start..end]), end))
    }

    /// A link as a value: `super` is the enclosing class's bases.
    fn value(&mut self, link: Link) -> ExprId {
        match link {
            Link::Path(segs) if segs.len() == 1 && &*segs[0] == "super" => {
                match self.class.as_ref().and_then(|c| self.classes[c].ty) {
                    Some(ty) => self.node(Expr::Super(ty)),
                    None => ExprId::UNKNOWN,
                }
            }
            Link::Path(segs) => self.path_expr(&segs),
            Link::Member(recv, name, _) => {
                let name = self.sym(&name);
                self.node(Expr::Field(recv, name))
            }
            Link::Value(v) => v,
        }
    }

    /// Call a link at the `(` (or chain end) at byte `paren`: a site.
    fn call(&mut self, link: Link, paren: usize) -> Option<ExprId> {
        let (expr, at) = match link {
            Link::Path(segs) => {
                let at = paren - segs.last().map_or(0, |s| s.len());
                let callee = self.path_expr(&segs);
                (self.node(Expr::Call(callee)), at)
            }
            Link::Member(recv, name, at) => {
                let name = self.sym(&name);
                (self.node(Expr::Method(recv, name)), at)
            }
            Link::Value(_) => return None,
        };
        let (func, scope) = self.site_cx();
        let row = self.row(at);
        self.f.sites.push(Site {
            func,
            scope,
            row,
            at: at as u32,
            kind: SiteKind::Call,
            expr,
        });
        Some(expr)
    }
}

/// Where a statement's text ends: at its period, or the end of the file.
fn before_period(st: &Statement) -> usize {
    st.period.map_or(st.end_byte(), |p| p.start_byte)
}

fn named(name: &str) -> TypeExpr {
    TypeExpr::Named {
        path: Path::single(name),
        args: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Layout: one root for the global names, one directory per object
// ---------------------------------------------------------------------------

/// Every file pools its scope 0 into the root, where the global classes,
/// interfaces, function modules and reports are seen repo-wide. Its scope 1
/// lives in its object's directory (`local_home`), which the parts of one
/// class or function group share, and which falls back to the root
/// (`dirs_fall_back`): an object sees its own local names first, and never
/// another object's. A file with no abapGit name is an object of its own.
fn layout(files: &[(&str, &FileFacts)]) -> Layout {
    let mut dirs: Vec<(String, Option<usize>)> = vec![(String::new(), None)];
    let mut objects: HashMap<String, usize> = HashMap::default();
    let mut orphan_dir = Vec::with_capacity(files.len());
    let mut local_home = Vec::with_capacity(files.len());
    for (path, _) in files {
        let file = path.rsplit('/').next().unwrap_or(path);
        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
        orphan_dir.push(Some((0, stem.to_string())));
        local_home.push(parse_abapgit_name(path).map(|object| {
            let name = object.name.to_ascii_lowercase();
            *objects.entry(name.clone()).or_insert_with(|| {
                dirs.push((name, Some(0)));
                dirs.len() - 1
            })
        }));
    }
    Layout {
        parent_of: vec![None; files.len()],
        dirs,
        orphan_dir,
        pooled: vec![true; files.len()],
        local_home,
        dirs_fall_back: true,
        ..Layout::default()
    }
}
