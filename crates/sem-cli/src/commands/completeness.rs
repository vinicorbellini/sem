//! Is a caller set complete? The evidence behind a "none".
//!
//! The static graph resolves the calls it can pin. A real sympy change
//! (SWE-bench sympy-12489) showed what that misses: `_af_new = Permutation._af_new` at
//! module level, then fifteen sibling methods call `_af_new(...)`. Every one
//! of those calls was unresolved, and `sem callers _af_new` said
//! "(callers: none)". A confident none an agent cannot trust is worse than no
//! answer.
//!
//! So every caller answer carries a verdict built from the source text:
//!
//! - every code mention of the name (comments and docstrings excluded) that
//!   is not the definition itself and not inside a resolved caller is an
//!   *unexplained mention*: a bare call the resolver could not bind (alias,
//!   import, scope), a member call through an untyped receiver, the name
//!   passed as a value, a string key equal to the name, an alias assignment
//!   (`x = Owner.name`, followed one hop: mentions of `x` count too);
//! - a dispatch or registration decorator on the definition
//!   (`@dispatch(...)`, `@*.register`, `@singledispatch*`, `@overload`,
//!   `@receiver`, route decorators, `@property`, `@pytest.fixture`) means it
//!   is reached through a registry or attribute access, not by a direct call;
//! - a `getattr(obj, 'prefix' + …)` (or `f'prefix{…}'`, `'prefix%s' % …`)
//!   anywhere in the corpus whose prefix starts the name can reach it with no
//!   mention at all.
//!
//! `complete` holds only when none of those exist. That is a bounded claim:
//! no textual path to the definition exists other than the resolved
//! callers. A fully computed name (`getattr(o, name)` with `name` from data)
//! remains outside it, and the verdict says so in `checked`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// How a line mentions the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A call whose target name is computed at run time (ABAP `CALL FUNCTION lv`,
    /// `CALL METHOD zcl_x=>(lv)`). Counts as a caller of what it can reach.
    Dynamic,
    /// `x = Owner.name` / `x = name`: an alias the resolver does not follow.
    Alias,
    /// `name(...)` not bound by the resolver (alias, import, scope).
    Call,
    /// `obj.name(...)` with a receiver the resolver could not type.
    MemberCall,
    /// ABAP `zif_x~name( )`: a call through an interface component selector.
    InterfaceCall,
    /// `'name'` as a whole string literal: getattr, registries, dispatch tables.
    StringKey,
    /// The name passed or stored as a value (callback, registry, `obj.name`).
    ValueRef,
    /// `from m import name` / `import name as x`: binds, does not call.
    Import,
    /// Another definition of the same name (an override, a sibling class).
    Definition,
}

impl Kind {
    fn counts_as_caller(self) -> bool {
        !matches!(self, Kind::Import | Kind::Definition)
    }
    /// The reason text. ABAP words some kinds differently: its `->` receiver is an
    /// untyped reference until story 2.2 binds receivers, and a string equal to a
    /// name is how a function module is called through a variable.
    fn words(self, abap: bool, function: bool) -> &'static str {
        if abap {
            match self {
                Kind::StringKey if !function => return "string literal equal to the name (a computed call or dispatch table can use it)",
                Kind::MemberCall => return "call through an untyped reference (2.2): `->` or `=>` call the resolver did not bind",
                Kind::StringKey => return "string-keyed function module name (the literal a `CALL FUNCTION lv` can hold)",
                Kind::Call => return "call by bare name not bound by the resolver (PERFORM, CALL METHOD, or a method of the same class)",
                _ => {}
            }
        }
        match self {
            Kind::Dynamic => "dynamic call (name computed at run time)",
            Kind::InterfaceCall => "call through an interface (2.3)",
            Kind::Alias => "alias assignment",
            Kind::Call => "call by bare name not bound by the resolver (alias, import or scope)",
            Kind::MemberCall => "member call with an untyped receiver (may be another class's member)",
            Kind::StringKey => "string literal equal to the name (getattr, registry or dispatch table)",
            Kind::ValueRef => "name used as a value (callback, registry, attribute access)",
            Kind::Import => "import",
            Kind::Definition => "another definition of the same name",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Mention {
    pub file: String,
    pub line: usize,
    pub kind: Kind,
    pub text: String,
    /// For `Kind::Alias` / `import … as x`: the alias name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// For a mention found by following an alias: the alias name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PossibleCaller {
    /// Enclosing entity (`Owner.name`), or `<module level>`.
    pub entity: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub file: String,
    /// The enclosing entity's first line (0 at module level).
    pub start_line: usize,
    pub sites: Vec<Site>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub line: usize,
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Reason {
    pub code: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Verdict {
    pub complete: bool,
    pub incomplete_because: Vec<Reason>,
    /// What the verdict checked, in words, so a "complete" is a bounded claim.
    pub checked: String,
    pub possible_callers: Vec<PossibleCaller>,
    pub possible_caller_sites: usize,
}

/// Byte class of a source position.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Code,
    Comment,
    Str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Python,
    Hash, // ruby, shell, yaml-ish: `#` comments
    CLike,
    Rust,
    /// ABAP: classes come from `strip_abap_content`, names fold case.
    Abap,
    Other,
}

fn family(path: &str) -> Family {
    match path.rsplit_once('.').map(|(_, e)| e).unwrap_or("") {
        "py" | "pyi" | "pyx" => Family::Python,
        "rb" | "sh" | "bash" | "pl" | "r" | "ex" | "exs" => Family::Hash,
        "rs" => Family::Rust,
        "abap" => Family::Abap,
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "go" | "java" | "kt" | "kts" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs"
        | "swift" | "scala" | "dart" | "php" | "vue" | "svelte" => Family::CLike,
        _ => Family::Other,
    }
}

/// Per-byte classes plus string-literal content spans.
struct Lexed {
    class: Vec<Class>,
    /// (content_start, content_end) of each string literal.
    strings: Vec<(usize, usize)>,
    /// ABAP only: the stripped text (comments and literals blanked), lower-cased,
    /// the same length as the source. Call shapes are read off it.
    code: Vec<u8>,
}

/// Whether `path` is ABAP source (names compare case-insensitively).
pub fn is_abap(path: &str) -> bool {
    family(path) == Family::Abap
}

/// ABAP classes from the one stripper. A byte the stripper blanked starts at a
/// non-blank character, so the literal or comment is re-read from the source at that
/// point (blanked spaces cannot be told from code spaces, but the span's first byte can).
fn lex_abap(src: &[u8]) -> Lexed {
    let n = src.len();
    let text = String::from_utf8_lossy(src);
    let stripped = if text.len() == n { sem_core::parser::plugins::code::strip_abap_content(&text).into_bytes() } else { src.to_vec() };
    let mut class = vec![Class::Code; n];
    let mut strings = Vec::new();
    let mut i = 0;
    while i < n {
        if stripped[i] == src[i] {
            i += 1;
            continue;
        }
        match src[i] {
            q @ (b'\'' | b'`') => {
                let start = i;
                i += 1;
                let mut content_end = n;
                while i < n && src[i] != b'\n' {
                    if src[i] == q {
                        if src.get(i + 1) == Some(&q) {
                            i += 2;
                            continue;
                        }
                        content_end = i;
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                if content_end == n {
                    content_end = i;
                }
                class[start..i].iter_mut().for_each(|c| *c = Class::Str);
                strings.push((start + 1, content_end));
            }
            b'*' | b'"' => {
                let start = i;
                while i < n && src[i] != b'\n' {
                    i += 1;
                }
                class[start..i].iter_mut().for_each(|c| *c = Class::Comment);
            }
            b'|' | b'}' => {
                // the literal part of a `|...|` template, up to `{` or the closing `|`
                let start = i;
                i += 1;
                while i < n {
                    match src[i] {
                        b'\\' => i = (i + 2).min(n),
                        b'|' | b'{' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                class[start..i].iter_mut().for_each(|c| *c = Class::Str);
            }
            _ => {
                class[i] = Class::Str;
                i += 1;
            }
        }
    }
    Lexed { class, strings, code: stripped.to_ascii_lowercase() }
}

fn lex(path: &str, src: &[u8]) -> Lexed {
    let fam = family(path);
    if fam == Family::Abap {
        return lex_abap(src);
    }
    let n = src.len();
    let mut class = vec![Class::Code; n];
    let mut strings = Vec::new();
    if fam == Family::Other {
        return Lexed { class, strings, code: Vec::new() };
    }
    let mut i = 0;
    while i < n {
        let b = src[i];
        // comments
        let line_comment = match fam {
            Family::Python | Family::Hash => b == b'#',
            Family::CLike | Family::Rust => b == b'/' && src.get(i + 1) == Some(&b'/'),
            Family::Abap | Family::Other => false,
        };
        if line_comment {
            while i < n && src[i] != b'\n' {
                class[i] = Class::Comment;
                i += 1;
            }
            continue;
        }
        if matches!(fam, Family::CLike | Family::Rust) && b == b'/' && src.get(i + 1) == Some(&b'*') {
            let start = i;
            i += 2;
            while i < n && !(src[i] == b'*' && src.get(i + 1) == Some(&b'/')) {
                i += 1;
            }
            i = (i + 2).min(n);
            class[start..i].iter_mut().for_each(|c| *c = Class::Comment);
            continue;
        }
        // strings
        let quote = match (fam, b) {
            (_, b'"') => true,
            (Family::Rust, b'\'') => false, // lifetimes and chars: not names
            (_, b'\'') => true,
            (Family::CLike, b'`') => true,
            _ => false,
        };
        if quote {
            let start = i;
            let triple = fam == Family::Python && src.get(i + 1) == Some(&b) && src.get(i + 2) == Some(&b);
            let raw_prefix = fam == Family::Python && start > 0 && matches!(src[start - 1], b'r' | b'R');
            let (open_len, close): (usize, &[u8]) = if triple {
                (3, if b == b'"' { b"\"\"\"" } else { b"'''" })
            } else {
                (1, std::slice::from_ref(&src[start]))
            };
            i += open_len;
            let content_start = i;
            let mut content_end = n;
            while i < n {
                if src[i] == b'\\' && !raw_prefix {
                    i += 2;
                    continue;
                }
                if src[i..].starts_with(close) {
                    content_end = i;
                    i += close.len();
                    break;
                }
                if !triple && b != b'`' && src[i] == b'\n' {
                    // unterminated single-line string: stop at the line end
                    content_end = i;
                    break;
                }
                i += 1;
            }
            let end = i.min(n);
            class[start..end].iter_mut().for_each(|c| *c = Class::Str);
            strings.push((content_start, content_end.min(n)));
            continue;
        }
        i += 1;
    }
    Lexed { class, strings, code: Vec::new() }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Every whole-word occurrence of `name` in `src`.
fn occurrences(src: &[u8], name: &str) -> Vec<usize> {
    let needle = name.as_bytes();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let finder = memchr::memmem::Finder::new(needle);
    for at in finder.find_iter(src) {
        let before_ok = at == 0 || !is_ident(src[at - 1]);
        let after = at + needle.len();
        let after_ok = after >= src.len() || !is_ident(src[after]);
        if before_ok && after_ok {
            out.push(at);
        }
    }
    out
}

fn line_bounds(src: &[u8], at: usize) -> (usize, usize) {
    let start = src[..at].iter().rposition(|&b| b == b'\n').map_or(0, |p| p + 1);
    let end = src[at..].iter().position(|&b| b == b'\n').map_or(src.len(), |p| at + p);
    (start, end)
}

fn prev_non_space(src: &[u8], at: usize, floor: usize) -> Option<u8> {
    src[floor..at].iter().rev().find(|b| !b.is_ascii_whitespace()).copied()
}

fn next_non_space(src: &[u8], at: usize) -> Option<u8> {
    src[at..].iter().find(|b| !matches!(b, b' ' | b'\t')).copied()
}

/// `x = …name` (whole right-hand side is a dotted path ending in the name).
fn alias_lhs(line: &str, name: &str) -> Option<String> {
    let (lhs, rhs) = line.split_once('=')?;
    if rhs.starts_with('=') || lhs.ends_with(['!', '<', '>', '=', '+', '-', '*', '/', '|', '&', '%', ':']) {
        return None;
    }
    let lhs = lhs.trim();
    let lhs = lhs.strip_prefix("const ").or_else(|| lhs.strip_prefix("let ")).or_else(|| lhs.strip_prefix("var ")).unwrap_or(lhs).trim();
    let rhs = rhs.trim().trim_end_matches(';').trim();
    let rhs_ok = (rhs == name || rhs.ends_with(&format!(".{name}")) || rhs.ends_with(&format!("::{name}")))
        && rhs.bytes().all(|b| is_ident(b) || b == b'.' || b == b':');
    let lhs_ok = !lhs.is_empty() && lhs.bytes().all(|b| is_ident(b) || b == b'.');
    (rhs_ok && lhs_ok).then(|| lhs.rsplit('.').next().unwrap_or(lhs).to_string())
}

fn import_alias(line: &str, name: &str) -> Option<String> {
    // `from m import name as x`, `import name as x`, `use a::name as x;`, `{ name as x }`
    let pat = format!("{name} as ");
    let at = line.find(&pat)?;
    let rest = &line[at + pat.len()..];
    let alias: String = rest.bytes().take_while(|&b| is_ident(b)).map(char::from).collect();
    (!alias.is_empty()).then_some(alias)
}

fn is_import_line(t: &str) -> bool {
    t.starts_with("from ") || t.starts_with("import ") || t.starts_with("use ") || t.starts_with("export {")
        || t.starts_with("export *") || (t.contains("require(") && t.contains("const "))
}

/// Characters of one ABAP operand: names, namespaces, `->`, `=>`, `~`, component `-`.
fn abap_operand_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'<' | b'>' | b'~' | b'=' | b'-' | b'$')
}

/// The word that ends just before `at` (skipping blanks and newlines), with its start.
fn abap_prev_word(code: &[u8], at: usize) -> Option<(&str, usize)> {
    let mut e = at;
    while e > 0 && code[e - 1].is_ascii_whitespace() {
        e -= 1;
    }
    let mut s = e;
    while s > 0 && (code[s - 1].is_ascii_alphanumeric() || matches!(code[s - 1], b'_' | b'-')) {
        s -= 1;
    }
    (s < e).then(|| (std::str::from_utf8(&code[s..e]).unwrap_or(""), s))
}

/// Whether the operand that contains `at` is the one right after `CALL METHOD`.
fn abap_after_call_method(code: &[u8], at: usize) -> bool {
    let mut s = at;
    while s > 0 && abap_operand_byte(code[s - 1]) {
        s -= 1;
    }
    match abap_prev_word(code, s) {
        Some(("method", w)) => matches!(abap_prev_word(code, w), Some(("call", _))),
        _ => false,
    }
}

/// Keywords after which a name is being declared or implemented, not used.
const ABAP_DECL: &[&str] = &[
    "methods", "class-methods", "class", "interface", "form", "function", "module", "method", "define", "data", "class-data", "constants",
    "types", "statics", "events", "class-events", "aliases",
];

/// `METHODS describe`, `METHODS: a, b` (a chain element), `FORM f`: a declaration.
fn abap_is_declaration(code: &[u8], at: usize) -> bool {
    let stmt_start = code[..at].iter().rposition(|&b| b == b'.').map_or(0, |p| p + 1);
    let head = String::from_utf8_lossy(&code[stmt_start..at]).to_string();
    let h = head.trim_start();
    let kw: String = h.bytes().take_while(|&b| b.is_ascii_alphabetic() || b == b'-').map(char::from).collect();
    if !ABAP_DECL.contains(&kw.as_str()) {
        return false;
    }
    let rest = h[kw.len()..].trim();
    if rest.is_empty() {
        return true; // `METHODS name`
    }
    // `METHODS: a, b` -- the name opens a chain element
    rest.strip_prefix(':').is_some_and(|chain| chain.rsplit(',').next().is_some_and(|last| last.trim().is_empty()))
}

/// A whole string literal equal to the name. `CALL FUNCTION 'NAME'` is a static call whose
/// literal the stripper hides; any other literal is a string key (a function module held
/// in a variable, a dispatch table).
fn abap_literal_kind(code: &[u8], at: usize) -> Kind {
    // the opening quote is at `at - 1`
    if let Some(("function", w)) = abap_prev_word(code, at.saturating_sub(1)) {
        if matches!(abap_prev_word(code, w), Some(("call", _))) {
            return Kind::Call;
        }
    }
    Kind::StringKey
}

/// How ABAP code mentions the name, read off the stripped, lower-cased text. `->` and
/// `=>` are receivers, `~` selects an interface component, `PERFORM f` and
/// `CALL METHOD x` name their target without parentheses.
fn abap_code_kind(code: &[u8], at: usize, len: usize) -> Kind {
    let paren = code.get(at + len) == Some(&b'(');
    let before = &code[..at];
    let arrow = before.ends_with(b"->");
    let stat = before.ends_with(b"=>");
    let tilde = before.ends_with(b"~");
    if abap_after_call_method(code, at) {
        return if arrow || stat {
            Kind::MemberCall
        } else if tilde {
            Kind::InterfaceCall
        } else {
            Kind::Call
        };
    }
    if abap_is_declaration(code, at) {
        return Kind::Definition;
    }
    if arrow || stat {
        return if paren { Kind::MemberCall } else { Kind::ValueRef };
    }
    if tilde {
        return if paren { Kind::InterfaceCall } else { Kind::ValueRef };
    }
    if matches!(abap_prev_word(code, at), Some(("perform", _))) || paren {
        return Kind::Call;
    }
    Kind::ValueRef
}

/// Classify every mention of `name` in one file. One mention per line, the
/// strongest kind on that line.
pub fn scan_file(path: &str, src: &str, name: &str) -> Vec<Mention> {
    let bytes = src.as_bytes();
    let abap = is_abap(path);
    // ABAP names compare case-insensitively: fold both sides (ASCII, so byte offsets hold)
    let occ = if abap { occurrences(&bytes.to_ascii_lowercase(), &name.to_ascii_lowercase()) } else { occurrences(bytes, name) };
    if occ.is_empty() {
        return Vec::new();
    }
    let lexed = lex(path, bytes);
    let mut by_line: BTreeMap<usize, Mention> = BTreeMap::new();
    let mut line_no = 1usize;
    let mut scanned_to = 0usize;
    for at in occ {
        line_no += bytes[scanned_to..at].iter().filter(|&&b| b == b'\n').count();
        scanned_to = at;
        let (ls, le) = line_bounds(bytes, at);
        let line_text = &src[ls..le];
        let trimmed = line_text.trim_start();
        let kind = match lexed.class[at] {
            Class::Comment => continue,
            Class::Str => {
                let whole = lexed.strings.iter().any(|&(s, e)| s == at && e == at + name.len());
                if !whole {
                    continue; // prose in a docstring or message, or the literal part of a template
                }
                if abap {
                    abap_literal_kind(&lexed.code, at)
                } else {
                    Kind::StringKey
                }
            }
            Class::Code if abap => abap_code_kind(&lexed.code, at, name.len()),
            Class::Code => {
                let prev = prev_non_space(bytes, at, ls);
                let next = next_non_space(bytes, at + name.len());
                let before = &src[ls..at];
                let before_t = before.trim_end();
                if is_import_line(trimmed) {
                    Kind::Import
                } else if before_t.ends_with("def") || before_t.ends_with("class") || before_t.ends_with("fn")
                    || before_t.ends_with("function") || before_t.ends_with("func") || before_t.ends_with("async def")
                {
                    Kind::Definition
                } else if alias_lhs(line_text, name).is_some() && !line_text[..at - ls].contains(name) && line_text.trim_end().ends_with(name) {
                    Kind::Alias
                } else if next == Some(b'=') && bytes.get(at + name.len()..).is_some_and(|r| {
                    let r = std::str::from_utf8(r).unwrap_or("").trim_start();
                    r.starts_with('=') && !r.starts_with("==")
                }) && alias_lhs(line_text, name).is_some()
                {
                    // the left-hand side of `name = Owner.name`: the RHS mention classifies the line
                    continue;
                } else if prev == Some(b'.') || (prev == Some(b':') && before_t.ends_with("::")) {
                    if next == Some(b'(') { Kind::MemberCall } else { Kind::ValueRef }
                } else if prev == Some(b'@') {
                    Kind::ValueRef
                } else if next == Some(b'(') {
                    Kind::Call
                } else {
                    Kind::ValueRef
                }
            }
        };
        let alias = match kind {
            Kind::Alias => alias_lhs(line_text, name),
            Kind::Import => import_alias(line_text, name),
            _ => None,
        };
        let _ = abap; // ABAP has no aliases by assignment and no imports
        let m = Mention { file: path.to_string(), line: line_no, kind, text: line_text.trim().chars().take(160).collect(), alias, via: None };
        match by_line.get(&line_no) {
            Some(old) if old.kind <= m.kind => {}
            _ => {
                by_line.insert(line_no, m);
            }
        }
    }
    by_line.into_values().collect()
}

/// Decorators written on (or just above) a definition's first line.
pub fn decorators(src: &str, start_line: usize) -> Vec<String> {
    let lines: Vec<&str> = src.lines().collect();
    if start_line == 0 || start_line > lines.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    // the entity span may or may not include its decorators
    let mut i = start_line - 1;
    while i < lines.len() && lines[i].trim_start().starts_with('@') {
        out.push(lines[i].trim().to_string());
        i += 1;
    }
    let mut j = start_line - 1;
    while j > 0 && lines[j - 1].trim_start().starts_with('@') {
        out.push(lines[j - 1].trim().to_string());
        j -= 1;
    }
    out
}

/// A decorator that routes calls through a registry, dispatcher, framework or
/// attribute access rather than a direct call by name.
pub fn routing_decorator(d: &str) -> Option<(&'static str, &'static str)> {
    let head = d.trim_start_matches('@');
    let head = head.split('(').next().unwrap_or(head);
    let last = head.rsplit('.').next().unwrap_or(head);
    match last {
        "dispatch" | "multimethod" | "multidispatch" => Some(("dispatch", "reached through the dispatcher, which picks a registration by argument types at runtime")),
        "register" | "register_lookup" | "register_function" | "register_type" => Some(("registered", "registered with a registry and invoked through it")),
        "singledispatch" | "singledispatchmethod" => Some(("dispatch", "single-dispatch generic: registrations are reached through it")),
        "overload" => Some(("dispatch", "typing overload: the runtime implementation is another definition")),
        "receiver" | "connect" | "listens_for" | "hookimpl" | "hookspec" => Some(("registered", "a signal or hook handler, invoked by the framework")),
        "route" | "get" | "post" | "put" | "delete" | "patch" | "websocket" | "api_view" | "action" => Some(("registered", "a route or view handler, invoked by the framework")),
        "property" | "cached_property" | "setter" | "getter" | "deleter" | "classproperty" => Some(("attribute", "a property: read as `obj.name`, not called")),
        "fixture" => Some(("registered", "a pytest fixture, injected by parameter name")),
        _ => None,
    }
}

/// `getattr(x, 'prefix' + …)`, `getattr(x, f'prefix{…}')`, `getattr(x, 'prefix%s' % …)`:
/// the literal prefixes a computed attribute name starts with.
pub fn dynamic_prefixes(path: &str, src: &str) -> Vec<(String, usize)> {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r#"(?:getattr|hasattr)\(\s*[^,()]+,\s*(?:[fFrR]{1,2})?['"]([A-Za-z_][A-Za-z0-9_]*)(?:\{|%s|['"]\s*(?:\+|%|\.format))"#).unwrap()
    });
    if !src.contains("getattr(") && !src.contains("hasattr(") {
        return Vec::new();
    }
    let _ = path;
    let mut out = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if !line.contains("attr(") {
            continue;
        }
        for c in re.captures_iter(line) {
            out.push((c[1].to_string(), i + 1));
        }
    }
    out
}

/// A cheap regex (case-insensitive) for the files that can hold a computed ABAP call, so a
/// verdict reads only those. `dynamic_sites` decides exactly. A `CALL FUNCTION` operand split
/// onto the next line is not seen by a line-based text search.
pub const ABAP_DYNAMIC_PREFILTER: &str = r"call\s+function\s+[^'`\s]|[-=]>\(|perform\s*\(|in\s+program\s*\(|type\s*\(|new\s*\(";

pub use sem_core::parser::calls::abap_dynamic::DynForm;

/// One ABAP call whose target is computed at run time, placed in a file.
#[derive(Debug, Clone)]
pub struct DynamicSite {
    /// The static class of the receiver, when the text names one: `zcl_x=>(lv)`, or `me->(lv)`
    /// inside `zcl_x`'s implementation. `None` for an untyped receiver (story 2.2 binds those).
    pub class: Option<String>,
    pub form: DynForm,
    /// The statically known name when only the program is computed (`PERFORM f IN PROGRAM (lv)`).
    pub name: Option<String>,
    pub file: String,
    pub line: usize,
    /// The source line, trimmed, for the reason.
    pub text: String,
}

/// Every computed call in one ABAP file. The detection is sem-core's
/// (`parser::calls::abap_dynamic`), the same one the call lowering of story 2.1 reports its
/// `Pick::Unknown` reasons from, so the verdict and `Stats.unresolved` cannot disagree.
pub fn dynamic_sites(path: &str, src: &str) -> Vec<DynamicSite> {
    if !is_abap(path) {
        return Vec::new();
    }
    sem_core::parser::calls::abap_dynamic::dynamic_sites(src)
        .into_iter()
        .map(|d| {
            let (ls, le) = line_bounds(src.as_bytes(), d.at);
            DynamicSite {
                class: d.class,
                form: d.form,
                name: d.name,
                file: path.to_string(),
                line: src.as_bytes()[..d.at].iter().filter(|&&b| b == b'\n').count() + 1,
                text: src[ls..le].trim().chars().take(120).collect(),
            }
        })
        .collect()
}

impl DynamicSite {
    /// Whether this site can reach `t`: the rule is the story's. A static class reaches the
    /// methods of that class; `CALL FUNCTION lv` the function modules; a computed form name
    /// the forms. An untyped receiver, and a computed class name, are counted, not attributed.
    fn reaches(&self, t: &Target<'_>) -> bool {
        match self.form {
            DynForm::Method => {
                // INTEGRATION 2.3: a method of a subclass of `class` is reached too.
                t.entity_type == "method"
                    && matches!((&self.class, t.owner), (Some(c), Some(o)) if c.eq_ignore_ascii_case(o))
            }
            DynForm::Function => t.entity_type == "function",
            DynForm::Form => t.entity_type == "form" && self.name.as_deref().is_none_or(|n| n.eq_ignore_ascii_case(t.name)),
            DynForm::Class => false,
        }
    }
}

/// The definition under assessment.
pub struct Target<'a> {
    pub name: &'a str,
    /// The entity's type as the parser names it (`method`, `function`, `form`, `class`, ...).
    pub entity_type: &'a str,
    pub file: &'a str,
    pub span: (usize, usize),
    pub decorators: Vec<String>,
    /// The owning class, for constructors (`__init__`, `__new__`).
    pub owner: Option<&'a str>,
}

/// An entity that encloses a line, for grouping possible callers.
pub struct Enclosing {
    pub display: String,
    pub entity_type: String,
    pub start_line: usize,
}

/// Build the verdict.
///
/// - `mentions`: every mention of the name in the corpus (`scan_file`);
/// - `alias_mentions`: mentions of alias names found one hop out, with `via` set;
/// - `resolved`: the resolved callers' spans `(file, start, end)` (and any
///   other span whose mentions are already explained);
/// - `dynamic`: `(prefix, file, line)` from `dynamic_prefixes` over the corpus;
/// - `abap_dyn`: every computed ABAP call in the corpus (`dynamic_sites`), empty for other languages;
/// - `not_checked`: a reason the scan could not run or was capped.
#[allow(clippy::too_many_arguments)]
pub fn assess(
    t: &Target<'_>,
    mentions: &[Mention],
    alias_mentions: &[Mention],
    resolved: &[(String, usize, usize)],
    dynamic: &[(String, String, usize)],
    abap_dyn: &[DynamicSite],
    files_scanned: usize,
    not_checked: Option<String>,
    enclosing: impl Fn(&str, usize) -> Option<Enclosing>,
) -> Verdict {
    let explained = |m: &Mention| {
        (m.file == t.file && m.line >= t.span.0 && m.line <= t.span.1)
            || resolved.iter().any(|(f, s, e)| *f == m.file && m.line >= *s && m.line <= *e)
    };
    let abap = is_abap(t.file);
    let mut reasons: Vec<Reason> = Vec::new();
    let mut unexplained: Vec<&Mention> = Vec::new();
    for m in mentions.iter().chain(alias_mentions) {
        if !m.kind.counts_as_caller() && m.kind != Kind::Import {
            continue;
        }
        if m.kind == Kind::Import {
            continue;
        }
        if explained(m) {
            continue;
        }
        unexplained.push(m);
    }
    // aliases first: they explain the bare calls that follow
    let mut aliases: BTreeSet<String> = BTreeSet::new();
    for m in mentions.iter().filter(|m| m.kind == Kind::Alias) {
        aliases.insert(format!("`{}` at {}:{}", m.text, m.file, m.line));
    }
    for m in mentions.iter().filter(|m| m.kind == Kind::Import && m.alias.is_some()) {
        aliases.insert(format!("`{}` at {}:{}", m.text, m.file, m.line));
    }
    if !aliases.is_empty() {
        let list: Vec<String> = aliases.iter().take(3).cloned().collect();
        reasons.push(Reason {
            code: "alias",
            detail: format!(
                "{} alias(es) of `{}` the resolver does not follow: {}{}",
                aliases.len(),
                t.name,
                list.join("; "),
                if aliases.len() > 3 { " …" } else { "" }
            ),
        });
    }
    let mut by_kind: BTreeMap<Kind, usize> = BTreeMap::new();
    for m in &unexplained {
        *by_kind.entry(m.kind).or_default() += 1;
    }
    for (k, n) in &by_kind {
        if *k == Kind::Alias {
            continue;
        }
        let code = match k {
            Kind::Call => "unresolved_calls",
            Kind::MemberCall => "untyped_member_calls",
            Kind::InterfaceCall => "interface_calls",
            Kind::StringKey => "string_keys",
            Kind::ValueRef => "value_refs",
            _ => "other",
        };
        let mut detail = format!("{n} {}", k.words(abap, t.entity_type == "function"));
        if abap && matches!(k, Kind::MemberCall | Kind::InterfaceCall) {
            // the same name defined in another class: the receiver's type decides which one a call reaches
            let homes: BTreeSet<String> = mentions
                .iter()
                .filter(|m| m.kind == Kind::Definition && !explained(m))
                .filter_map(|m| enclosing(&m.file, m.line))
                .map(|e| e.display.split('.').next().unwrap_or("").to_string())
                .filter(|h| !h.is_empty() && !t.owner.is_some_and(|o| o.eq_ignore_ascii_case(h)))
                .collect();
            if !homes.is_empty() {
                let list: Vec<String> = homes.iter().take(3).cloned().collect();
                detail += &format!("; `{}` is also defined in {}{}, so the receiver's type decides which one a call reaches", t.name, list.join(", "), if homes.len() > 3 { " …" } else { "" });
            }
        }
        reasons.push(Reason { code, detail });
    }
    // one reason per decorator head (`@dispatch` x 15 is one fact)
    let mut by_head: BTreeMap<(&'static str, String), (usize, &'static str, String)> = BTreeMap::new();
    for d in &t.decorators {
        if let Some((code, why)) = routing_decorator(d) {
            let head = d.split('(').next().unwrap_or(d).trim().to_string();
            let e = by_head.entry((code, head)).or_insert((0, why, d.split('#').next().unwrap_or(d).trim().to_string()));
            e.0 += 1;
        }
    }
    for ((code, head), (n, why, example)) in by_head {
        let detail = if n == 1 { format!("`{example}`: {why}") } else { format!("`{head}` on {n} registrations (e.g. `{example}`): {why}") };
        reasons.push(Reason { code, detail });
    }
    if let Some(owner) = t.owner {
        if matches!(t.name, "__init__" | "__new__" | "constructor") {
            reasons.push(Reason {
                code: "constructor",
                detail: format!("`{}` runs on every construction `{owner}(...)` (and of its subclasses): their call sites are `sem callers {owner}`", t.name),
            });
        }
    }
    let dyn_hits: Vec<&(String, String, usize)> = dynamic.iter().filter(|(p, _, _)| !p.is_empty() && t.name.starts_with(p.as_str()) && t.name != p).collect();
    if !dyn_hits.is_empty() {
        let (p, f, l) = dyn_hits[0];
        reasons.push(Reason {
            code: "dynamic_getattr",
            detail: format!(
                "computed attribute lookup `getattr(…, '{p}' + …)` at {f}:{l}{} can reach `{}` with no mention of its name",
                if dyn_hits.len() > 1 { format!(" (+{} more)", dyn_hits.len() - 1) } else { String::new() },
                t.name
            ),
        });
    }
    // ABAP computed calls: attributed by the story's rule, the rest counted in `checked`
    let abap_hits: Vec<&DynamicSite> = abap_dyn.iter().filter(|d| d.reaches(t)).collect();
    if abap && !abap_hits.is_empty() {
        let first = abap_hits[0];
        let why = match first.form {
            DynForm::Method => "its static class is this method's class",
            DynForm::Function => "a computed function module name can be any module",
            DynForm::Form => "a computed form name can be any form",
            DynForm::Class => "",
        };
        reasons.push(Reason {
            code: "dynamic_call",
            detail: format!(
                "{} dynamic call(s) (name computed at run time), first `{}` at {}:{}{}: {why}, so it can reach this definition without naming it",
                abap_hits.len(),
                first.text,
                first.file,
                first.line,
                if abap_hits.len() > 1 { format!(" (+{} more)", abap_hits.len() - 1) } else { String::new() },
            ),
        });
    }
    if let Some(why) = not_checked {
        reasons.push(Reason { code: "not_checked", detail: why });
    }

    // group unexplained mentions by enclosing entity
    let mut groups: BTreeMap<(String, usize, String), PossibleCaller> = BTreeMap::new();
    for m in &unexplained {
        let enc = enclosing(&m.file, m.line);
        let (display, ty, start) = match enc {
            Some(e) => (e.display, e.entity_type, e.start_line),
            None => ("<module level>".to_string(), "module".to_string(), 0),
        };
        let key = (m.file.clone(), start, display.clone());
        let g = groups.entry(key).or_insert_with(|| PossibleCaller {
            entity: display,
            entity_type: ty,
            file: m.file.clone(),
            start_line: start,
            sites: Vec::new(),
        });
        g.sites.push(Site { line: m.line, kind: m.kind, via: m.via.clone() });
    }
    let mut dyn_sites = 0;
    for d in abap_hits.iter().filter(|d| abap && d.form == DynForm::Method) {
        let (display, ty, start) = match enclosing(&d.file, d.line) {
            Some(e) => (e.display, e.entity_type, e.start_line),
            None => ("<module level>".to_string(), "module".to_string(), 0),
        };
        let g = groups.entry((d.file.clone(), start, display.clone())).or_insert_with(|| PossibleCaller {
            entity: display,
            entity_type: ty,
            file: d.file.clone(),
            start_line: start,
            sites: Vec::new(),
        });
        g.sites.push(Site { line: d.line, kind: Kind::Dynamic, via: None });
        dyn_sites += 1;
    }
    let mut possible: Vec<PossibleCaller> = groups.into_values().collect();
    // aliases and calls first (most actionable), nearest to the definition
    // first (same file, same directory, same top-level package), then by file
    let dir = |f: &str| f.rsplit_once('/').map(|(d, _)| d.to_string()).unwrap_or_default();
    let top = |f: &str| f.split('/').next().unwrap_or("").to_string();
    let distance = |f: &str| -> u8 {
        if f == t.file {
            0
        } else if dir(f) == dir(t.file) {
            1
        } else if top(f) == top(t.file) && f.matches('/').count() > 0 && dir(f).starts_with(&dir(&dir(t.file))) {
            2
        } else {
            3
        }
    };
    possible.sort_by_key(|p| (p.sites.iter().map(|s| s.kind).min(), distance(&p.file), p.file.clone(), p.start_line));
    let complete = reasons.is_empty();
    Verdict {
        complete,
        incomplete_because: reasons,
        checked: if abap {
            // computed calls of this definition's kind that the text cannot tie to it (an untyped
            // receiver, a computed class): counted, not attributed
            let (rel, what) = match t.entity_type {
                "method" => (Some(DynForm::Method), "computed method call(s) on an untyped receiver or computed class"),
                "class" => (Some(DynForm::Class), "computed class name(s)"),
                _ => (None, ""),
            };
            let n = rel.map_or(0, |r| abap_dyn.iter().filter(|d| d.form == r && !d.reaches(t)).count());
            let rest = if n == 0 { String::new() } else { format!("; {n} {what} are not attributed to any definition, so any of them might reach it") };
            format!(
                "every code mention of `{}` in {files_scanned} file(s), any case (comments and literals excluded), `->` `=>` `~` and CALL METHOD / CALL FUNCTION / PERFORM call shapes, computed calls (`CALL FUNCTION lv`, `zcl_x=>(lv)`, `PERFORM (lv)`) by their static class{rest}",
                t.name
            )
        } else {
            format!(
                "every code mention of `{}` in {files_scanned} file(s) (comments and docstrings excluded), aliases one hop, dispatch/registration decorators, getattr with a literal prefix; a name computed entirely at runtime is not modeled",
                t.name
            )
        },
        possible_caller_sites: unexplained.len() + dyn_sites,
        possible_callers: possible,
    }
}

/// The text rendering shared by `callers`, `impact` and `certify`.
pub fn render_text(v: &Verdict, cap: usize, indent: &str) -> String {
    let mut o = String::new();
    if v.complete {
        o += &format!("{indent}complete: no other textual path to this definition ({})\n", v.checked);
        return o;
    }
    o += &format!("{indent}INCOMPLETE: the resolved callers above are not the whole set:\n");
    for r in &v.incomplete_because {
        o += &format!("{indent}  - {}\n", r.detail);
    }
    if !v.possible_callers.is_empty() {
        o += &format!(
            "{indent}possible callers ({} site(s) in {} entit{}), unresolved by the static graph:\n",
            v.possible_caller_sites,
            v.possible_callers.len(),
            if v.possible_callers.len() == 1 { "y" } else { "ies" }
        );
        for p in v.possible_callers.iter().take(cap) {
            let sites: Vec<String> = p
                .sites
                .iter()
                .take(4)
                .map(|s| format!("L{} {}{}", s.line, kind_short(s.kind), s.via.as_ref().map(|a| format!(" via `{a}`")).unwrap_or_default()))
                .collect();
            o += &format!(
                "{indent}  {} {} {}:{} ({}{})\n",
                p.entity_type,
                p.entity,
                p.file,
                p.start_line,
                sites.join(", "),
                if p.sites.len() > 4 { format!(", +{}", p.sites.len() - 4) } else { String::new() }
            );
        }
        if v.possible_callers.len() > cap {
            o += &format!("{indent}  … {} more (raise --limit)\n", v.possible_callers.len() - cap);
        }
    }
    o
}

fn kind_short(k: Kind) -> &'static str {
    match k {
        Kind::Alias => "alias",
        Kind::Call => "call",
        Kind::MemberCall => "member call",
        Kind::Dynamic => "dynamic call",
        Kind::InterfaceCall => "interface call",
        Kind::StringKey => "string key",
        Kind::ValueRef => "value use",
        Kind::Import => "import",
        Kind::Definition => "definition",
    }
}

/// Follow aliases one hop: for every `x = …name` / `import name as x` with
/// `x != name`, the mentions of `x` (in the alias's file for an import, the
/// whole corpus for an assignment), tagged `via`.
pub fn alias_hop(mentions: &[Mention], name: &str, mut scan: impl FnMut(&str, Option<&str>) -> Vec<Mention>) -> Vec<Mention> {
    let mut out = Vec::new();
    let mut done = BTreeSet::new();
    for m in mentions {
        let Some(a) = m.alias.as_deref() else { continue };
        if a == name || !done.insert((a.to_string(), m.kind == Kind::Import, m.file.clone())) {
            continue;
        }
        // an alias is a binding in its file (a module global, a local, an
        // import): follow it there; importers re-bind and are found by name
        let scope = Some(m.file.as_str());
        for mut x in scan(a, scope) {
            if x.file == m.file && x.line == m.line {
                continue;
            }
            if matches!(x.kind, Kind::Definition | Kind::Import) {
                continue;
            }
            x.via = Some(a.to_string());
            out.push(x);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERMS: &str = r#"class Permutation(object):
    def __new__(cls, *args):
        return _af_new(list(args))

    @staticmethod
    def _af_new(perm):
        """Internally `_af_new` is used. _af_new(x) builds."""
        p = object.__new__(Permutation)
        return p

    def rmul(self):
        # _af_new is fast
        return _af_new([2])

_af_new = Permutation._af_new
REG = {'_af_new': Permutation._af_new}
"#;

    #[test]
    fn classifies_alias_calls_strings_and_skips_prose() {
        let ms = scan_file("p.py", PERMS, "_af_new");
        let got: Vec<(usize, Kind)> = ms.iter().map(|m| (m.line, m.kind)).collect();
        assert_eq!(
            got,
            vec![(3, Kind::Call), (6, Kind::Definition), (13, Kind::Call), (15, Kind::Alias), (16, Kind::StringKey)]
        );
        assert_eq!(ms[3].alias.as_deref(), Some("_af_new"));
    }

    #[test]
    fn verdict_lists_alias_callers_and_is_incomplete() {
        let ms = scan_file("p.py", PERMS, "_af_new");
        let t = Target { name: "_af_new", entity_type: "method", file: "p.py", span: (5, 9), decorators: vec!["@staticmethod".into()], owner: Some("Permutation") };
        let v = assess(&t, &ms, &[], &[], &[], &[], 1, None, |_, l| {
            Some(Enclosing { display: format!("line{l}"), entity_type: "function".into(), start_line: l })
        });
        assert!(!v.complete);
        assert!(v.incomplete_because.iter().any(|r| r.code == "alias"));
        assert!(v.incomplete_because.iter().any(|r| r.code == "unresolved_calls"));
        assert_eq!(v.possible_caller_sites, 4);
    }

    #[test]
    fn resolved_callers_explain_their_mentions() {
        let src = "def target():\n    return 1\n\ndef a():\n    return target()\n";
        let ms = scan_file("m.py", src, "target");
        let t = Target { name: "target", entity_type: "function", file: "m.py", span: (1, 2), decorators: vec![], owner: None };
        let v = assess(&t, &ms, &[], &[("m.py".into(), 4, 5)], &[], &[], 1, None, |_, _| None);
        assert!(v.complete, "{:?}", v.incomplete_because);
    }

    #[test]
    fn dynamic_getattr_prefix() {
        let src = "def rewrite(self, n):\n    return getattr(self, '_eval_rewrite_as_' + n)()\n";
        let d = dynamic_prefixes("b.py", src);
        assert_eq!(d, vec![("_eval_rewrite_as_".to_string(), 2)]);
        let src = "x = getattr(obj, f'_eval_is_{name}', None)\n";
        assert_eq!(dynamic_prefixes("b.py", src)[0].0, "_eval_is_");
    }

    #[test]
    fn decorators_above_or_on_the_span() {
        let src = "@dispatch(Set, Set)\ndef is_subset_sets(a, b):\n    pass\n";
        assert_eq!(decorators(src, 2), vec!["@dispatch(Set, Set)".to_string()]);
        assert_eq!(decorators(src, 1), vec!["@dispatch(Set, Set)".to_string()]);
        assert!(routing_decorator("@dispatch(Set, Set)").is_some());
        assert!(routing_decorator("@staticmethod").is_none());
        assert!(routing_decorator("@is_subset_sets.register(A, B)").is_some());
    }

    #[test]
    fn js_comments_and_template_strings() {
        let src = "// foo() here\nconst x = `foo`;\n/* foo */ foo();\nobj.foo(1);\n";
        let ms = scan_file("a.ts", src, "foo");
        let got: Vec<(usize, Kind)> = ms.iter().map(|m| (m.line, m.kind)).collect();
        assert_eq!(got, vec![(2, Kind::StringKey), (3, Kind::Call), (4, Kind::MemberCall)]);
    }

    #[test]
    fn import_alias_hop() {
        let src = "from p import _af_new as mk\n\ndef g():\n    return mk([1])\n";
        let ms = scan_file("g.py", src, "_af_new");
        assert_eq!(ms[0].kind, Kind::Import);
        assert_eq!(ms[0].alias.as_deref(), Some("mk"));
        let hop = alias_hop(&ms, "_af_new", |a, scope| {
            assert_eq!(scope, Some("g.py"));
            scan_file("g.py", src, a)
        });
        assert_eq!(hop.len(), 1);
        assert_eq!(hop[0].line, 4);
        assert_eq!(hop[0].via.as_deref(), Some("mk"));
    }

    const ABAP: &str = "CLASS zcl_a IMPLEMENTATION.
  METHOD run.
    lo->Describe( ).
    ZCL_X=>describe( ).
    zif_x~describe( ).
    CALL METHOD lo->describe.
    CALL METHOD zif_x~describe.
    CALL METHOD describe.
    PERFORM describe USING 1.
    describe( ).
    lv = 'DESCRIBE'.
    \" describe in a comment
* describe in a column-1 comment
    lv = 'a describe b'.
    lv = |describe { lv } describe|.
    lv = lo->describe.
    lv = lo->other( describe ).
  ENDMETHOD.
  METHODS describe REDEFINITION.
ENDCLASS.
";

    fn abap_kinds(src: &str, name: &str) -> Vec<(usize, Kind)> {
        scan_file("zcl_a.clas.abap", src, name).iter().map(|m| (m.line, m.kind)).collect()
    }

    #[test]
    fn abap_call_shapes_fold_case_and_skip_comments_and_literals() {
        assert_eq!(
            abap_kinds(ABAP, "describe"),
            vec![
                (3, Kind::MemberCall),    // lo->Describe( ), any case
                (4, Kind::MemberCall),    // ZCL_X=>describe( ), not a bare call
                (5, Kind::InterfaceCall), // zif_x~describe( )
                (6, Kind::MemberCall),    // CALL METHOD lo->describe
                (7, Kind::InterfaceCall), // CALL METHOD zif_x~describe
                (8, Kind::Call),          // CALL METHOD describe
                (9, Kind::Call),          // PERFORM describe
                (10, Kind::Call),         // describe( )
                (11, Kind::StringKey),    // 'DESCRIBE'
                (16, Kind::ValueRef),     // lo->describe is an attribute, not a call
                (17, Kind::ValueRef),     // passed as a value
                (19, Kind::Definition),   // METHODS describe REDEFINITION
            ]
        );
    }

    #[test]
    fn abap_name_in_comment_or_literal_is_not_a_mention() {
        let src = "* zfx_fm here\nlv = 'call zfx_fm now'. \" zfx_fm\nlv = |zfx_fm { lv }|.\n";
        assert!(scan_file("a.prog.abap", src, "zfx_fm").is_empty());
        // the name inside a template expression is code
        let src = "lv = |text { zfx_fm( ) }|.\n";
        assert_eq!(scan_file("a.prog.abap", src, "ZFX_FM")[0].kind, Kind::Call);
    }

    #[test]
    fn abap_string_key_any_case_and_static_function_call() {
        let src = "CALL FUNCTION 'ZFX_FM'\n  EXPORTING a = 1.\nlv_fm = 'zfx_fm'.\nlv_x = 'Zfx_Fm'.\n";
        let got: Vec<(usize, Kind)> = scan_file("a.prog.abap", src, "zfx_fm").iter().map(|m| (m.line, m.kind)).collect();
        assert_eq!(got, vec![(1, Kind::Call), (3, Kind::StringKey), (4, Kind::StringKey)]);
    }

    #[test]
    fn abap_chained_declarations_are_definitions() {
        let src = "METHODS: a,\n  describe,\n  b RETURNING VALUE(rv) TYPE describe.\n";
        let got: Vec<(usize, Kind)> = scan_file("a.clas.abap", src, "describe").iter().map(|m| (m.line, m.kind)).collect();
        assert_eq!(got, vec![(2, Kind::Definition), (3, Kind::ValueRef)]);
    }

    #[test]
    fn other_languages_still_match_case_exactly() {
        assert!(scan_file("a.py", "Foo()\n", "foo").is_empty());
    }
}
