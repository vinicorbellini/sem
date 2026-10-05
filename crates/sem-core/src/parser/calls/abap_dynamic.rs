//! ABAP calls whose target is computed at run time.
//!
//! `CALL FUNCTION lv_fm`, `CALL METHOD zcl_x=>(lv_meth)`, `lo->(lv)`, `PERFORM (lv_form)`,
//! `PERFORM f IN PROGRAM (lv)`, `CREATE OBJECT lo TYPE (lv_cls)` and `NEW (lv)( )` name nothing
//! the resolver can bind. They are found by shape in the comment-and-literal-stripped text (the
//! one ABAP stripper, [`strip_abap_content`]), so a name in a comment or a literal is not a
//! site, and the literal after `CALL FUNCTION` is read from the source at the same offsets (a
//! quote there means the module is named statically).
//!
//! Two consumers share this one detector so they cannot disagree: the call lowering reports
//! each site as `Pick::Unknown` with [`DynForm::reason`], counted in `Stats.unresolved`, and the
//! CLI's completeness verdict (`commands::completeness`) lists them as possible references.
//! INTEGRATION 2.1: `calls/abap.rs`'s lowering calls [`dynamic_sites`] on the file's source and
//! emits one `Site` per result, with an expression that resolves to `Pick::Unknown(form.reason())`
//! and no edge.

use std::sync::OnceLock;

use crate::parser::graph::strip_abap_content;

/// What a computed call names at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynForm {
    /// `CALL METHOD zcl_x=>(lv)`, `lo->(lv)`, `(lv_class)=>m( )`.
    Method,
    /// `CALL FUNCTION lv_fm`.
    Function,
    /// `PERFORM (lv_form)`, `PERFORM f IN PROGRAM (lv)`.
    Form,
    /// `CREATE OBJECT lo TYPE (lv)`, `NEW (lv)( )`.
    Class,
}

impl DynForm {
    /// The reason an unresolved site carries (a `Pick::Unknown` reason is `&'static str`).
    pub fn reason(self) -> &'static str {
        match self {
            DynForm::Method => "dynamic method name",
            DynForm::Function => "dynamic function name",
            DynForm::Form => "dynamic form name",
            DynForm::Class => "dynamic class name",
        }
    }
}

/// One computed call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynSite {
    /// Byte offset of the statement's keyword (or the receiver, for `x->(lv)`) in the source.
    pub at: usize,
    pub form: DynForm,
    /// The static class of the receiver when the text names one: `zcl_x=>(lv)`, or `me->(lv)`
    /// inside `zcl_x`'s implementation. `None` for an untyped receiver (story 2.2 binds those).
    pub class: Option<String>,
    /// The statically known name when only the program is computed (`PERFORM f IN PROGRAM (lv)`).
    pub name: Option<String>,
}

/// Every computed call in one ABAP source, in source order.
pub fn dynamic_sites(src: &str) -> Vec<DynSite> {
    static RES: OnceLock<[regex::Regex; 6]> = OnceLock::new();
    static CLS: OnceLock<regex::Regex> = OnceLock::new();
    let re = RES.get_or_init(|| {
        [
            regex::Regex::new(r"\bcall\s+function\b").unwrap(),
            regex::Regex::new(r"(\(?[a-z0-9_/<>\-]*\)?|\))\s*(->|=>)\(").unwrap(),
            regex::Regex::new(r"\bperform\s*\(").unwrap(),
            regex::Regex::new(r"\bperform\s+([a-z0-9_/<>\-]+)\s+in\s+program\s*\(").unwrap(),
            regex::Regex::new(r"\bcreate\s+object\s+[^\s.]+\s+type\s*\(").unwrap(),
            regex::Regex::new(r"\bnew\s*\(").unwrap(),
        ]
    });
    let cls = CLS.get_or_init(|| regex::Regex::new(r"\bclass\s+([a-z0-9_/]+)\s+implementation\b").unwrap());
    let code = strip_abap_content(src).to_ascii_lowercase();
    let hay = code.as_str();
    // the class whose implementation a position is in
    let classes: Vec<(usize, String)> = cls.captures_iter(hay).map(|c| (c.get(0).unwrap().start(), c[1].to_string())).collect();
    let class_at = |at: usize| classes.iter().rev().find(|(s, _)| *s < at).map(|(_, n)| n.clone());
    let mut out = Vec::new();
    for m in re[0].find_iter(hay) {
        // `CALL FUNCTION 'NAME'` is static: the literal is blanked in `hay`, so read the source
        let next = src.as_bytes()[m.end()..].iter().find(|b| !b.is_ascii_whitespace()).copied();
        if matches!(next, Some(b'\'' | b'`' | b'.') | None) {
            continue;
        }
        out.push(DynSite { at: m.start(), form: DynForm::Function, class: None, name: None });
    }
    for c in re[1].captures_iter(hay) {
        let recv = c[1].trim_matches(|ch| ch == '(' || ch == ')');
        let computed_recv = c[1].starts_with('(') || &c[1] == ")";
        let at = c.get(0).unwrap().start();
        let class = match (&c[2], computed_recv) {
            ("=>", false) if !recv.is_empty() => Some(recv.to_string()),
            ("->", false) if recv == "me" => class_at(at),
            // INTEGRATION 2.2: `lo->(lv)` with `lo` bound to a class gives that class here
            _ => None,
        };
        out.push(DynSite { at, form: DynForm::Method, class, name: None });
    }
    for m in re[2].find_iter(hay) {
        out.push(DynSite { at: m.start(), form: DynForm::Form, class: None, name: None });
    }
    for c in re[3].captures_iter(hay) {
        out.push(DynSite { at: c.get(0).unwrap().start(), form: DynForm::Form, class: None, name: Some(c[1].to_string()) });
    }
    for m in re[4].find_iter(hay).chain(re[5].find_iter(hay)) {
        out.push(DynSite { at: m.start(), form: DynForm::Class, class: None, name: None });
    }
    out.sort_by_key(|s| s.at);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DYNAMIC: &str = "CLASS zcl_fx_order IMPLEMENTATION.
  METHOD run.
    CALL METHOD me->(lv_m).
    lo->(lv_m).
  ENDMETHOD.
ENDCLASS.
START-OF-SELECTION.
  CALL FUNCTION lv_fm.
  CALL FUNCTION 'ZFX_FM'.
  CALL METHOD zcl_fx_order=>(lv_meth).
  CALL METHOD (lv_c)=>(lv_meth).
  CALL METHOD lo_any->(lv_meth).
  PERFORM (lv_form) IN PROGRAM zfx_report.
  PERFORM show_order IN PROGRAM (lv_prog).
  CREATE OBJECT lo TYPE (lv_cls).
  lo = NEW (lv_cls)( ).
  \" CALL FUNCTION lv_commented.
  lv = 'CALL FUNCTION lv_text'.
";

    #[test]
    fn abap_dynamic_sites_one_per_form_with_static_class() {
        let line = |at: usize| DYNAMIC.as_bytes()[..at].iter().filter(|&&b| b == b'\n').count() + 1;
        let sites = dynamic_sites(DYNAMIC);
        let got: Vec<(usize, DynForm, Option<&str>, Option<&str>)> =
            sites.iter().map(|s| (line(s.at), s.form, s.class.as_deref(), s.name.as_deref())).collect();
        assert_eq!(
            got,
            vec![
                (3, DynForm::Method, Some("zcl_fx_order"), None), // me-> inside the implementation
                (4, DynForm::Method, None, None),                 // lo-> is untyped until story 2.2
                (8, DynForm::Function, None, None),
                (10, DynForm::Method, Some("zcl_fx_order"), None),
                (11, DynForm::Method, None, None),
                (12, DynForm::Method, None, None),
                (13, DynForm::Form, None, None),
                (14, DynForm::Form, None, Some("show_order")),
                (15, DynForm::Class, None, None),
                (16, DynForm::Class, None, None),
            ]
        );
    }

    #[test]
    fn each_form_has_its_own_reason() {
        let reasons: Vec<&str> = dynamic_sites(DYNAMIC).iter().map(|s| s.form.reason()).collect();
        for want in ["dynamic method name", "dynamic function name", "dynamic form name", "dynamic class name"] {
            assert!(reasons.contains(&want), "{want}: {reasons:?}");
        }
    }
}
