/// An ABAP object named by its abapGit file name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbapObject {
    /// Object name as abapGit wrote it, with `#ns#` turned back into `/ns/`.
    /// Case is kept; comparison folds it.
    pub name: String,
    /// Four-letter object type, lower-cased (`clas`, `prog`, `fugr`, `intf`).
    pub object_type: String,
    /// Include or local-part name between the type and the extension:
    /// `locals_imp`, `testclasses`, `lzfg_f01`.
    pub part: Option<String>,
}

/// Reads the object name, type and part from an abapGit source file name such as
/// `src/zcl_foo.clas.testclasses.abap`. Returns `None` for any other `.abap` name.
pub fn parse_abapgit_name(path: &str) -> Option<AbapObject> {
    let normalized = path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next()?;

    let segments: Vec<&str> = file_name.split('.').collect();
    if segments.len() < 3 || !segments[segments.len() - 1].eq_ignore_ascii_case("abap") {
        return None;
    }

    let object_type = segments[1];
    if object_type.len() != 4 || !object_type.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }

    let name = unescape_namespace(segments[0]);
    if name.is_empty() {
        return None;
    }

    let parts = &segments[2..segments.len() - 1];
    let part = if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    };

    Some(AbapObject {
        name,
        object_type: object_type.to_ascii_lowercase(),
        part,
    })
}

/// How far an ABAP definition reaches by name alone, before receivers have
/// types. Global names are unique per system; local ones live in their object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The whole repo: a global class or interface, a function module, a report.
    Repo,
    /// Its own object, and the whole repo when no other definition repo-wide
    /// shares the name: a method or attribute of a global class or interface.
    Unique,
    /// Its own object only: a local class and its members, a form, a module, a
    /// macro. A file with no abapGit name is an object of its own.
    Object,
}

/// The [`Scope`] of an entity of type `entity_type` defined in `file_path`.
///
/// | Candidate | Scope |
/// |-----------|-------|
/// | global class or interface (`zcl_x.clas.abap`, `zif_x.intf.abap`) | `Repo` |
/// | function module (`<group>.fugr.<fm>.abap`), report | `Repo` |
/// | anything else in a global class or interface's main file | `Unique` |
/// | anything in a local part (`locals_def`, `locals_imp`, `testclasses`) | `Object` |
/// | form, module, macro, and anything else | `Object` |
///
/// The file name decides the part, so a local class can never reach past its
/// object, whatever it is named: were `CLASS zcl_x DEFINITION LOCAL FRIENDS ...`
/// in `zcl_x.clas.testclasses.abap` an entity, it would stay `Object`.
pub fn abap_global_scope(file_path: &str, entity_type: &str) -> Scope {
    let Some(object) = parse_abapgit_name(file_path) else {
        return Scope::Object;
    };
    let part = object.part.as_deref();
    match (object.object_type.as_str(), part, entity_type) {
        ("clas", None, "class") | ("intf", None, "interface") => Scope::Repo,
        ("clas" | "intf", None, _) => Scope::Unique,
        ("fugr", Some(_), "function") | ("prog", _, "report") => Scope::Repo,
        _ => Scope::Object,
    }
}

/// ABAP keywords that are also method names somewhere: `CREATE PUBLIC` in a
/// class definition is not a call of a method `create`. Sorted, folded.
const KEYWORDS: &[&str] = &[
    "abap", "abstract", "accepting", "add", "adjacent", "aliases", "all", "and", "any",
    "append", "appending", "as", "ascending", "assert", "assign", "assigned", "assigning",
    "at", "authority", "begin", "between", "binary", "bound", "break", "by", "call", "casting",
    "catch", "changing", "check", "class", "cleanup", "clear", "close", "collect", "commit",
    "compute", "concatenate", "condense", "constants", "continue", "convert", "corresponding",
    "create", "data", "default", "deferred", "define", "definition", "delete", "descending",
    "distinct", "divide", "do", "else", "elseif", "end", "endat", "endcase", "endclass",
    "enddo", "endform", "endfunction", "endif", "endinterface", "endloop", "endmethod",
    "endmodule", "endselect", "endtry", "endwhile", "event", "events", "exceptions", "exit",
    "exiting", "export", "exporting", "fetch", "field", "final", "find", "for", "form", "free",
    "friends", "from", "function", "generate", "get", "handler", "hashed", "if",
    "implementation", "import", "importing", "in", "include", "index", "inheriting", "initial",
    "initialization", "insert", "instance", "interface", "interfaces", "into", "is", "join",
    "key", "leave", "like", "line", "lines", "load", "local", "loop", "message", "method",
    "methods", "modify", "module", "move", "multiply", "new", "not", "of", "on", "optional",
    "or", "others", "overlay", "pack", "parameters", "perform", "private", "program",
    "protected", "public", "raise", "raising", "ranges", "read", "receiving", "redefinition",
    "reduce", "ref", "refresh", "replace", "report", "return", "returning", "rollback",
    "section", "select", "set", "shift", "single", "skip", "sort", "sorted", "split",
    "standard", "start", "statics", "submit", "subtract", "sum", "table", "tables", "testing",
    "to", "try", "type", "types", "unassign", "unique", "unpack", "up", "update", "using",
    "value", "when", "where", "while", "with", "write",
];

/// Whether `word`, folded, is an ABAP keyword (see [`KEYWORDS`]).
pub fn is_abap_keyword(word: &str) -> bool {
    KEYWORDS.binary_search(&word).is_ok()
}

/// abapGit writes the `/` of a namespaced name as `#`: `#ns#zcl_foo` is `/ns/zcl_foo`.
fn unescape_namespace(name: &str) -> String {
    if let Some(rest) = name.strip_prefix('#') {
        if let Some(end) = rest.find('#') {
            return format!("/{}/{}", &rest[..end], &rest[end + 1..]);
        }
    }
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(path: &str) -> (String, String, Option<String>) {
        let object = parse_abapgit_name(path).unwrap_or_else(|| panic!("no object for {path}"));
        (object.name, object.object_type, object.part)
    }

    #[test]
    fn abap_name_object_and_type() {
        assert_eq!(
            parsed("src/zcl_foo.clas.abap"),
            ("zcl_foo".into(), "clas".into(), None)
        );
        assert_eq!(
            parsed("src/zfoo.prog.abap"),
            ("zfoo".into(), "prog".into(), None)
        );
        assert_eq!(
            parsed("src/zif_x.intf.abap"),
            ("zif_x".into(), "intf".into(), None)
        );
    }

    #[test]
    fn abap_name_part() {
        assert_eq!(
            parsed("src/zfg.fugr.lzfg_f01.abap"),
            ("zfg".into(), "fugr".into(), Some("lzfg_f01".into()))
        );
        assert_eq!(
            parsed("src/zcl_foo.clas.locals_imp.abap"),
            ("zcl_foo".into(), "clas".into(), Some("locals_imp".into()))
        );
        assert_eq!(
            parsed("src/zcl_foo.clas.testclasses.abap"),
            ("zcl_foo".into(), "clas".into(), Some("testclasses".into()))
        );
    }

    #[test]
    fn abap_name_namespace() {
        assert_eq!(
            parsed("src/#ns#zcl_foo.clas.abap"),
            ("/ns/zcl_foo".into(), "clas".into(), None)
        );
        assert_eq!(
            parsed("src/#ns#zcl_foo.clas.testclasses.abap"),
            (
                "/ns/zcl_foo".into(),
                "clas".into(),
                Some("testclasses".into())
            )
        );
    }

    #[test]
    fn abap_name_keeps_name_case_and_lowers_type() {
        assert_eq!(
            parsed("src/ZCL_Foo.CLAS.abap"),
            ("ZCL_Foo".into(), "clas".into(), None)
        );
    }

    #[test]
    fn abap_name_ignores_directories() {
        assert_eq!(
            parsed("src\\zdemo\\zcl_foo.clas.abap"),
            ("zcl_foo".into(), "clas".into(), None)
        );
    }

    #[test]
    fn abap_global_scope_by_part_and_type() {
        let cases = [
            ("src/zcl_foo.clas.abap", "class", Scope::Repo),
            ("src/zif_foo.intf.abap", "interface", Scope::Repo),
            ("src/zfg.fugr.z_fm.abap", "function", Scope::Repo),
            ("src/zfoo.prog.abap", "report", Scope::Repo),
            ("src/zcl_foo.clas.abap", "method", Scope::Unique),
            ("src/zcl_foo.clas.abap", "variable", Scope::Unique),
            ("src/zif_foo.intf.abap", "type", Scope::Unique),
            ("src/zcl_foo.clas.locals_imp.abap", "class", Scope::Object),
            ("src/zcl_foo.clas.locals_imp.abap", "method", Scope::Object),
            // `CLASS zcl_foo DEFINITION LOCAL FRIENDS ltc_x.` names the global class.
            ("src/zcl_foo.clas.testclasses.abap", "class", Scope::Object),
            ("src/zfg.fugr.lzfgf01.abap", "form", Scope::Object),
            ("src/zfoo.prog.abap", "form", Scope::Object),
            ("src/zfoo.prog.abap", "macro", Scope::Object),
            ("src/foo.abap", "class", Scope::Object),
        ];
        for (path, entity_type, scope) in cases {
            assert_eq!(
                abap_global_scope(path, entity_type),
                scope,
                "{path} {entity_type}"
            );
        }
    }

    #[test]
    fn abap_keywords_sorted_and_folded() {
        assert!(KEYWORDS.windows(2).all(|w| w[0] < w[1]));
        assert!(KEYWORDS.iter().all(|k| *k == k.to_ascii_lowercase()));
        assert!(is_abap_keyword("create") && is_abap_keyword("public"));
        assert!(!is_abap_keyword("describe") && !is_abap_keyword("CREATE"));
    }

    #[test]
    fn abap_name_rejects_non_abapgit_names() {
        assert_eq!(parse_abapgit_name("src/foo.abap"), None);
        assert_eq!(parse_abapgit_name("src/foo.bar.abap"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas.xml"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas"), None);
        assert_eq!(parse_abapgit_name("src/.clas.abap"), None);
    }
}
