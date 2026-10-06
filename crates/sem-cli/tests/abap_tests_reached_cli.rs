//! `sem impact --tests` on ABAP (story 2.6): the tests are the ABAP Unit methods that reach
//! the target through resolved calls, dispatch and the fixture calls ABAP Unit makes
//! (`setup` and friends), named with their class. The lexical fallback that lists test
//! bodies naming the target is off for ABAP: an empty answer prints the "NO TEST REACHES"
//! line and the completeness verdict, from the graph and from the index alike.
//!
//! The fixture is the ABAP fixture repository, as in `abap_completeness_cli.rs`.

use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const FILES: &[&str] = &[
    "zif_fx_order.intf.abap",
    "zif_fx_order.intf.xml",
    "zif_fx_audit.intf.abap",
    "zif_fx_audit.intf.xml",
    "zcl_fx_order.clas.abap",
    "zcl_fx_order.clas.locals_def.abap",
    "zcl_fx_order.clas.locals_imp.abap",
    "zcl_fx_order.clas.testclasses.abap",
    "zcl_fx_order.clas.xml",
    "zcl_fx_order_sub.clas.abap",
    "zcl_fx_order_sub.clas.xml",
    "zcl_fx_order_alt.clas.abap",
    "zcl_fx_order_alt.clas.xml",
    "zcl_fx_user.clas.abap",
    "zcl_fx_user.clas.testclasses.abap",
    "zcl_fx_user.clas.xml",
    "zcl_fx_other.clas.abap",
    "zcl_fx_other.clas.locals_imp.abap",
    "zcl_fx_other.clas.testclasses.abap",
    "zcl_fx_other.clas.xml",
];

/// A class whose `helper` no test calls through a resolved edge, though a test body
/// writes its name: `lo_any` is typed `REF TO object`, so `lo_any->helper( )` binds nothing.
const UNREACHED: &str = "CLASS zcl_fx_lone DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS helper.\nENDCLASS.\n\nCLASS zcl_fx_lone IMPLEMENTATION.\n  METHOD helper.\n  ENDMETHOD.\nENDCLASS.\n";
const UNREACHED_TEST: &str = "CLASS ltc_lone DEFINITION FINAL FOR TESTING\n  DURATION SHORT RISK LEVEL HARMLESS.\n  PRIVATE SECTION.\n    METHODS calls_helper FOR TESTING.\nENDCLASS.\n\nCLASS ltc_lone IMPLEMENTATION.\n  METHOD calls_helper.\n    DATA lo_any TYPE REF TO object.\n    CALL METHOD lo_any->('HELPER').\n    \" helper\n    lo_any->helper( ).\n  ENDMETHOD.\nENDCLASS.\n";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").current_dir(dir).args(args).status().expect("git");
    assert!(status.success(), "git {args:?}");
}

fn fixture_repo() -> TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sem-core/tests/fixtures/abap");
    let repo = TempDir::new().expect("tempdir");
    for f in FILES {
        fs::copy(src.join(f), repo.path().join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    fs::write(repo.path().join("zcl_fx_lone.clas.abap"), UNREACHED).unwrap();
    fs::write(repo.path().join("zcl_fx_lone.clas.testclasses.abap"), UNREACHED_TEST).unwrap();
    for args in [&["init", "-q"][..], &["config", "user.email", "t@example.com"], &["config", "user.name", "T"]] {
        git(repo.path(), args);
    }
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-q", "-m", "init"]);
    repo
}

/// `sem <args>`, from the graph (`index: false`, `SEM_NO_INDEX=1`) or from a warm index.
fn sem(repo: &TempDir, args: &[&str], index: bool) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sem"));
    cmd.current_dir(repo.path())
        .env("DO_NOT_TRACK", "1")
        .env("SEM_LOCAL", "1")
        .env("SEM_NO_UPDATE_CHECK", "1")
        .env("NO_COLOR", "1")
        .env("SEM_CACHE_DIR", repo.path().join(".git/test-cache"))
        .args(args);
    if !index {
        cmd.env("SEM_NO_INDEX", "1");
    }
    let output = cmd.output().expect("run sem");
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).expect("utf-8")
}

fn tests_json(repo: &TempDir, name: &str, file: &str, index: bool) -> Value {
    let out = sem(repo, &["impact", name, "--file", file, "--tests", "--json"], index);
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("{name}: {e}: {out}"))
}

/// `class.method` of each test in an `impact --tests --json` answer, sorted.
fn labels(v: &Value) -> Vec<String> {
    let mut out: Vec<String> = v["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| format!("{}.{}", t["class"].as_str().unwrap_or("?"), t["name"].as_str().unwrap()))
        .collect();
    out.sort();
    out
}

#[test]
fn abap_fixture_2_6_tests_named_with_class() {
    // `create` is reached by every test of ltc_order through `setup`, by
    // zcl_fx_other's test directly, and by zcl_fx_user's `run_labels_order`
    // through `run`. The text output names each as `class.method`.
    let repo = fixture_repo();
    let v = tests_json(&repo, "create", "zcl_fx_order.clas.abap", false);
    let got = labels(&v);
    for want in [
        "ltc_order.setup",
        "ltc_order.total_starts_at_zero",
        "ltc_order.describe_mentions_id",
        "ltc_other.label_has_tag",
        "ltc_user.run_labels_order",
    ] {
        assert!(got.contains(&want.to_string()), "{want}: {got:?}");
    }
    assert!(!got.contains(&"ltc_user.describe_is_user".to_string()), "{got:?}");
    assert_eq!(v["noTestReaches"], false);
    let text = sem(&repo, &["impact", "create", "--file", "zcl_fx_order.clas.abap", "--tests"], false);
    assert!(text.contains("ltc_order.total_starts_at_zero"), "{text}");
}

#[test]
fn abap_fixture_2_6_no_lexical_fallback_for_abap() {
    // `calls_helper` writes `helper` three times (a dynamic call, a comment,
    // a call on a `REF TO object`) and reaches it through no edge. The
    // lexical fallback would list it; for ABAP the answer is that no test
    // reaches it, with the possible callers.
    let repo = fixture_repo();
    for index in [false, true] {
        let v = tests_json(&repo, "helper", "zcl_fx_lone.clas.abap", index);
        assert_eq!(v["tests"].as_array().unwrap().len(), 0, "index {index}: {v}");
        assert_eq!(v["noTestReaches"], true, "index {index}: {v}");
        assert_eq!(v["callersComplete"], false, "index {index}: {v}");
        let text = sem(&repo, &["impact", "helper", "--file", "zcl_fx_lone.clas.abap", "--tests"], index);
        assert!(text.contains("NO TEST REACHES `helper`"), "index {index}: {text}");
        assert!(!text.contains("lexical fallback"), "index {index}: {text}");
    }
}

#[test]
fn abap_fixture_2_6_cli_index_and_graph_agree() {
    // The same answer from the graph and from the index, found or not.
    let repo = fixture_repo();
    sem(&repo, &["graph", "--json"], true); // warms the cache and writes the index
    for (name, file) in [
        ("create", "zcl_fx_order.clas.abap"),
        ("run", "zcl_fx_user.clas.abap"),
        ("zif_fx_order~get_total", "zcl_fx_order_alt.clas.abap"),
        ("helper", "zcl_fx_lone.clas.abap"),
    ] {
        let graph = tests_json(&repo, name, file, false);
        let index = tests_json(&repo, name, file, true);
        assert_eq!(labels(&graph), labels(&index), "{name}");
        assert_eq!(graph["noTestReaches"], index["noTestReaches"], "{name}");
    }
}

#[test]
fn abap_fixture_2_6_diff_selects_tests() {
    // `sem impact --diff HEAD --tests` on a change to `create` selects the
    // tests that reach it.
    let repo = fixture_repo();
    let path = repo.path().join("zcl_fx_order.clas.abap");
    let source = fs::read_to_string(&path).unwrap();
    fs::write(&path, source.replace("ro_order = NEW #( iv_id = iv_id ).", "ro_order = NEW #( iv_id = iv_id + 0 ).")).unwrap();
    let out = sem(&repo, &["impact", "--diff", "HEAD", "--tests", "--json"], false);
    let mut got: Vec<String> = Vec::new();
    for line in out.lines().filter(|l| !l.trim().is_empty()) {
        let v: Value = serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}"));
        if v["entity"]["name"] == "create" {
            got = labels(&v);
        }
    }
    for want in ["ltc_order.total_starts_at_zero", "ltc_order.describe_mentions_id", "ltc_user.run_labels_order"] {
        assert!(got.contains(&want.to_string()), "{want}: {got:?} in {out}");
    }
}
