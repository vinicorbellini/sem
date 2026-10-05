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
    fn abap_name_rejects_non_abapgit_names() {
        assert_eq!(parse_abapgit_name("src/foo.abap"), None);
        assert_eq!(parse_abapgit_name("src/foo.bar.abap"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas.xml"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas"), None);
        assert_eq!(parse_abapgit_name("src/.clas.abap"), None);
    }
}
