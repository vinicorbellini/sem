//! The completeness verdict on ABAP (story 2.5): the "possible callers" of a name come from a
//! source scan, and for `.abap` that scan strips comments and literals, folds case, and reads
//! `->`, `=>`, `~`, `CALL METHOD`, `CALL FUNCTION` and `PERFORM` as call shapes. Computed calls
//! (`CALL FUNCTION lv`, `zcl_x=>(lv)`) are reported with a reason, never dropped.
//!
//! The fixture is the ABAP fixture repository. The resolved callers come from story 2.1's call
//! lowering, and the per-reason counts of `Stats.unresolved` are in `sem graph --json`.

use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const FILES: &[&str] = &[
    "zcl_fx_order.clas.abap",
    "zcl_fx_order.clas.xml",
    "zcl_fx_order_sub.clas.abap",
    "zcl_fx_order_sub.clas.xml",
    "zcl_fx_user.clas.abap",
    "zcl_fx_user.clas.xml",
    "zif_fx_order.intf.abap",
    "zif_fx_order.intf.xml",
    "zfx_report.prog.abap",
    "zfx_report.prog.xml",
    "zfx_report_f01.prog.abap",
    "zfx_report_f01.prog.xml",
    "zfx_dynamic.prog.abap",
    "zfx_dynamic.prog.xml",
    "zfx_fg.fugr.zfx_fm.abap",
    "zfx_fg.fugr.lzfx_fgf01.abap",
    "zfx_fg.fugr.lzfx_fgf01.xml",
];

fn fixture_repo() -> TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sem-core/tests/fixtures/abap");
    let repo = TempDir::new().expect("tempdir");
    for f in FILES {
        fs::copy(src.join(f), repo.path().join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    repo
}

fn find_callers(repo: &TempDir, name: &str, file: Option<&str>) -> Value {
    let mut args = vec!["find", name, "--callers", "--json"];
    if let Some(f) = file {
        args.extend(["--file", f]);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_sem"))
        .current_dir(repo.path())
        .env("DO_NOT_TRACK", "1")
        .env("SEM_LOCAL", "1")
        .env("SEM_NO_INDEX", "1")
        .args(&args)
        .output()
        .expect("run sem");
    assert!(output.status.success(), "{name}: {}", String::from_utf8_lossy(&output.stderr));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("json");
    rows[0].clone()
}

/// (entity, line, kind) of every possible-caller site in `file`.
fn sites(row: &Value, file: &str) -> Vec<(String, u64, String)> {
    let mut out = Vec::new();
    for p in row["possible_callers"].as_array().unwrap() {
        if p["file"] != file {
            continue;
        }
        for s in p["sites"].as_array().unwrap() {
            out.push((p["entity"].as_str().unwrap().to_string(), s["line"].as_u64().unwrap(), s["kind"].as_str().unwrap().to_string()));
        }
    }
    out.sort();
    out
}

fn codes(row: &Value) -> Vec<String> {
    row["incomplete_because"].as_array().unwrap().iter().map(|r| r["code"].as_str().unwrap().to_string()).collect()
}

#[test]
fn abap_fixture_2_5_string_key_any_case() {
    let repo = fixture_repo();
    let row = find_callers(&repo, "ZFX_FM", None);
    // `lv_fm ... VALUE 'ZFX_FM'` names the module in upper case; the lookup used lower case
    let dynamic = sites(&row, "zfx_dynamic.prog.abap");
    assert!(dynamic.iter().any(|(_, line, kind)| *line == 4 && kind == "string_key"), "{dynamic:?}");
    assert!(codes(&row).contains(&"dynamic_call".to_string()), "{:?}", row["incomplete_because"]);
    // `zfx_report` is a resolved caller (`CALL FUNCTION 'ZFX_FM'`), by the calls pipeline
    let report_is_resolved = row["related"].as_array().unwrap().iter().any(|e| e["file"] == "zfx_report.prog.abap");
    assert!(report_is_resolved, "{:?}", row["related"]);
}

#[test]
fn abap_fixture_2_5_dynamic_function_visible() {
    let repo = fixture_repo();
    let row = find_callers(&repo, "zfx_fm", None);
    let why = row["incomplete_because"].as_array().unwrap().iter().find(|r| r["code"] == "dynamic_call").expect("dynamic_call reason");
    let detail = why["detail"].as_str().unwrap();
    assert!(detail.contains("CALL FUNCTION lv_fm") && detail.contains("zfx_dynamic.prog.abap:14"), "{detail}");
}

#[test]
fn abap_fixture_2_5_dynamic_method_static_class() {
    let repo = fixture_repo();
    let row = find_callers(&repo, "describe", Some("zcl_fx_order.clas.abap"));
    let why = row["incomplete_because"].as_array().unwrap().iter().find(|r| r["code"] == "dynamic_call").expect("dynamic_call reason");
    assert!(why["detail"].as_str().unwrap().contains("CALL METHOD zcl_fx_order=>(lv_meth)"), "{why}");
    // the site is listed, as a computed call
    assert!(sites(&row, "zfx_dynamic.prog.abap").contains(&("<module level>".to_string(), 15, "dynamic".to_string())));
    // the computed calls with an unknown receiver are counted, not attributed
    let checked = row["checked"].as_str().unwrap();
    assert!(checked.contains("might reach it"), "{checked}");
}

#[test]
fn abap_fixture_2_5_quiet_without_a_dynamic_form() {
    let repo = fixture_repo();
    // a form is reachable by `PERFORM (lv_form)`, so it has the reason; a class and an
    // interface are reached by no computed call the scan can attribute
    let row = find_callers(&repo, "calc_extra", None);
    assert!(codes(&row).contains(&"dynamic_call".to_string()), "{:?}", row["incomplete_because"]);
    for name in ["zcl_fx_user", "zif_fx_order"] {
        let row = find_callers(&repo, name, None);
        assert!(!codes(&row).contains(&"dynamic_call".to_string()), "{name}: {:?}", row["incomplete_because"]);
    }
}

#[test]
fn abap_fixture_2_5_comment_is_not_a_mention() {
    let repo = fixture_repo();
    let row = find_callers(&repo, "zfx_fm", None);
    // line 12 is a comment, 23 a template's literal part, 24 a literal
    let lines: Vec<u64> = sites(&row, "zfx_dynamic.prog.abap").iter().map(|(_, l, _)| *l).collect();
    for banned in [12, 23, 24] {
        assert!(!lines.contains(&banned), "line {banned} is not code: {lines:?}");
    }
}

#[test]
fn abap_fixture_2_5_arrow_is_member_call() {
    let repo = fixture_repo();
    let row = find_callers(&repo, "describe", Some("zcl_fx_order.clas.abap"));
    let got = sites(&row, "zfx_dynamic.prog.abap");
    // `lo_any->describe( )` and `...=>create( 2 )->describe( )`: receivers, not bare calls
    for line in [21, 22] {
        assert!(got.contains(&("<module level>".to_string(), line, "member_call".to_string())), "{got:?}");
    }
    assert!(!got.iter().any(|(_, _, k)| k == "call"), "{got:?}");
}

#[test]
fn abap_fixture_2_5_dynamic_reasons_counted() {
    // `sem graph --json` carries the unresolved-call counts by reason in its stats block, one per
    // computed call of `zfx_dynamic`, with no env var
    let repo = fixture_repo();
    let output = Command::new(env!("CARGO_BIN_EXE_sem"))
        .current_dir(repo.path())
        .env("DO_NOT_TRACK", "1")
        .env("SEM_LOCAL", "1")
        .env("SEM_NO_INDEX", "1")
        .args(["graph", "--json", "--no-default-excludes"])
        .output()
        .expect("run sem");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let graph: Value = serde_json::from_slice(&output.stdout).expect("json");
    let stats = &graph["stats"];
    assert!(stats["entityCount"].as_u64().unwrap() > 0 && stats["edgeCount"].as_u64().is_some());
    let unresolved = stats["unresolved"].as_object().expect("unresolved counts for an ABAP repo");
    // CALL FUNCTION lv_fm; CALL METHOD zcl_fx_order=>(lv_meth) and lo_any->(lv_meth); PERFORM
    // (lv_form) IN PROGRAM zfx_report and PERFORM show_order IN PROGRAM (lv_prog); CREATE OBJECT
    // ... TYPE (lv_cls)
    for (reason, n) in [
        ("dynamic function name", 1),
        ("dynamic method name", 2),
        ("dynamic form name", 2),
        ("dynamic class name", 1),
    ] {
        assert_eq!(unresolved.get(reason).and_then(Value::as_u64), Some(n), "{reason}: {unresolved:?}");
    }
}

#[test]
fn graph_json_stats_unchanged_without_abap() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("a.py"), "def f():\n    g()\n\n\ndef g():\n    pass\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_sem"))
        .current_dir(repo.path())
        .env("DO_NOT_TRACK", "1")
        .env("SEM_LOCAL", "1")
        .env("SEM_NO_INDEX", "1")
        .args(["graph", "--json"])
        .output()
        .expect("run sem");
    assert!(output.status.success());
    let graph: Value = serde_json::from_slice(&output.stdout).expect("json");
    let keys: Vec<&String> = graph["stats"].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["edgeCount", "entityCount"]);
}
