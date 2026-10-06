//! `sem find NAME --callers` where a test shares the name. ABAP Unit names a local test
//! class's method after the method it tests (`METHODS create FOR TESTING` beside the global
//! `create`), so a name an agent asks callers of is often defined twice, once as a test. With
//! every candidate but one a test, the answer is that one's and the tests skipped are named;
//! two definitions that are not tests are still refused. Each caller carries the lines in it
//! that write the name (`call_lines`), so the call needs no grep.
//!
//! The fixture is the ABAP fixture repository, as in `abap_completeness_cli.rs`, plus a test
//! class whose method is named `create`.

use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const FILES: &[&str] = &[
    "zif_fx_order.intf.abap",
    "zif_fx_order.intf.xml",
    "zcl_fx_order.clas.abap",
    "zcl_fx_order.clas.locals_def.abap",
    "zcl_fx_order.clas.locals_imp.abap",
    "zcl_fx_order.clas.testclasses.abap",
    "zcl_fx_order.clas.xml",
    "zcl_fx_order_sub.clas.abap",
    "zcl_fx_order_sub.clas.xml",
    "zcl_fx_user.clas.abap",
    "zcl_fx_user.clas.testclasses.abap",
    "zcl_fx_user.clas.xml",
    "zcl_fx_calls.clas.abap",
    "zcl_fx_calls.clas.xml",
];

/// A test class whose test method is named like the global `zcl_fx_order=>create` it calls.
const SAME_NAME_TEST: &str = "CLASS ltc_spy DEFINITION FINAL FOR TESTING\n  DURATION SHORT RISK LEVEL HARMLESS.\n  PRIVATE SECTION.\n    METHODS create FOR TESTING.\nENDCLASS.\n\nCLASS ltc_spy IMPLEMENTATION.\n  METHOD create.\n    cl_abap_unit_assert=>assert_bound( zcl_fx_order=>create( 3 ) ).\n  ENDMETHOD.\nENDCLASS.\n";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn fixture_repo() -> TempDir {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sem-core/tests/fixtures/abap");
    let repo = TempDir::new().expect("tempdir");
    for f in FILES {
        fs::copy(src.join(f), repo.path().join(f)).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    fs::write(
        repo.path().join("zcl_fx_spy.clas.testclasses.abap"),
        SAME_NAME_TEST,
    )
    .unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "t@example.com"],
        &["config", "user.name", "T"],
    ] {
        git(repo.path(), args);
    }
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-q", "-m", "init"]);
    repo
}

/// `sem <args>`, from the graph (`index: false`, `SEM_NO_INDEX=1`) or from a warm index.
fn sem(repo: &TempDir, args: &[&str], index: bool) -> std::process::Output {
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
    cmd.output().expect("run sem")
}

fn callers_json(repo: &TempDir, name: &str, file: Option<&str>, index: bool) -> Value {
    let mut args = vec!["find", name, "--callers", "--json"];
    if let Some(f) = file {
        args.extend(["--file", f]);
    }
    let output = sem(repo, &args, index);
    assert!(
        output.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let rows: Value = serde_json::from_slice(&output.stdout).expect("json");
    rows[0].clone()
}

/// The `call_lines` of the caller named `name` in `file`.
fn call_lines(row: &Value, name: &str, file: &str) -> Vec<u64> {
    let caller = row["related"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name && r["file"] == file)
        .unwrap_or_else(|| panic!("{name} in {file} is not a caller: {}", row["related"]));
    caller["call_lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_u64().unwrap())
        .collect()
}

#[test]
fn callers_skip_a_same_named_test() {
    let repo = fixture_repo();
    sem(&repo, &["graph", "--json"], true); // warms the cache and writes the index
    for index in [false, true] {
        let row = callers_json(&repo, "create", None, index);
        assert_eq!(
            row["entity"]["file"], "zcl_fx_order.clas.abap",
            "index {index}: {row}"
        );
        let skipped = row["skipped"].as_array().unwrap();
        assert_eq!(skipped.len(), 1, "index {index}: {row}");
        assert_eq!(skipped[0]["file"], "zcl_fx_spy.clas.testclasses.abap");
        assert_eq!(skipped[0]["name"], "create");
        // the test still calls the global method, so it is still a caller
        assert_eq!(
            call_lines(&row, "create", "zcl_fx_spy.clas.testclasses.abap"),
            [9],
            "index {index}"
        );
    }
    let output = sem(&repo, &["find", "create", "--callers"], false);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("\n  skipped 1 test definition of the same name: method create zcl_fx_spy.clas.testclasses.abap:8\n"),
        "{text}"
    );
}

#[test]
fn callers_without_a_same_named_test_skip_nothing() {
    let repo = fixture_repo();
    let row = callers_json(&repo, "run", None, false);
    assert_eq!(row["entity"]["file"], "zcl_fx_user.clas.abap");
    assert_eq!(row["skipped"].as_array().unwrap().len(), 0, "{row}");
    let output = sem(&repo, &["find", "run", "--callers"], false);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("skipped"));
}

#[test]
fn callers_of_two_non_tests_still_refused() {
    // `describe` is a method of zcl_fx_order, zcl_fx_order_sub, zcl_fx_user and zcl_fx_calls:
    // none is a test, so no candidate is the answer
    let repo = fixture_repo();
    let output = sem(&repo, &["find", "describe", "--callers", "--json"], false);
    assert!(!output.status.success());
    let refusal: Value = serde_json::from_slice(&output.stdout).expect("refusal json");
    assert_eq!(refusal["resolved"], false);
    assert_eq!(
        refusal["candidates"].as_array().unwrap().len(),
        4,
        "{refusal}"
    );
}

#[test]
fn callers_carry_their_call_lines() {
    let repo = fixture_repo();
    sem(&repo, &["graph", "--json"], true);
    for index in [false, true] {
        let row = callers_json(&repo, "create", None, index);
        // `ZCL_FX_ORDER=>CREATE( 1 )` on line 13 of `run`, in another case than the definition
        assert_eq!(
            call_lines(&row, "run", "zcl_fx_user.clas.abap"),
            [13],
            "index {index}"
        );
        // the functional call in both cases and `CALL METHOD`, one line each
        assert_eq!(
            call_lines(&row, "static_call", "zcl_fx_calls.clas.abap"),
            [32, 33, 34],
            "index {index}"
        );
        assert_eq!(
            call_lines(&row, "setup", "zcl_fx_order.clas.testclasses.abap"),
            [15],
            "index {index}"
        );
    }
    let output = sem(&repo, &["find", "create", "--callers"], false);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("  method run zcl_fx_user.clas.abap:12 (call at zcl_fx_user.clas.abap:13)\n"),
        "{text}"
    );
    assert!(
        text.contains(" (calls at zcl_fx_calls.clas.abap:32, zcl_fx_calls.clas.abap:33, zcl_fx_calls.clas.abap:34)\n"),
        "{text}"
    );
}

#[test]
fn callers_with_no_call_line_have_none() {
    // `setup` runs before each test method of its class, called by ABAP Unit and written by
    // no line: the edge is there, the line list is empty
    let repo = fixture_repo();
    let row = callers_json(
        &repo,
        "setup",
        Some("zcl_fx_order.clas.testclasses.abap"),
        false,
    );
    let related = row["related"].as_array().unwrap();
    assert!(!related.is_empty(), "{row}");
    for caller in related {
        assert_eq!(caller["call_lines"], Value::Array(Vec::new()), "{caller}");
    }
}
