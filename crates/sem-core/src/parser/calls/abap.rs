//! ABAP front end: lowers an ABAP source into [`FileFacts`], plus ABAP's
//! data for the shared stages: names are case-insensitive (folded to lower
//! case), a global class, interface, function module or report is visible
//! repo-wide while a local class, form or module stays in its object (the
//! files of one class or function group, joined with the programs an
//! `INCLUDE` pastes them into), classes are types whose methods live in an
//! impl and whose `INHERITING FROM` base is searched for what they lack, an
//! interface is a trait whose `METHODS` it declares, a method calls its own
//! class's methods with no receiver written, and `CALL FUNCTION 'X'` and
//! `PERFORM f` call by name.
//!
//! Receivers are typed by what is declared, never guessed from a name: a
//! `TYPE REF TO` on a local, a parameter or an attribute, the constructor
//! forms `NEW zcl_x( )`, `CAST zcl_x( )` and `NEW #( )` (the target's
//! declared type), and the declared `RETURNING` type of a called method.
//! A method implementing an interface's method, or redefining its base
//! class's, has the parameters declared there (`Expr::Signature`). A
//! receiver typed `REF TO object`, `data` or `any` is unknown with its
//! reason, and so is a parameter whose declaration is in no file the
//! lowering or that join reaches (a local class's `METHODS` in
//! `locals_def` with its `METHOD` in `locals_imp`, an interface from
//! outside the repo). Every other name a statement reads is a reference site,
//! which binds when the name is a class, an interface, a type, an attribute
//! or a global, so a `TYPE REF TO zcl_x` is an edge to `zcl_x`.
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
use super::abap_dynamic::{dynamic_sites, DynSite};
use crate::parser::plugins::code::abap_fallback::{include_names, statements, Statement};
use crate::parser::plugins::code::abap_include::{
    reads_includes, IncludeGraph, INCLUDE_NOT_IN_REPO,
};
use crate::parser::plugins::code::abap_name::{is_abap_keyword, parse_abapgit_name};

pub struct Abap;

pub static ABAP: Abap = Abap;

/// Why a receiver typed `REF TO object`, `data` or `any` binds no call.
const GENERIC: &str = "generic reference type";
/// Why a parameter of a method whose `METHODS` is in another file is
/// unknown: a local class's in `locals_def`, or an interface's or a base
/// class's from outside the repo.
const SIGNATURE_ELSEWHERE: &str = "signature in another file";
/// Why `NEW #( )` or `CREATE OBJECT lo` binds no constructor: the target's
/// type is not declared in this file.
const NO_TARGET: &str = "NEW # or CREATE OBJECT without a declared type";

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

    fn function_scoped_names(&self) -> bool {
        // `DATA` is declared for the whole procedure: there are no blocks
        true
    }

    fn case_insensitive(&self) -> bool {
        true
    }

    fn implicit_self(&self) -> bool {
        true
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
        interfaces: HashMap::default(),
        class: None,
        section: Section::None,
        body: None,
        in_scope: HashSet::default(),
        declared_by: None,
        report: None,
        macros: HashSet::default(),
        in_macro: false,
        interface: None,
        decl_scope: None,
        declaring: Vec::new(),
        nesting: 0,
        dynamic: dynamic_sites(src),
        next_dynamic: 0,
    };
    cx.f.includes = include_names(src).iter().map(|n| fold(n)).collect();
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
    cx.dynamic_sites_before(usize::MAX);
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
    /// Attributes (`DATA`, `CLASS-DATA`, `CONSTANTS`) its definition
    /// declares, with their declared types.
    attrs: HashMap<Name, TypeExpr>,
    /// Static methods (`CLASS-METHODS`) its definition declares.
    statics: HashSet<Name>,
    /// Its methods' signatures, as its definition declares them.
    sigs: HashMap<Name, Sig>,
    /// Its `INHERITING FROM` base.
    base: Option<Name>,
}

/// A method's parameters as a `METHODS` declares them, in order, and its
/// `RETURNING` one.
#[derive(Clone, Default)]
struct Sig {
    params: Vec<Param>,
    ret: Option<Param>,
    /// `REDEFINITION`: the parameters are the base class's.
    redefinition: bool,
}

impl Sig {
    fn param_types(&self) -> Vec<TypeExpr> {
        self.params.iter().map(|p| p.decl.type_expr()).collect()
    }

    fn ret_type(&self) -> Option<TypeExpr> {
        self.ret.as_ref().map(|p| p.decl.type_expr())
    }
}

/// One parameter of a signature, with its offset.
#[derive(Clone)]
struct Param {
    name: Name,
    at: usize,
    decl: Decl,
}

/// What a declaration says of a name's type.
#[derive(Clone)]
enum Decl {
    /// `TYPE x`, `TYPE REF TO x`.
    Type(TypeExpr),
    /// `LIKE x`: whatever `x` is.
    Like(Name),
    /// No type written (`DATA x.`, a form's `USING p`), or one that holds no
    /// reference (a table, a structure).
    Untyped,
}

impl Decl {
    fn type_expr(&self) -> TypeExpr {
        match self {
            Decl::Type(t) => t.clone(),
            Decl::Like(_) | Decl::Untyped => TypeExpr::Unknown,
        }
    }
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
    /// Its parameters are declared in another file, so a name it reads that
    /// is none of its locals may be one of them.
    elsewhere: bool,
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
    /// The method signatures of each interface of this file.
    interfaces: HashMap<Name, HashMap<Name, Sig>>,
    class: Option<Name>,
    section: Section,
    body: Option<Body>,
    /// The names of the body's locals and parameters.
    in_scope: HashSet<Name>,
    /// Where the body's parameters are declared when it is not in this file:
    /// the interface or base class, and the method's name there.
    declared_by: Option<(Name, Name)>,
    /// The `REPORT`, whose body is every statement outside a procedure.
    report: Option<u32>,
    /// `DEFINE` names: a statement starting with one is a macro call.
    macros: HashSet<Name>,
    in_macro: bool,
    /// The `INTERFACE` being lowered, and its trait.
    interface: Option<(Name, u32)>,
    /// The scope of a declaration part's sites: a class's definition, an
    /// interface, a `CLASS` statement. They belong to no procedure.
    decl_scope: Option<u32>,
    /// Offsets of the names the statement being read declares, which are
    /// not references.
    declaring: Vec<usize>,
    /// `BEGIN OF` blocks open across statements: their components are not
    /// declarations of their own.
    nesting: usize,
    /// The file's computed calls (`abap_dynamic`), in source order, and how
    /// many the statement walk has passed.
    dynamic: Vec<DynSite>,
    next_dynamic: usize,
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

/// The keywords a signature lists its parameters after.
const SECTIONS: &[&str] = &[
    "IMPORTING",
    "EXPORTING",
    "CHANGING",
    "RETURNING",
    "USING",
    "TABLES",
];
/// The keywords after which it lists no parameters: its exceptions, the
/// event a handler is for, and how the method is declared.
const NOT_PARAMETERS: &[&str] = &["RAISING", "EXCEPTIONS", "FOR", "ABSTRACT", "FINAL"];

fn fold(s: &str) -> Name {
    s.to_ascii_lowercase().into()
}

/// The first segment of an `IN PROGRAM x` path. Not `x` itself: that names
/// the report's function in the root, which would answer first.
fn program_key(program: &str) -> Name {
    format!("program:{program}").into()
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

    fn type_idx(&mut self, t: TypeExpr) -> u32 {
        self.f.type_pool.push(t);
        self.f.type_pool.len() as u32 - 1
    }

    fn typed(&mut self, t: TypeExpr) -> ExprId {
        let t = self.type_idx(t);
        self.node(Expr::Typed(t))
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

    fn list(&mut self, items: &[ExprId]) -> Span {
        let start = self.f.expr_pool.len() as u32;
        self.f.expr_pool.extend_from_slice(items);
        Span {
            start,
            len: items.len() as u32,
        }
    }

    /// One statement: a declaration opens or closes a class, an interface,
    /// a procedure or a macro, and anything else is read for calls and
    /// references.
    fn statement(&mut self, st: &Statement) {
        let code = self.code;
        let words: Vec<&str> = st.tokens.iter().map(|t| t.text(code)).collect();
        let Some(head) = words.first().map(|w| w.to_ascii_uppercase()) else {
            return;
        };
        let start = st.tokens[0].start_byte;
        let end = before_period(st);
        self.dynamic_sites_before(st.end_byte());
        if self.in_macro {
            self.in_macro = head != "END-OF-DEFINITION";
            return;
        }
        let word = |i: usize| words.get(i).copied().filter(|w| !matches!(*w, ":" | ","));
        let is = |i: usize, w: &str| words.get(i).is_some_and(|x| x.eq_ignore_ascii_case(w));
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
            "ENDINTERFACE" => self.interface = None,
            "METHOD" if self.section == Section::Implementation => {
                if let Some(name) = word(1) {
                    self.method(name, start);
                }
            }
            "FORM" | "MODULE" | "FUNCTION" => {
                if let Some(name) = word(1) {
                    self.procedure(&head, name, st);
                }
            }
            "ENDMETHOD" | "ENDFORM" | "ENDMODULE" | "ENDFUNCTION" => self.close_body(start),
            // a program include names a file, not a reference: `lower` has
            // read it into `FileFacts.includes` (`include_names`)
            "INCLUDE" if !is(1, "STRUCTURE") && !is(1, "TYPE") => {}
            _ if self.interface.is_some() => self.interface_member(&head, st),
            _ if self.section == Section::Definition => self.declaration(&head, st),
            _ if self.section == Section::Implementation && self.body.is_none() => {}
            _ if self.macros.contains(&*fold(&words[0])) => {} // a macro call
            "PERFORM" => self.perform(st, &words),
            "CALL" if is(1, "FUNCTION") => self.call_function(st),
            "CALL" if is(1, "METHOD") => {
                if let Some(t) = st.tokens.get(2) {
                    self.expressions(t.start_byte, end, true, None);
                }
            }
            "CREATE" if is(1, "OBJECT") => self.create_object(st, &words),
            _ if matches!(words.get(1), Some(&"=") | Some(&"?=")) => self.assignment(st, &words),
            _ => {
                self.locals(&head, st);
                self.expressions(start, end, false, None);
                self.declaring.clear();
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
        self.nesting = 0;
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
                .and_then(|i| words.get(3 + i + 2))
                .map(|b| fold(b));
            let outer = u32::from(!public);
            let embeds = base.iter().map(|b| named(b)).collect::<Vec<_>>();
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
            let cx = self.class_cx(&name, outer);
            cx.ty = Some(ty);
            cx.base = base;
            // its base and its friends are read where the class is named
            if let Some(t) = st.tokens.get(3) {
                self.declaration_part(t.start_byte, before_period(st), outer);
            }
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
                    attrs: HashMap::default(),
                    statics: HashSet::default(),
                    sigs: HashMap::default(),
                    base: None,
                },
            );
        }
        self.classes.get_mut(name).unwrap()
    }

    /// `INTERFACE x [PUBLIC].` declares a trait, whose `METHODS` follow.
    fn interface(&mut self, words: &[&str], start: usize) {
        let Some(name) = words.get(1) else { return };
        let rest: Vec<String> = words[2..].iter().map(|w| w.to_ascii_uppercase()).collect();
        if rest.iter().any(|w| w == "DEFERRED" || w == "LOAD") {
            return;
        }
        let name = fold(name);
        self.f.traits.push(TraitDecl {
            name: name.clone(),
            row: self.row(start),
            scope: u32::from(!rest.iter().any(|w| w == "PUBLIC")),
            generics: Vec::new(),
            supertraits: Vec::new(),
            assoc: Vec::new(),
        });
        self.interface = Some((name, self.f.traits.len() as u32 - 1));
    }

    /// A statement in an `INTERFACE`: its `METHODS` are the trait's methods,
    /// with their signatures, and whatever else it reads is the interface's
    /// reference.
    fn interface_member(&mut self, head: &str, st: &Statement) {
        let Some((iface, trait_)) = self.interface.clone() else {
            return;
        };
        let scope = self.f.traits[trait_ as usize].scope;
        match head {
            "METHODS" | "CLASS-METHODS" => {
                for (name, at, sig) in self.signatures(st) {
                    let row = self.row(at);
                    self.f.fns.push(FnDecl {
                        name: name.clone(),
                        row,
                        scope,
                        owner: Owner::Trait(trait_),
                        generics: Vec::new(),
                        params: sig.param_types(),
                        has_self: head == "METHODS",
                        enclosing: None,
                        ret: sig.ret_type(),
                    });
                    // its parameters, for the methods implementing it
                    // (`Expr::Signature`)
                    let func = self.f.fns.len() as u32 - 1;
                    let params: Vec<Param> =
                        sig.params.iter().cloned().chain(sig.ret.clone()).collect();
                    self.bind(func, at, &params);
                    self.declaring.push(at);
                    self.interfaces
                        .entry(iface.clone())
                        .or_default()
                        .insert(name, sig);
                }
            }
            "DATA" | "CLASS-DATA" | "CONSTANTS" | "TYPES" => {
                let declared = self.declared(st);
                self.declaring
                    .extend(declared.into_iter().map(|(_, at, _)| at));
            }
            _ => {}
        }
        self.declaration_part(st.tokens[0].start_byte, before_period(st), scope);
    }

    /// A declaration in a class's definition: its attributes and their
    /// types, its methods' signatures and which are static, and its type
    /// aliases. What it reads (a parameter's type, an interface) is the
    /// class's reference.
    fn declaration(&mut self, head: &str, st: &Statement) {
        let Some(class) = self.class.clone() else {
            return;
        };
        let scope = self.classes[&class].scope;
        match head {
            "DATA" | "CLASS-DATA" | "CONSTANTS" => {
                let ty = self.classes[&class].ty;
                for (name, at, decl) in self.declared(st) {
                    let row = self.row(at);
                    let te = decl.type_expr();
                    self.f.values.push(ValueDecl {
                        name: name.clone(),
                        row,
                        scope,
                        ty: (te != TypeExpr::Unknown).then(|| te.clone()),
                        init: None,
                    });
                    if let (Some(t), Decl::Type(te)) = (ty, &decl) {
                        self.f.types[t as usize]
                            .fields
                            .push((name.clone(), te.clone()));
                    }
                    self.classes.get_mut(&class).unwrap().attrs.insert(name, te);
                    self.declaring.push(at);
                }
            }
            "METHODS" | "CLASS-METHODS" => {
                for (name, at, sig) in self.signatures(st) {
                    let cx = self.classes.get_mut(&class).unwrap();
                    if head == "CLASS-METHODS" {
                        cx.statics.insert(name.clone());
                    }
                    cx.sigs.insert(name, sig);
                    self.declaring.push(at);
                }
            }
            "TYPES" => self.types(st, scope),
            _ => {}
        }
        self.declaration_part(st.tokens[0].start_byte, before_period(st), scope);
    }

    /// Read `code[from..to]` for references only, as sites of no procedure
    /// in `scope`.
    fn declaration_part(&mut self, from: usize, to: usize, scope: u32) {
        self.decl_scope = Some(scope);
        self.expressions(from, to, false, None);
        self.decl_scope = None;
        self.declaring.clear();
    }

    /// `TYPES x TYPE ...`: aliases in `scope`, so a `TYPE REF TO` reaches
    /// the class through them.
    fn types(&mut self, st: &Statement, scope: u32) {
        for (name, at, decl) in self.declared(st) {
            let row = self.row(at);
            self.f.types.push(TypeDecl {
                name,
                row,
                scope,
                generics: Vec::new(),
                kind: TypeKind::Alias(decl.type_expr()),
                fields: Vec::new(),
                variants: Vec::new(),
                embeds: Vec::new(),
                field_inits: Vec::new(),
            });
            self.declaring.push(at);
        }
    }

    /// The names a declaration statement declares, with their offsets and
    /// types: the word after the keyword, and in a chain (`DATA: a ..., b
    /// ....`) the word after each comma. A structure (`BEGIN OF s ... END
    /// OF s`, in one statement or several) is `s`, not its components.
    fn declared(&mut self, st: &Statement) -> Vec<(Name, usize, Decl)> {
        let code = self.code;
        let words: Vec<&str> = st.tokens.iter().map(|t| t.text(code)).collect();
        let mut out = Vec::new();
        let mut next = true; // the next word names something
        let mut i = 1;
        while i < words.len() {
            let w = words[i];
            i += 1;
            if matches!(w, ":" | ",") {
                next = true;
                continue;
            }
            if !std::mem::take(&mut next) {
                continue;
            }
            match w.to_ascii_uppercase().as_str() {
                "BEGIN" => {
                    // `BEGIN OF s`, `BEGIN OF ENUM s`
                    let mut k = i + 1;
                    if words
                        .get(k)
                        .is_some_and(|w| matches!(&*w.to_ascii_uppercase(), "ENUM" | "MESH"))
                    {
                        k += 1;
                    }
                    if let (Some(s), 0) = (words.get(k), self.nesting) {
                        out.push((fold(s), st.tokens[k].start_byte, Decl::Untyped));
                    }
                    self.nesting += 1;
                    i = k + 1;
                }
                "END" => self.nesting = self.nesting.saturating_sub(1),
                _ if self.nesting == 0 => {
                    let (decl, _) = type_clause(&words, i);
                    out.push((fold(w), st.tokens[i - 1].start_byte, decl));
                }
                _ => {}
            }
        }
        out
    }

    /// The methods a `METHODS` or `CLASS-METHODS` statement declares (one
    /// per name of a chain), with their offsets and signatures.
    fn signatures(&self, st: &Statement) -> Vec<(Name, usize, Sig)> {
        let code = self.code;
        let mut out = Vec::new();
        let words: Vec<(&str, usize)> = st.tokens[1..]
            .iter()
            .map(|t| (t.text(code), t.start_byte))
            .collect();
        for part in words.split(|(w, _)| matches!(*w, ":" | ",")) {
            let Some(&(name, at)) = part.first() else {
                continue;
            };
            let upper: Vec<String> = part.iter().map(|(w, _)| w.to_ascii_uppercase()).collect();
            // an event handler's parameters take the event's types
            let handler = upper.windows(2).any(|w| w[0] == "FOR" && w[1] == "EVENT");
            let mut sig = Sig {
                redefinition: upper.iter().any(|w| w == "REDEFINITION"),
                ..Sig::default()
            };
            for (p, returning) in parameters(&part[1..], handler) {
                match returning {
                    true => sig.ret = Some(p),
                    false => sig.params.push(p),
                }
            }
            out.push((fold(name), at, sig));
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

    /// `FORM f USING ...`, `MODULE m ...`, `FUNCTION f.`: a procedure with its
    /// parameters. A form's are on its statement, a function module's in the
    /// comment block abapGit writes after it.
    fn procedure(&mut self, head: &str, name: &str, st: &Statement) {
        let start = st.tokens[0].start_byte;
        self.close_body(start);
        // a function module is global; a form or module is its object's
        let scope = u32::from(head != "FUNCTION");
        let func = self.free_fn(name, start, scope);
        self.body = Some(Body {
            func,
            scope,
            method: false,
            locals: self.f.locals.len(),
            elsewhere: false,
        });
        let code = self.code;
        let params = match head {
            "FORM" => {
                let words: Vec<(&str, usize)> = st.tokens[2..]
                    .iter()
                    .map(|t| (t.text(code), t.start_byte))
                    .collect();
                parameters(&words, true)
            }
            "FUNCTION" => self.function_parameters(st.end_byte()),
            _ => Vec::new(),
        };
        let params: Vec<Param> = params.into_iter().map(|(p, _)| p).collect();
        self.bind(func, start, &params);
        self.f.fns[func as usize].params = params.iter().map(|p| p.decl.type_expr()).collect();
        if let ("FORM", Some(t)) = (head, st.tokens.get(2)) {
            self.declaring.extend(params.iter().map(|p| p.at));
            self.expressions(t.start_byte, before_period(st), false, None);
            self.declaring.clear();
        }
    }

    /// A function module's parameters, from the `*"` comment block after
    /// its `FUNCTION` statement (`*"  IMPORTING  VALUE(IV_ID) TYPE  I`).
    fn function_parameters(&self, from: usize) -> Vec<(Param, bool)> {
        let mut words: Vec<(&str, usize)> = Vec::new();
        let mut at = from;
        for line in self.src[from..].split_inclusive('\n') {
            let text = line.trim_start();
            let offset = at + (line.len() - text.len());
            at += line.len();
            if text.trim().is_empty() {
                continue;
            }
            let Some(body) = text.strip_prefix("*\"") else {
                break;
            };
            let base = offset + 2;
            words.extend(
                body.split_whitespace()
                    .map(|w| (w, base + (w.as_ptr() as usize - body.as_ptr() as usize))),
            );
        }
        parameters(&words, false)
    }

    /// Parameters `params` of function `func`, from byte `at` to the end of
    /// its body: locals of their declared types.
    fn bind(&mut self, func: u32, at: usize, params: &[Param]) {
        for p in params {
            self.local(func, &p.name, at, &p.decl, None);
        }
    }

    /// A local `name` of `func` from byte `at`: of its declared type, or of
    /// what `init` evaluates to. An interface method's parameters are
    /// locals of its declaration, outside every body.
    fn local(&mut self, func: u32, name: &Name, at: usize, decl: &Decl, init: Option<ExprId>) {
        let (ty, init) = match decl {
            Decl::Type(t) => (Some(self.type_idx(t.clone())), init),
            Decl::Like(of) => (None, Some(self.path_expr(std::slice::from_ref(of)))),
            Decl::Untyped => (None, init),
        };
        let sym = self.sym(name);
        self.f.locals.push(Local {
            func,
            name: sym,
            at: at as u32,
            until: u32::MAX,
            ty,
            init,
        });
        if self.body.is_some() {
            self.in_scope.insert(name.clone());
        }
    }

    /// `METHOD m.` in a class's implementation: its parameters from the
    /// class's `METHODS m`, an interface's for `zif_x~m`, or the base
    /// class's for a redefinition. Where none of those is in this file, a
    /// receiver is read from that declaration as the resolver finds it
    /// (`receiver`), or is unknown and said to be.
    fn method(&mut self, name: &str, start: usize) {
        let Some(class) = self.class.clone() else {
            return;
        };
        self.close_body(start);
        let name = fold(name);
        let cx = &self.classes[&class];
        let (scope, imp, has_self) = (cx.scope, cx.imp, !cx.statics.contains(&name));
        let Some(imp) = imp else { return };
        let sig = self.signature_of(&class, &name);
        self.declared_by = match name.split_once('~') {
            _ if sig.is_some() => None,
            Some((iface, method)) => Some((iface.into(), method.into())),
            None => cx.base.clone().map(|base| (base, name.clone())),
        };
        let row = self.row(start);
        let func = self.f.fns.len() as u32;
        self.f.fns.push(FnDecl {
            name: name.clone(),
            row,
            scope,
            owner: Owner::Impl(imp),
            generics: Vec::new(),
            params: sig.as_ref().map(Sig::param_types).unwrap_or_default(),
            has_self,
            enclosing: None,
            ret: sig.as_ref().and_then(Sig::ret_type),
        });
        self.body = Some(Body {
            func,
            scope,
            method: true,
            locals: self.f.locals.len(),
            elsewhere: sig.is_none(),
        });
        if let Some(sig) = sig {
            let params: Vec<Param> = sig.params.into_iter().chain(sig.ret).collect();
            self.bind(func, start, &params);
        }
        // `METHOD zif_x~m.` reads the interface
        if let Some((iface, _)) = name.split_once('~') {
            let at = start + self.code[start..].find('~').unwrap_or(0) - iface.len();
            self.reference(&[iface.into()], at);
        }
    }

    /// The signature `METHODS name` declares for `class` in this file, where
    /// one does: the class's own, its interface's (`zif_x~m`), or for a
    /// `REDEFINITION` its nearest base's.
    fn signature_of(&self, class: &Name, name: &Name) -> Option<Sig> {
        if let Some((iface, method)) = name.split_once('~') {
            return self.interfaces.get(iface)?.get(method).cloned();
        }
        let own = &self.classes[class];
        own.ty?;
        let mut cx = match own.sigs.get(name) {
            Some(sig) if !sig.redefinition => return Some(sig.clone()),
            Some(_) => own,
            // declared by no `METHODS` of its definition
            None => return Some(Sig::default()),
        };
        for _ in 0..16 {
            cx = self.classes.get(cx.base.as_ref()?)?;
            cx.ty?;
            if let Some(sig) = cx.sigs.get(name).filter(|s| !s.redefinition) {
                return Some(sig.clone());
            }
        }
        None
    }

    /// End the procedure being lowered at byte `end`: its locals go out of
    /// scope there.
    fn close_body(&mut self, end: usize) {
        if let Some(body) = self.body.take() {
            for l in &mut self.f.locals[body.locals..] {
                l.until = end as u32;
            }
        }
        self.in_scope.clear();
        self.declared_by = None;
        self.nesting = 0;
    }

    /// Where a statement's sites are: `(function, scope)`. Outside every
    /// procedure, a declaration part's sites belong to no function, and a
    /// program's statements are its report's body.
    fn site_cx(&self) -> (Option<u32>, u32) {
        match (self.body, self.decl_scope) {
            (Some(b), _) => (Some(b.func), b.scope),
            (None, Some(scope)) => (None, scope),
            (None, None) => (self.report, 1),
        }
    }

    /// What a statement in a procedure declares: `DATA x TYPE REF TO y` and
    /// `STATICS`, `CONSTANTS` and `FIELD-SYMBOLS` are locals of their declared
    /// types, `TYPES` are aliases; an inline `DATA(x)`, `FINAL(x)` or
    /// `FIELD-SYMBOL(<x>)` (in a `LOOP` or a `READ TABLE`) is untyped, but
    /// `CATCH zcx_x INTO DATA(x)` is a `zcx_x`. Outside every procedure, a
    /// program's `DATA` and `TYPES` are its globals.
    fn locals(&mut self, head: &str, st: &Statement) {
        let Some(body) = self.body else {
            return self.globals(head, st);
        };
        let code = self.code;
        if matches!(head, "DATA" | "STATICS" | "CONSTANTS" | "FIELD-SYMBOLS") {
            for (name, at, decl) in self.declared(st) {
                self.local(body.func, &name, st.tokens[0].start_byte, &decl, None);
                self.declaring.push(at);
            }
        }
        if head == "TYPES" {
            // a method's types are its class's; a form's its unit's
            let scope = if body.method { body.scope } else { 1 };
            self.types(st, scope);
        }
        let words: Vec<&str> = st.tokens.iter().map(|t| t.text(code)).collect();
        let caught = match head {
            "CATCH" => {
                let classes: Vec<&str> = words[1..]
                    .iter()
                    .take_while(|w| !w.eq_ignore_ascii_case("INTO"))
                    .filter(|w| {
                        !w.eq_ignore_ascii_case("BEFORE") && !w.eq_ignore_ascii_case("UNWIND")
                    })
                    .copied()
                    .collect();
                match classes[..] {
                    [one] => Decl::Type(class_ref(one)),
                    _ => Decl::Untyped,
                }
            }
            _ => Decl::Untyped,
        };
        for w in words {
            if let Some(name) = inline_name(w.trim_start_matches('@')) {
                self.local(body.func, &name, st.tokens[0].start_byte, &caught, None);
            }
        }
    }

    /// A program's `DATA` and `CONSTANTS` outside every procedure: values of
    /// its unit, seen by all its procedures. Its `TYPES` likewise.
    fn globals(&mut self, head: &str, st: &Statement) {
        match head {
            "DATA" | "CONSTANTS" | "STATICS" => {
                for (name, at, decl) in self.declared(st) {
                    let row = self.row(at);
                    let te = decl.type_expr();
                    self.f.values.push(ValueDecl {
                        name,
                        row,
                        scope: 1,
                        ty: (te != TypeExpr::Unknown).then_some(te),
                        init: None,
                    });
                    self.declaring.push(at);
                }
            }
            "TYPES" => self.types(st, 1),
            _ => {}
        }
    }

    /// `x = <expression>.` An inline `DATA(x) = <expression>` declares a
    /// local of the expression's type, and a `NEW #( )` or `CAST #( )` in it
    /// is of the declared type of `x`.
    fn assignment(&mut self, st: &Statement, words: &[&str]) {
        let (lhs, Some(rhs)) = (st.tokens[0], st.tokens.get(2)) else {
            return;
        };
        let inline = inline_name(words[0]);
        let target = match inline {
            Some(_) => None,
            None => Some(self.expressions(lhs.start_byte, lhs.end_byte, false, None)),
        };
        let value = self.expressions(rhs.start_byte, before_period(st), false, target);
        if let (Some(name), Some(body)) = (inline, self.body) {
            let init = (value != ExprId::UNKNOWN).then_some(value);
            self.local(body.func, &name, lhs.start_byte, &Decl::Untyped, init);
        }
    }

    /// `CREATE OBJECT lo [TYPE zcl_x] [EXPORTING ...]`: a call of the
    /// constructor of `zcl_x`, or else of the class `lo` is declared to
    /// refer to. `TYPE (lv)` names the class at run time (story 2.5).
    fn create_object(&mut self, st: &Statement, words: &[&str]) {
        let Some(&target) = st.tokens.get(2) else {
            return;
        };
        let value = self.expressions(target.start_byte, target.end_byte, false, None);
        let mut args = 3;
        if words.get(3).is_some_and(|w| w.eq_ignore_ascii_case("TYPE")) {
            args = 5;
            if let Some(class) = words.get(4).filter(|w| !w.starts_with('(')) {
                let path = type_path(class);
                self.constructor(&path, st.tokens[4].start_byte);
            }
        } else {
            match self.declared_type(value).as_ref().and_then(class_path) {
                Some(path) => self.constructor(&path, target.start_byte),
                None => self.opaque_site(NO_TARGET, target.start_byte),
            }
        }
        if let Some(t) = st.tokens.get(args) {
            self.expressions(t.start_byte, before_period(st), false, None);
        }
    }

    /// The type a value is declared with in this file: a local's or a
    /// parameter's, or an attribute's of the class being implemented.
    fn declared_type(&self, value: ExprId) -> Option<TypeExpr> {
        let attr = |name: &str| {
            let class = self.class.as_ref()?;
            self.classes[class].attrs.get(name).cloned()
        };
        match self.f.expr(value) {
            Expr::Path(p) if p.len == 1 => {
                let sym = self.f.path_pool[p.start as usize];
                let body = self.body?;
                let local = self.f.locals[body.locals..]
                    .iter()
                    .rev()
                    .find(|l| l.name == sym);
                match local {
                    Some(l) => match (l.ty, l.init.map(|i| self.f.expr(i))) {
                        (Some(t), _) | (None, Some(Expr::Typed(t))) => {
                            Some(self.f.type_pool[t as usize].clone())
                        }
                        _ => None,
                    },
                    None => attr(self.f.sym(sym)),
                }
            }
            Expr::Field(recv, name) => match self.f.expr(recv) {
                Expr::Path(p)
                    if p.len == 1 && self.f.sym(self.f.path_pool[p.start as usize]) == "me" =>
                {
                    attr(self.f.sym(name))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// A call of the constructor of class `path` at byte `at`.
    fn constructor(&mut self, path: &[Name], at: usize) {
        let callee = self.path_expr(path);
        let expr = self.node(Expr::Call(callee));
        self.site(SiteKind::Call, expr, at);
    }

    /// A call nothing declared binds, for reason `why`.
    fn opaque_site(&mut self, why: &'static str, at: usize) {
        let expr = self.node(Expr::Opaque(why));
        self.site(SiteKind::Call, expr, at);
    }

    /// A name read and not called, at byte `at`: a reference, an edge when
    /// it names a class, an interface, a type, an attribute or a global.
    fn reference(&mut self, segs: &[Name], at: usize) {
        let expr = self.path_expr(segs);
        self.site(SiteKind::Ref, expr, at);
    }

    fn site(&mut self, kind: SiteKind, expr: ExprId, at: usize) {
        let (func, scope) = self.site_cx();
        let row = self.row(at);
        self.f.sites.push(Site {
            func,
            scope,
            row,
            at: at as u32,
            kind,
            expr,
        });
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

    /// `PERFORM f ...`, `PERFORM: f, g.`: a form of this object's unit
    /// (its program and includes). `IN PROGRAM p` is a form of `p`'s unit
    /// ([`Self::perform_in_program`]); a dynamic name (`(f)`) is a dynamic
    /// site, and the old `f(p)` is not read.
    fn perform(&mut self, st: &Statement, words: &[&str]) {
        let upper: Vec<String> = words.iter().map(|w| w.to_ascii_uppercase()).collect();
        if let Some(i) = upper.windows(2).position(|w| w[0] == "IN" && w[1] == "PROGRAM") {
            self.perform_in_program(st, words, i + 2);
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
                self.declaring.push(t.start_byte);
            }
            next = false;
        }
        if let Some(t) = st.tokens.get(2) {
            self.expressions(t.start_byte, before_period(st), false, None);
        }
        self.declaring.clear();
    }

    /// `PERFORM f IN PROGRAM x`: the form `f` of program `x`'s unit, as the
    /// two-segment path `program:x`, `f` (the program is its own name space:
    /// a bare `x` is the report's function). A computed program (`IN PROGRAM (lv)`) or
    /// form (`PERFORM (lv) IN PROGRAM x`) is a dynamic site, reported by
    /// [`Self::dynamic_sites_before`]; the old `PERFORM f(x)` is not read.
    fn perform_in_program(&mut self, st: &Statement, words: &[&str], program_at: usize) {
        let (Some(form), Some(program)) = (words.get(1), words.get(program_at)) else {
            return;
        };
        if matches!(*form, ":" | ",") || form.contains('(') || program.contains('(') {
            return;
        }
        let at = st.tokens[1].start_byte;
        let (func, scope) = self.site_cx();
        let func = func.filter(|_| !self.body.is_some_and(|b| b.method));
        let callee = self.path_expr(&[program_key(&fold(program)), fold(form)]);
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

    /// One unresolved site for each computed call (`abap_dynamic`) that
    /// starts before byte `end`, in the context the walk is in: the reason
    /// is the form's, and it names no edge. Calls inside a macro's body are
    /// not the file's.
    fn dynamic_sites_before(&mut self, end: usize) {
        while let Some(d) = self.dynamic.get(self.next_dynamic) {
            if d.at >= end {
                break;
            }
            let (at, reason) = (d.at, d.form.reason());
            self.next_dynamic += 1;
            if self.in_macro {
                continue;
            }
            let index = DYNAMIC_REASONS.iter().position(|r| *r == reason).unwrap_or(0);
            let expr = self.node(Expr::Dynamic(index as u8));
            self.site(SiteKind::Call, expr, at);
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
            self.expressions(t.start_byte, end, false, None);
        }
    }

    /// The calls and references in `code[from..to]`: every name directly
    /// followed by `(` and a blank or `)` is called (`m( )`, `lo->m( x )`,
    /// `zcl_x=>m( )`), unlike an offset (`s+1(2)`) or an inline declaration
    /// (`DATA(x)`), and any other name read is a reference. With
    /// `call_method`, the first chain is called though it has no
    /// parentheses (`CALL METHOD lo->m EXPORTING ...`).
    ///
    /// Returns the range's value when it is one expression, as the
    /// right-hand side of an assignment is: a chain, `NEW zcl_x( )`, `CAST
    /// zif_x( lo )`. `target` is what it is assigned to, whose declared type
    /// a `NEW #( )` or `CAST #( )` takes.
    fn expressions(
        &mut self,
        from: usize,
        to: usize,
        call_method: bool,
        target: Option<ExprId>,
    ) -> ExprId {
        let b = self.code.as_bytes();
        let mut parens: Vec<Option<ExprId>> = Vec::new();
        // a constructor operator just read: the type after it
        let mut operator: Option<Name> = None;
        let mut forced = call_method;
        // the top level's value, and how many pieces it has
        let mut value = ExprId::UNKNOWN;
        let mut pieces = 0;
        let mut i = from;
        while i < to {
            let c = b[i];
            if c.is_ascii_whitespace() {
                i += 1;
                continue;
            }
            // only a name right at the start is `CALL METHOD`'s
            let first = std::mem::take(&mut forced);
            let op = operator.take();
            // `NEW #( )`, `CAST #( )`: of the target's type
            if c == b'#' && b.get(i + 1) == Some(&b'(') {
                let inferred = match op.as_deref() {
                    Some(op @ ("new" | "cast")) => Some(self.inferred(op == "new", target, i)),
                    _ => None,
                };
                parens.push(inferred);
                i += 2;
                continue;
            }
            if c == b'(' {
                parens.push(None);
                i += 1;
                continue;
            }
            if c == b')' {
                // `)->m( )`: a call on the parenthesized value
                let inner = parens.pop().flatten().unwrap_or(ExprId::UNKNOWN);
                let (next, ended) =
                    self.chain(Link::Value(inner), i + 1, to, &mut parens, None, false);
                if let (Some(link), true) = (ended, parens.is_empty()) {
                    value = self.value(link);
                    pieces += 1;
                }
                i = next;
                continue;
            }
            let fresh = i == from || !name_byte(b[i - 1]);
            let Some((name, end)) = fresh.then(|| self.name(i, to)).flatten() else {
                pieces += usize::from(parens.is_empty());
                i += 1;
                continue;
            };
            // `NEW zcl_x( )`, `CONV i( x )`, `CAST #( )`: an operator, then a type
            let typed = b.get(end).is_some_and(u8::is_ascii_whitespace)
                && self.code[end..to]
                    .trim_start()
                    .bytes()
                    .next()
                    .is_some_and(|c| name_byte(c) || c == b'#' || c == b'/');
            if typed && (OPERATORS.contains(&&*name) || &*name == "new") {
                operator = Some(name);
                i = end;
                continue;
            }
            let top = parens.is_empty();
            let (link, next) = self.head(name, i, end, to);
            let (next, ended) = self.chain(link, next, to, &mut parens, op.as_deref(), first);
            if let (Some(link), true) = (ended, top) {
                value = self.value(link);
                pieces += 1;
            }
            i = next;
        }
        if pieces == 1 {
            value
        } else {
            ExprId::UNKNOWN
        }
    }

    /// `NEW #( )`, `CAST #( )` at byte `at`: of the declared type of the
    /// `target` assigned to, a local, a parameter or an attribute of this
    /// class. A `NEW` calls that class's constructor.
    fn inferred(&mut self, new: bool, target: Option<ExprId>, at: usize) -> ExprId {
        let declared = target.and_then(|t| self.declared_type(t));
        if new {
            match declared.as_ref().and_then(class_path) {
                Some(path) => self.constructor(&path, at),
                None => self.opaque_site(NO_TARGET, at),
            }
        }
        match (declared, target) {
            (Some(te), _) => self.typed(te),
            // declared elsewhere: typed when it is resolved (an inherited attribute)
            (None, Some(target)) => target,
            (None, None) => self.node(Expr::Opaque(NO_TARGET)),
        }
    }

    /// The first link of a chain, `class=>member` or a name at byte `at`,
    /// and where it ends. A name read and not called is a reference, and so
    /// is the class of `class=>member` and the interface of `zif_x~m`.
    fn head(&mut self, name: Name, at: usize, end: usize, to: usize) -> (Link, usize) {
        if let Some((iface, _)) = name.split_once('~') {
            self.reference(&[iface.into()], at);
        }
        if let Some((member, end)) = self.arrow(end, to, b"=>") {
            if self.read(&name, at) {
                self.reference(std::slice::from_ref(&name), at);
            }
            return (Link::Path(vec![name, member]), end);
        }
        let called = self.code.as_bytes().get(end) == Some(&b'(');
        if !called && self.read(&name, at) {
            self.reference(std::slice::from_ref(&name), at);
        }
        (Link::Path(vec![name]), end)
    }

    /// Whether the name at byte `at` may be a reference: not a local, not a
    /// name being declared, not the class or interface a declaration part
    /// is in, not `me`, a field symbol nor a keyword (unless the class has
    /// an attribute of that name).
    fn read(&self, name: &str, at: usize) -> bool {
        let own = self.decl_scope.is_some()
            && (self.class.as_deref() == Some(name)
                || self.interface.as_ref().is_some_and(|(i, _)| &**i == name));
        if own
            || self.in_scope.contains(name)
            || self.declaring.contains(&at)
            || matches!(name, "me" | "super")
            || name.starts_with('<')
            || name.contains('~')
        {
            return false;
        }
        let attr = self
            .class
            .as_ref()
            .is_some_and(|c| self.classes[c].attrs.contains_key(name));
        attr || !is_abap_keyword(name)
    }

    /// Read the rest of a chain from byte `i`: `->m`, `=>m` and `-c` links,
    /// up to a call's `(` (pushed on `parens`, so the chain goes on after
    /// its `)`) or the chain's end. After an `operator` the first link is a
    /// type: `NEW zcl_x( )` calls its constructor and is a `zcl_x`, `CAST
    /// zif_x( lo )` is a `zif_x`, `CONV i( x )` calls nothing. Returns where
    /// it stopped, and the chain when it ended there.
    fn chain(
        &mut self,
        mut link: Link,
        mut i: usize,
        to: usize,
        parens: &mut Vec<Option<ExprId>>,
        operator: Option<&str>,
        call_method: bool,
    ) -> (usize, Option<Link>) {
        let b = self.code.as_bytes();
        loop {
            if i < to && b[i] == b'(' {
                let called = b
                    .get(i + 1)
                    .is_some_and(|c| c.is_ascii_whitespace() || *c == b')');
                let value = match operator {
                    Some("new") => {
                        let ty = self.link_type(&link);
                        if called {
                            self.call(link, i);
                        }
                        ty
                    }
                    Some(op) => {
                        let ty = if op == "cast" {
                            self.link_type(&link)
                        } else {
                            None
                        };
                        if let Link::Path(segs) = &link {
                            if segs.len() == 1 && self.read(&segs[0], i - segs[0].len()) {
                                self.reference(segs, i - segs[0].len());
                            }
                        }
                        ty
                    }
                    None if called => self.call(link, i),
                    None => None,
                };
                parens.push(value);
                return (i + 1, None);
            }
            if let Some((member, end)) = self
                .arrow(i, to, b"->")
                .or_else(|| self.arrow(i, to, b"=>"))
            {
                if let Some((iface, _)) = member.split_once('~') {
                    self.reference(&[iface.into()], i + 2);
                }
                // `me->x`, not called, reads the attribute `x`
                let own = matches!(&link, Link::Path(segs) if segs.len() == 1 && &*segs[0] == "me");
                if own && b.get(end) != Some(&b'(') && !member.contains('~') {
                    self.reference(std::slice::from_ref(&member), i + 2);
                }
                let recv = self.receiver(link);
                link = Link::Member(recv, member, i + 2);
                i = end;
                continue;
            }
            // a structure component, `s-c`; not `a - b` nor `->`
            if i + 1 < to && b[i] == b'-' && b[i + 1] != b'>' {
                if let Some((member, end)) = self.name(i + 1, to) {
                    let recv = self.receiver(link);
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
            return (i, None);
        }
        (i, Some(link))
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

    /// A link a member is read from. In a method whose parameters are
    /// declared in another file, a bare name that is none of its locals may
    /// be one of them: an attribute or a global if it is one, else the
    /// parameter of that name the interface or the base class declares,
    /// else unknown for that reason.
    fn receiver(&mut self, link: Link) -> ExprId {
        if let (Some(true), Link::Path(segs)) = (self.body.map(|b| b.elsewhere), &link) {
            if let [name] = segs.as_slice() {
                if !matches!(&**name, "me" | "super") && !self.in_scope.contains(name) {
                    let mut arms = vec![self.path_expr(segs)];
                    if let Some((owner, method)) = self.declared_by.clone() {
                        let (o, m, p) = (self.sym(&owner), self.sym(&method), self.sym(name));
                        arms.push(self.node(Expr::Signature(o, m, p)));
                    }
                    arms.push(self.node(Expr::Opaque(SIGNATURE_ELSEWHERE)));
                    let arms = self.list(&arms);
                    return self.node(Expr::Branches(arms));
                }
            }
        }
        self.value(link)
    }

    /// The class a constructor operator's type link names, as a value of it.
    fn link_type(&mut self, link: &Link) -> Option<ExprId> {
        match link {
            Link::Path(segs) => {
                let te = class_ref_path(segs.clone());
                Some(self.typed(te))
            }
            _ => None,
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
        self.site(SiteKind::Call, expr, at);
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

/// A type name as written, folded: `zif_x=>ty` is the path `zif_x`, `ty`.
fn type_path(name: &str) -> Vec<Name> {
    name.split("=>").map(fold).collect()
}

/// `REF TO x`: a reference to class or interface `x`. The generic `object`,
/// `data` and `any` name nothing a call can bind to.
fn class_ref(name: &str) -> TypeExpr {
    class_ref_path(type_path(name))
}

fn class_ref_path(segs: Vec<Name>) -> TypeExpr {
    if let [one] = segs.as_slice() {
        if matches!(&**one, "object" | "data" | "any") {
            return TypeExpr::Opaque(GENERIC);
        }
    }
    TypeExpr::Ref(Box::new(TypeExpr::Named {
        path: Path(segs),
        args: Vec::new(),
    }))
}

/// The class a `REF TO` type names, whose constructor `NEW` calls.
fn class_path(te: &TypeExpr) -> Option<Vec<Name>> {
    match te {
        TypeExpr::Ref(inner) => match &**inner {
            TypeExpr::Named { path, .. } => Some(path.0.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The name an inline declaration declares: `DATA(x)`, `FINAL(x)`,
/// `FIELD-SYMBOL(<x>)`.
fn inline_name(word: &str) -> Option<Name> {
    let lower = word.to_ascii_lowercase();
    ["data(", "final(", "field-symbol("]
        .iter()
        .find_map(|prefix| {
            let (name, _) = lower.strip_prefix(prefix)?.split_once(')')?;
            Some(name.into())
        })
}

/// The type clause from word `i` (`TYPE`, `LIKE`, `STRUCTURE`): what it
/// declares, and the index of the word after it. Only a `TYPE REF TO`
/// names a class; a table or a line of one holds no reference.
fn type_clause(words: &[&str], i: usize) -> (Decl, usize) {
    let up = |k: usize| words.get(k).map(|w| w.to_ascii_uppercase());
    let is = |k: usize, w: &str| up(k).as_deref() == Some(w);
    match up(i).as_deref() {
        Some("TYPE") if is(i + 1, "REF") && is(i + 2, "TO") => match words.get(i + 3) {
            Some(x) => (Decl::Type(class_ref(x)), i + 4),
            None => (Decl::Untyped, i + 3),
        },
        Some("TYPE" | "LIKE")
            if matches!(
                up(i + 1).as_deref(),
                Some(
                    "STANDARD"
                        | "SORTED"
                        | "HASHED"
                        | "ANY"
                        | "INDEX"
                        | "TABLE"
                        | "LINE"
                        | "RANGE"
                        | "REF"
                )
            ) =>
        {
            let mut k = i + 1;
            let mut of = false;
            while let Some(w) = up(k) {
                if !matches!(
                    w.as_str(),
                    "STANDARD"
                        | "SORTED"
                        | "HASHED"
                        | "ANY"
                        | "INDEX"
                        | "TABLE"
                        | "LINE"
                        | "RANGE"
                        | "OF"
                        | "REF"
                        | "TO"
                ) {
                    break;
                }
                of = matches!(w.as_str(), "OF" | "TO");
                k += 1;
            }
            // the element's type, if one is named
            (Decl::Untyped, k + usize::from(of))
        }
        Some("TYPE") => match words.get(i + 1) {
            Some(x) => (
                Decl::Type(TypeExpr::Named {
                    path: Path(type_path(x)),
                    args: Vec::new(),
                }),
                i + 2,
            ),
            None => (Decl::Untyped, i + 1),
        },
        Some("LIKE") => match words.get(i + 1) {
            Some(x) => (Decl::Like(fold(x)), i + 2),
            None => (Decl::Untyped, i + 1),
        },
        Some("STRUCTURE") => (Decl::Untyped, i + 2),
        _ => (Decl::Untyped, i),
    }
}

/// The parameters a signature lists, `(word, offset)` from after the
/// method's or the form's name: `IMPORTING a TYPE x b TYPE REF TO y`,
/// `RETURNING VALUE(r) TYPE z`, `USING p CHANGING q TYPE i`. A name is a
/// parameter when a `TYPE`, `LIKE` or `STRUCTURE` follows it, or, where
/// `untyped` ones are allowed (a form's, an event handler's), whenever it
/// stands where a parameter does. Each says whether it is the `RETURNING`
/// one.
fn parameters(words: &[(&str, usize)], untyped: bool) -> Vec<(Param, bool)> {
    let texts: Vec<&str> = words.iter().map(|(w, _)| *w).collect();
    let mut out = Vec::new();
    let mut section: Option<String> = None;
    let mut i = 0;
    while i < words.len() {
        let (w, at) = words[i];
        let up = w.to_ascii_uppercase();
        if SECTIONS.contains(&up.as_str()) {
            section = Some(up);
            i += 1;
            continue;
        }
        if NOT_PARAMETERS.contains(&up.as_str()) {
            section = None;
        }
        match up.as_str() {
            _ if section.is_none() => i += 1,
            "OPTIONAL" => i += 1,
            "DEFAULT" => i += 2,
            "PREFERRED" => i += 3,
            _ => {
                let typed = texts.get(i + 1).is_some_and(|n| {
                    matches!(&*n.to_ascii_uppercase(), "TYPE" | "LIKE" | "STRUCTURE")
                });
                if !typed && !untyped {
                    i += 1;
                    continue;
                }
                let (decl, next) = match typed {
                    true => type_clause(&texts, i + 1),
                    false => (Decl::Untyped, i + 1),
                };
                let name = param_name(w);
                let returning = section.as_deref() == Some("RETURNING");
                out.push((Param { name, at, decl }, returning));
                i = next;
            }
        }
    }
    out
}

/// A parameter's name as declared: `VALUE(x)`, `REFERENCE(x)`, `!x`.
fn param_name(word: &str) -> Name {
    let lower = word.to_ascii_lowercase();
    let bare = ["value(", "reference("]
        .iter()
        .find_map(|p| lower.strip_prefix(p))
        .map_or(lower.as_str(), |rest| rest.trim_end_matches(')'));
    bare.trim_start_matches('!').into()
}

// ---------------------------------------------------------------------------
// Layout: one root for the global names, one directory per unit
// ---------------------------------------------------------------------------

/// Every file pools its scope 0 into the root, where the global classes,
/// interfaces, function modules and reports are seen repo-wide. Its scope 1
/// lives in its compiled unit's directory (`local_home`), which the parts of
/// one class or function group share, and which falls back to the root
/// (`dirs_fall_back`): a unit sees its own local names first, and never
/// another unit's. The unit is the object, joined with the objects its
/// `INCLUDE`s name ([`IncludeGraph`]): a program with its includes is one.
/// Each program is also importable by its name, so `PERFORM f IN PROGRAM x`
/// (the path `program:x`, `f`) looks in `x`'s unit only. An `INCLUDE` of a file the
/// repo does not hold is counted as unresolved. A file with no abapGit name
/// is an object of its own.
fn layout(files: &[(&str, &FileFacts)]) -> Layout {
    let paths: Vec<String> = files.iter().map(|(p, _)| p.to_string()).collect();
    let named: Vec<(&str, Vec<String>)> = files
        .iter()
        .filter(|(p, f)| !f.includes.is_empty() && reads_includes(p))
        .map(|(p, f)| (*p, f.includes.iter().map(|n| n.to_string()).collect()))
        .collect();
    let includes = IncludeGraph::from_names(&paths, &named);
    let mut dirs: Vec<(String, Option<usize>)> = vec![(String::new(), None)];
    let mut units: HashMap<String, usize> = HashMap::default();
    let mut dir_crates: HashMap<String, usize> = HashMap::default();
    let mut orphan_dir = Vec::with_capacity(files.len());
    let mut local_home = Vec::with_capacity(files.len());
    for (path, _) in files {
        let file = path.rsplit('/').next().unwrap_or(path);
        let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
        orphan_dir.push(Some((0, stem.to_string())));
        local_home.push(parse_abapgit_name(path).map(|object| {
            let name = object.name.to_ascii_lowercase();
            let unit = includes.unit(&name);
            let dir = *units.entry(unit.clone()).or_insert_with(|| {
                dirs.push((unit, Some(0)));
                dirs.len() - 1
            });
            if object.object_type == "prog" {
                dir_crates.entry(program_key(&name).to_string()).or_insert(dir);
            }
            dir
        }));
    }
    let mut unresolved: HashMap<&'static str, usize> = HashMap::default();
    for missing in &includes.unresolved {
        *unresolved.entry(missing.reason()).or_default() += 1;
    }
    debug_assert!(includes.unresolved.iter().all(|m| m.reason() == INCLUDE_NOT_IN_REPO));
    Layout {
        parent_of: vec![None; files.len()],
        dirs,
        orphan_dir,
        pooled: vec![true; files.len()],
        local_home,
        dir_crates,
        dirs_fall_back: true,
        unresolved,
        ..Layout::default()
    }
}
