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
    fn abap_name_rejects_non_abapgit_names() {
        assert_eq!(parse_abapgit_name("src/foo.abap"), None);
        assert_eq!(parse_abapgit_name("src/foo.bar.abap"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas.xml"), None);
        assert_eq!(parse_abapgit_name("src/zcl_foo.clas"), None);
        assert_eq!(parse_abapgit_name("src/.clas.abap"), None);
    }
}
