//! The include graph of an ABAP repo: which program or function group each
//! file is compiled into.
//!
//! `INCLUDE zfoo_f01.` pastes that file into the including program, so a
//! `PERFORM` in the program reaches a form that lives in another abapGit file.
//! Story 2.0 scopes a form to its abapGit object, which already makes a
//! function group one unit (its modules and includes are parts of one object).
//! A program include is an object of its own, and only the `INCLUDE` statement
//! links it. This module joins the objects an `INCLUDE` connects into one
//! compiled unit (union-find over object names) and says which includes name
//! a file that is not in the repo.
//!
//! The unit may over-reach: an include shared by two programs merges both into
//! one unit, so each sees the other's forms. [`IncludeGraph::shared`] counts
//! them.
//!
//! # Use by the calls pipeline (`calls/abap.rs`)
//!
//! - `layout()` takes its per-object directory key from [`IncludeGraph::unit`]
//!   instead of `AbapObject.name`, and `local_home` follows, so a form of an
//!   include is homed in its unit's scope 1.
//! - the lowering reads `INCLUDE` names with `abap_fallback::include_names` (the
//!   same statement cutter) into `FileFacts.includes`, and `layout()` resolves
//!   them through [`IncludeGraph::from_names`]; nothing re-reads the files.
//! - `PERFORM f IN PROGRAM x` lowers to the two-segment path `program:x`, `f`
//!   and every program is importable under that name, so the resolver looks in
//!   program `x`'s unit only (test `abap_fixture_2_4_in_program_target`).
//!   `IN PROGRAM (lv)` is a dynamic site (story 2.5).
//! - [`INCLUDE_NOT_IN_REPO`] joins `Stats.unresolved`, one count per entry of
//!   [`IncludeGraph::unresolved`].
//!
//! This is the only reader of the include graph: since story 2.2 the
//! bag-of-words pass no longer runs on `.abap` files (`replaces_bow()`), so its
//! `build_abap_includes` and the `AbapUnit` incremental table that recorded a
//! reader's unit are gone. The pipeline resolves every ABAP file on each
//! build, a cached-graph rebuild and the red-green session's included
//! (`calls::resolve_call_edges`), and its `layout()` joins the units from
//! every file's lowered `includes`, so a unit that gains or loses an object
//! is seen without a table of its own
//! (`abap_fixture_2_4_incremental_follows_include`).

use std::collections::{BTreeMap, HashMap};

use super::abap_fallback::include_names;
use super::abap_name::{include_file_names, parse_abapgit_name};

/// The reason an `INCLUDE` that names no file of the repo is reported with.
/// abapGit does not serialise a function group's generated `uxx` include, so
/// this is the normal case, and not an error.
///
/// `calls/abap.rs`'s `layout()` adds one count of it to `Stats.unresolved` per
/// entry of [`IncludeGraph::unresolved`].
pub const INCLUDE_NOT_IN_REPO: &str = "include not in repo";

/// One `INCLUDE` statement that names no file of the repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedInclude {
    /// The file that includes it.
    pub file: String,
    /// The include name as written.
    pub include: String,
}

impl UnresolvedInclude {
    pub fn reason(&self) -> &'static str {
        INCLUDE_NOT_IN_REPO
    }
}

/// Include edges between files, and the compiled unit of every object.
#[derive(Debug, Clone, Default)]
pub struct IncludeGraph {
    /// `(including file, included file)`, as the repo spells the paths.
    pub edges: Vec<(String, String)>,
    pub unresolved: Vec<UnresolvedInclude>,
    /// Number of included files that two or more different objects include.
    pub shared: usize,
    /// Folded object name to the folded name of its unit's representative.
    /// Objects no include connects are absent: their unit is themselves.
    unit_of: HashMap<String, String>,
}

impl IncludeGraph {
    /// Reads every program and function group file of `files` (paths under
    /// `root`) for its `INCLUDE` statements. Files that are not abapGit
    /// program or function group sources are not read.
    pub fn build(root: &std::path::Path, files: &[String]) -> Self {
        let sources: Vec<(&str, String)> = files
            .iter()
            .filter(|f| reads_includes(f))
            .filter_map(|f| Some((f.as_str(), std::fs::read_to_string(root.join(f)).ok()?)))
            .collect();
        let named: Vec<(&str, Vec<String>)> = sources
            .iter()
            .map(|(file, text)| (*file, include_names(text)))
            .filter(|(_, names)| !names.is_empty())
            .collect();
        Self::from_names(files, &named)
    }

    /// The graph from each file's include names, already read. `files` is every
    /// file of the repo, the targets an include can resolve to.
    pub fn from_names(files: &[String], includes: &[(&str, Vec<String>)]) -> Self {
        let mut by_file_name: HashMap<String, &str> = HashMap::new();
        for file in files {
            let file_name = file.replace('\\', "/");
            let file_name = file_name.rsplit('/').next().unwrap_or(&file_name);
            by_file_name
                .entry(file_name.to_ascii_lowercase())
                .or_insert(file.as_str());
        }

        let mut graph = IncludeGraph::default();
        let mut parent: BTreeMap<String, String> = BTreeMap::new();
        let mut includers: HashMap<&str, Vec<String>> = HashMap::new();
        for (file, names) in includes {
            let Some(from) = parse_abapgit_name(file) else {
                continue;
            };
            let from_key = from.name.to_ascii_lowercase();
            for name in names {
                let target = include_file_names(name, &from)
                    .into_iter()
                    .find_map(|candidate| by_file_name.get(&candidate).copied());
                let Some(target) = target else {
                    graph.unresolved.push(UnresolvedInclude {
                        file: file.to_string(),
                        include: name.clone(),
                    });
                    continue;
                };
                graph.edges.push((file.to_string(), target.to_string()));
                let Some(to) = parse_abapgit_name(target) else {
                    continue;
                };
                let to_key = to.name.to_ascii_lowercase();
                if to_key != from_key {
                    let objects = includers.entry(target).or_default();
                    if !objects.contains(&from_key) {
                        objects.push(from_key.clone());
                    }
                }
                union(&mut parent, &from_key, &to_key);
            }
        }
        graph.shared = includers
            .values()
            .filter(|objects| objects.len() > 1)
            .count();
        let objects: Vec<String> = parent.keys().cloned().collect();
        let mut by_root: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for object in objects {
            let root = find(&mut parent, &object);
            by_root.entry(root).or_default().push(object);
        }
        // An object no include joins to another is its own unit, as if absent.
        for (root, members) in by_root.into_iter().filter(|(_, m)| m.len() > 1) {
            for member in members {
                graph.unit_of.insert(member, root.clone());
            }
        }
        graph
    }

    /// The compiled unit `object` (an `AbapObject.name`, any case) belongs to,
    /// as a folded name. The unit, not the object, is the scope of a form.
    pub fn unit(&self, object: &str) -> String {
        let key = object.to_ascii_lowercase();
        self.unit_of.get(&key).cloned().unwrap_or(key)
    }

    /// Whether any include joins two objects.
    pub fn is_empty(&self) -> bool {
        self.unit_of.is_empty()
    }
}

/// Only a program or a function group can hold a program `INCLUDE`.
pub fn reads_includes(file: &str) -> bool {
    parse_abapgit_name(file).is_some_and(|o| matches!(o.object_type.as_str(), "prog" | "fugr"))
}

fn find(parent: &mut BTreeMap<String, String>, x: &str) -> String {
    let mut root = x.to_string();
    while let Some(next) = parent.get(&root) {
        if *next == root {
            break;
        }
        root = next.clone();
    }
    parent.entry(x.to_string()).or_insert_with(|| x.to_string());
    root
}

/// Joins two objects; the smaller folded name stays the representative, so the
/// unit's name does not depend on the order files are read.
fn union(parent: &mut BTreeMap<String, String>, a: &str, b: &str) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        let (keep, drop) = if ra < rb { (ra, rb) } else { (rb, ra) };
        parent.insert(drop, keep);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| format!("src/{n}")).collect()
    }

    #[test]
    fn include_joins_program_and_include() {
        let all = files(&[
            "zfx_report.prog.abap",
            "zfx_report_f01.prog.abap",
            "zfx_other.prog.abap",
        ]);
        let report = all[0].as_str();
        let graph = IncludeGraph::from_names(&all, &[(report, vec!["ZFX_REPORT_F01".into()])]);
        assert_eq!(graph.unit("zfx_report"), graph.unit("ZFX_REPORT_F01"));
        assert_ne!(graph.unit("zfx_report"), graph.unit("zfx_other"));
        assert_eq!(graph.edges, [(all[0].clone(), all[1].clone())]);
        assert!(graph.unresolved.is_empty());
    }

    #[test]
    fn include_of_function_group_resolves_to_group_part() {
        let all = files(&["zfx_fg.fugr.saplzfx_fg.abap", "zfx_fg.fugr.lzfx_fgtop.abap"]);
        let main = all[0].as_str();
        let graph = IncludeGraph::from_names(
            &all,
            &[(main, vec!["lzfx_fgtop".into(), "lzfx_fguxx".into()])],
        );
        assert_eq!(graph.edges, [(all[0].clone(), all[1].clone())]);
        assert_eq!(
            graph.unresolved,
            [UnresolvedInclude {
                file: all[0].clone(),
                include: "lzfx_fguxx".into()
            }]
        );
        assert_eq!(graph.unresolved[0].reason(), "include not in repo");
        // One object already: no merge to record.
        assert!(graph.is_empty(), "{graph:?}");
    }

    #[test]
    fn shared_include_is_counted() {
        let all = files(&["a.prog.abap", "b.prog.abap", "s.prog.abap"]);
        let graph = IncludeGraph::from_names(
            &all,
            &[(&all[0], vec!["s".into()]), (&all[1], vec!["s".into()])],
        );
        assert_eq!(graph.shared, 1);
        assert_eq!(graph.unit("a"), graph.unit("b"));
    }
}
