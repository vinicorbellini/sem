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

/// Whether `file_path` is the `TOP` include of a function group or a program:
/// `<group>.fugr.l<group>top.abap`, or `<name>top.prog.abap` (`zfoo_top`).
/// Its top-level `DATA` is the global data of the compiled unit, so it is the
/// one place an ABAP `DATA` outside a class becomes an entity.
pub fn is_abap_top_include(file_path: &str) -> bool {
    let Some(object) = parse_abapgit_name(file_path) else {
        return false;
    };
    let name = object.name.to_ascii_lowercase();
    match (object.object_type.as_str(), object.part.as_deref()) {
        ("fugr", Some(part)) => {
            part.replace('#', "/").to_ascii_lowercase() == format!("l{name}top")
        }
        ("prog", None) => name.ends_with("top"),
        _ => false,
    }
}

/// The abapGit file names that can satisfy `INCLUDE <include>.` written in a
/// file of `from`, lower-cased: the program include `<include>.prog.abap`, and
/// for a function group's own file the group's include
/// `<group>.fugr.<include>.abap`. A `/ns/` name is written `#ns#`.
pub fn include_file_names(include: &str, from: &AbapObject) -> Vec<String> {
    let escape = |name: &str| name.to_ascii_lowercase().replace('/', "#");
    let include_key = escape(include);
    let mut names = vec![format!("{include_key}.prog.abap")];
    if from.object_type == "fugr" {
        names.push(format!("{}.fugr.{include_key}.abap", escape(&from.name)));
    }
    names
}

/// ABAP keywords that are also names somewhere: `CREATE PUBLIC` in a class
/// definition does not read a `create`, nor `TYPE TABLE OF` a `table`, so
/// the calls pipeline's reference sites skip them (`calls/abap.rs`). Sorted,
/// folded.
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
    fn abap_top_include_by_name() {
        assert!(is_abap_top_include("src/zfx_fg.fugr.lzfx_fgtop.abap"));
        assert!(is_abap_top_include("src/ZFX_FG.FUGR.LZFX_FGTOP.abap"));
        assert!(is_abap_top_include("src/#ns#fg.fugr.l#ns#fgtop.abap"));
        assert!(is_abap_top_include("src/zfoo_top.prog.abap"));
        assert!(!is_abap_top_include("src/zfx_fg.fugr.lzfx_fgf01.abap"));
        assert!(!is_abap_top_include("src/zfx_fg.fugr.saplzfx_fg.abap"));
        assert!(!is_abap_top_include("src/zfx_fg.fugr.lother_top.abap"));
        assert!(!is_abap_top_include("src/zcl_top.clas.abap"));
        assert!(!is_abap_top_include("src/foo.abap"));
    }

    #[test]
    fn abap_include_file_names() {
        let group = parse_abapgit_name("src/zfx_fg.fugr.saplzfx_fg.abap").unwrap();
        assert_eq!(
            include_file_names("LZFX_FGF01", &group),
            ["lzfx_fgf01.prog.abap", "zfx_fg.fugr.lzfx_fgf01.abap"]
        );
        let prog = parse_abapgit_name("src/zfx_report.prog.abap").unwrap();
        assert_eq!(
            include_file_names("zfx_report_f01", &prog),
            ["zfx_report_f01.prog.abap"]
        );
        assert_eq!(
            include_file_names("/ns/zinc", &prog),
            ["#ns#zinc.prog.abap"]
        );
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
