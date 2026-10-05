pub(crate) mod abap_fallback;
pub mod abap_name;
pub mod abap_include;
mod entity_extractor;
pub mod languages;
#[cfg(feature = "oxc-fastpath")]
pub mod oxc_extractor;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::model::entity::SemanticEntity;
use crate::parser::cache;
use crate::parser::fast_extractor;
use crate::parser::plugin::{ParseStats, SemanticParserPlugin};
use crate::utils::hash::{content_hash, structural_hash};
use entity_extractor::extract_entities;
use languages::{get_all_code_extensions, get_language_config};

/// Walk an already-parsed tree and build entities, without re-parsing.
///
/// Exposed so callers that already hold a `Tree` (from
/// [`SemanticParserPlugin::extract_entities_with_tree`] or [`parse_tree`]) can
/// attribute parse cost separately from walk cost. This is the second half of
/// `extract_entities`.
pub use entity_extractor::extract_entities as extract_entities_from_tree;

/// The ABAP comment-and-literal stripper, shared with the CLI's completeness scan so
/// there is one stripping rule. Blanks with spaces and keeps byte length and newlines.
pub use crate::parser::graph::strip_abap_content;

pub struct CodeParserPlugin;

// Thread-local parser cache: one Parser per language per thread.
// Avoids creating a new Parser for every file during parallel graph builds.
thread_local! {
    static PARSER_CACHE: RefCell<HashMap<&'static str, tree_sitter::Parser>> = RefCell::new(HashMap::new());
}

/// Resolve the tree-sitter language config for a file, by extension first and
/// then by shebang. `None` means "not a code file this build can parse".
pub fn language_config_for_content(
    content: &str,
    file_path: &str,
) -> Option<&'static languages::LanguageConfig> {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    get_language_config(&ext).or_else(|| {
        detect_ext_from_content(content).and_then(|shebang_ext| get_language_config(&shebang_ext))
    })
}

/// Parse `content` with the thread-local parser for `config`, from scratch.
pub fn parse_tree(
    config: &'static languages::LanguageConfig,
    content: &str,
) -> Option<tree_sitter::Tree> {
    parse_tree_incremental(config, content, None)
}

/// Count the `ERROR` and `MISSING` nodes of a parse tree, each once. A subtree
/// the parser marks error-free is not entered.
pub fn count_error_nodes(root: tree_sitter::Node) -> usize {
    let mut count = 0;
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.has_error() || node.is_missing() {
            if node.is_error() || node.is_missing() {
                count += 1;
            }
            if cursor.goto_first_child() {
                continue;
            }
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return count;
            }
        }
    }
}

/// One top-level import statement located by the tree-sitter parser: a direct
/// child of the syntax-tree root whose node kind is an import kind for the
/// file's language. Line numbers are 1-based; byte offsets are into the file.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopLevelImport {
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: String,
}

/// The file's top-level import statements, as the parser sees them.
///
/// Only direct children of the tree root are considered, so an import-shaped
/// run of text inside a string or template literal (not a node at all), inside
/// a comment, or nested in a block such as `declare module { ... }` (a child of
/// that block, not of the root) never appears. A multi-line import is one node,
/// returned with the line span of the whole statement. An import that follows
/// other statements is still a top-level child, so it is returned too.
///
/// Returns None for unsupported languages or invalid syntax. Some(empty)
/// means a successful parse with no imports; callers must not confuse them.
pub fn top_level_imports(file_path: &str, content: &str) -> Option<Vec<TopLevelImport>> {
    let Some(config) = language_config_for_content(content, file_path) else {
        return None;
    };
    let Some(tree) = parse_tree(config, content) else {
        return None;
    };
    let src = content.as_bytes();
    let root = tree.root_node();
    if root.has_error() || !matches!(config.id, "typescript" | "tsx" | "javascript" | "python" | "rust" | "go" | "java" | "c" | "cpp") {
        return None;
    }
    let mut cursor = root.walk();
    let mut out = Vec::new();
    for child in root.children(&mut cursor) {
        if !is_import_node_kind(config.id, child.kind()) {
            continue;
        }
        let start = child.start_byte();
        let end = child.end_byte();
        out.push(TopLevelImport {
            kind: child.kind().to_string(),
            start_line: child.start_position().row + 1,
            end_line: child.end_position().row + 1,
            start_byte: start,
            end_byte: end,
            text: String::from_utf8_lossy(&src[start..end]).into_owned(),
        });
    }
    Some(out)
}

/// Whether `kind` is the tree-sitter node kind of an import/use declaration for
/// the language `lang_id`. Covers the languages `sem.addImport` reasons about;
/// an unlisted language yields no parser-backed imports and the caller falls
/// back to its text scan.
fn is_import_node_kind(lang_id: &str, kind: &str) -> bool {
    match lang_id {
        "typescript" | "tsx" | "javascript" => kind == "import_statement",
        "python" => matches!(
            kind,
            "import_statement" | "import_from_statement" | "future_import_statement"
        ),
        "rust" => matches!(kind, "use_declaration" | "mod_item"),
        "go" => kind == "import_declaration",
        "java" => kind == "import_declaration",
        "c" | "cpp" => kind == "preproc_include",
        _ => false,
    }
}

/// Hard wall-clock ceiling for a single-file parse. Healthy files parse in
/// microseconds to low milliseconds, so this budget is far above the normal
/// case and never fires for healthy input. It exists for the pathological
/// case: tree-sitter's GLR error recovery (`ts_parser__handle_error` ->
/// `ts_parser__do_all_potential_reductions`, and `ts_parser__recover` ->
/// `ts_stack_pop_count`) goes super-linear on large inputs that end up in an
/// error-recovery parse (deliberately malformed compiler-fixture files; or,
/// per, a pathologically deep/adversarial data fixture that a
/// grammar never designed for such depth also drives into error recovery --
/// see `crates/sem-core/, "C# pathology
/// (dotnet-runtime)"). Shared with `scope_resolve.rs`'s pass-2 reparse loop,
/// which established this mechanism first for exactly the
/// TypeScript-fixture shape of this same failure mode; this is the pass-1
/// (initial parse) sibling.
///
/// raised from the original 2s to 10s after this budget
/// started spawning a supervisor thread (see `parse_tree_within_budget`'s doc
/// comment) whose wall-clock timing is scheduler-sensitive under load, not
/// just tree-sitter's own progress-callback cancellation. dotnet-runtime
/// ships 6 files (`hugeexpr1.cs`, `HugeField1/2.cs`, `HugeArray1.cs`,
/// `TestData.g.cs`, and siblings under `src/tests/JIT/jit64/opt/cse/`) that
/// are legitimately slow to parse -- 1.5-2.8s in isolation, genuinely
/// error-recovery-bound generated JIT torture-test fixtures, not a bug to
/// route around -- clustered close enough to the old 2s budget that
/// scheduler jitter under 18-way parallelism made whether any given one
/// finished in time non-deterministic: two in-process builds of the same
/// corpus disagreed on `hugeexpr1.cs`'s fate and produced different edge
/// counts (`incr_probe`'s own `cold-vs-build` and `warm-vs-cold` oracles both
/// caught this). The next corpus file above that cluster is >30x slower
/// (`EncryptedXmlSample4.xml`, ~90s, see `LARGE_FILE_BUDGET_THRESHOLD`'s
/// sibling section) with nothing in between, so 10s clears every known
/// legitimate file with a >=3.5x margin while still bounding the genuinely
/// pathological one to a small fraction of its unbounded cost.
///
/// no longer read from either hot path (pass 1's
/// `extract_entities_with_tree`, pass 2's `scope_resolve.rs` reparse loop) --
/// both now use [`is_pathological_large_file`], a deterministic content-shape
/// predicate, as their sole give-up decision. A hybrid was tried first
/// (predicate ahead of this budget, budget kept as fallback for whatever the
/// predicate didn't flag) and *measured* to still reproduce 's
/// chunk-boundary edge-count nondeterminism on dotnet-runtime -- proof some
/// file other than the one confirmed pathological one was still racing this
/// clock. Left defined, undeleted: still correct machinery, still worth
/// knowing about if a future call site genuinely needs a bounded-wall-clock
/// parse (not a give-up-or-not classification decided ahead of time).
pub const PARSE_TIME_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

/// Files at or below this size skip the budget supervisor entirely and go
/// through the plain, unconditional [`parse_tree`] -- exactly the code path
/// and behavior pass 1 has always used. See the call site in
/// `CodeParserPlugin::extract_entities_with_tree` for why this gate exists
/// (thread-spawn scale safety, not a correctness requirement of the budget
/// mechanism itself). 128 KiB is comfortably above every healthy source file
/// this document's corpora contain and comfortably below the multi-megabyte
/// generated/fixture files that have actually been observed pathological.
///
/// no longer read from either hot path -- see [`PARSE_TIME_BUDGET`]'s
/// doc comment. Left defined, undeleted: the thread-oversubscription finding
/// this constant's doc comment records is still true and still worth
/// knowing if a future call site wants a bounded-wall-clock parse, and
/// [`parse_tree_within_budget`] itself is kept working (not gutted) for
/// exactly that case.
pub const LARGE_FILE_BUDGET_THRESHOLD: u64 = 128 * 1024;

/// Parse `content` for `config` with a hard wall-clock ceiling of `budget`.
///
/// no longer called from pass 1 (`extract_entities_with_tree`) or
/// pass 2 (`scope_resolve.rs`'s reparse loop) -- both use
/// [`is_pathological_large_file`], a deterministic content-shape predicate,
/// as their sole give-up decision instead. See that function's doc comment
/// for why a hybrid (predicate ahead of this function, this function kept as
/// fallback) was tried and rejected: measured directly against
/// dotnet-runtime at two chunk sizes, the hybrid still reproduced 's
/// chunk-boundary edge-count nondeterminism, so any file still able to reach
/// this wall-clock race keeps the underlying bug alive regardless of what
/// the predicate catches. Kept working, not deleted: a future caller that
/// genuinely needs a bounded-wall-clock parse (as opposed to a
/// give-up-or-not classification decided ahead of time, immune to
/// scheduling) still has this available.
///
/// Returns `None` if the language is unset (same contract as [`parse_tree`])
/// or if the parse blew through `budget`, in which case the caller must treat
/// the file as unparseable -- the exact same `(Vec::new(), None)` shape
/// [`CodeParserPlugin::extract_entities_with_tree`] already returns for any
/// other unparseable file, so callers need no new handling for this case.
///
/// this runs the parse on a supervisor thread and races it against
/// `budget` with `recv_timeout`, rather than tree-sitter's own
/// `parse_with_options` + `progress_callback` cancellation mechanism (what an
/// earlier revision of this function used, mirroring the pass-2 reparse loop
/// this budget was first built for in). That callback-based read API
/// turned out to have a correctness bug independent of timing: on at least
/// one small, fast-to-parse but adversarially-crafted file in the
/// dotnet-runtime corpus (`EncryptedXmlSample5.xml`, 5,586 bytes -- an XML
/// decryption-transform-chain fixture, not a large or slow one), it returned
/// a *completed* tree containing a node with `end_byte() = 5630`, past the
/// end of the 5,586-byte input -- verified by isolating the same content
/// through the plain `Parser::parse` (this function's read path) instead,
/// which parses it correctly (`root_kind=document`, no error, 29 entities,
/// matching every other well-formed file's contract) in under a millisecond.
/// Nothing here changed what the *progress_callback* budget mechanism itself
/// does for pass 2's reparse loop (untouched); this function no longer uses
/// it, at all, so pass 1 (every file, not just the >20k-file chunked-repo
/// reparse subset) cannot hit that bug either.
///
/// A supervisor thread per call means every pass-1 file pays one thread
/// spawn+join even on the overwhelming majority of files that never approach
/// `budget` -- measured net win regardless (see the C# pathology section above): thread spawn/join is microseconds, the
/// pathology it bounds was tens of seconds on a single file.
pub fn parse_tree_within_budget(
    config: &'static languages::LanguageConfig,
    content: &str,
    budget: std::time::Duration,
) -> Option<tree_sitter::Tree> {
    let language = (config.get_language)()?;
    let content_owned = content.to_string();
    let (tx, rx) = std::sync::mpsc::channel::<Option<tree_sitter::Tree>>();

    // `Parser` and `Tree` are both `Send` (tree-sitter's own unsafe impls).
    // The spawned thread is intentionally allowed to outlive this call on
    // the timeout path below -- there is no `join` to wait on, so a
    // pathological file's still-running parse is abandoned, not aborted;
    // it consumes one background thread's CPU until tree-sitter's own parse
    // completes, same as it always would have, just off the critical path.
    let spawned = std::thread::Builder::new().spawn(move || {
        let mut parser = tree_sitter::Parser::new();
        let _ = parser.set_language(&language);
        let tree = parser.parse(content_owned.as_bytes(), None);
        let _ = tx.send(tree);
    });
    if spawned.is_err() {
        // Thread-spawn failure (resource exhaustion): fall back to a direct,
        // unbounded parse on the calling thread rather than silently losing
        // the file's entities -- same behavior as before this budget existed.
        return parse_tree(config, content);
    }

    rx.recv_timeout(budget).unwrap_or_default()
}

/// The shape threshold [`is_pathological_large_file`] classifies on: the
/// longest single `\n`-delimited run in a file's content, in bytes.
///
/// ("Memory attribution" section):
/// measured directly with `examples/parse_time_probe.rs` (sequential,
/// zero-contention, single-file-at-a-time -- the wall-clock time a file
/// actually costs to parse, isolated from anything [`PARSE_TIME_BUDGET`]'s
/// concurrent-supervisor-thread race adds on top) across dotnet-runtime,
/// linux, llvm-project, elasticsearch, TypeScript-monster, and tiptap's
/// whole >128KiB-file populations:
///
/// - **The one confirmed pathological file**: dotnet-runtime's
///   `EncryptedXmlSample4.xml`, 9.5MB total, one embedded `<CipherValue>`
///   payload forming a single 8,441,855-byte line (not deep tag-nesting --
///   only ~1,245 open tags total) -- **92.265s** solo parse, 9.2x over
///   `PARSE_TIME_BUDGET`.
/// - **Every other large-line file measured parses fast, regardless of
///   line length** -- and line length alone does *not* cleanly separate
///   these from the pathological file the way an earlier revision of this
///   fix assumed: TypeScript-monster ships
///   `.../should-be-able-to-return-the-file-size-when-a-JS-file-is-too-large-to-load-into-text.js`,
///   a deliberate torture fixture whose single line is 4,194,306 bytes --
///   only 2x below the pathological XML's line length -- yet it parses in
///   **52 milliseconds**. Several other TypeScript-monster fixtures are
///   ~100% one line (`codeFixClassImplementInterfaceNoTruncationProperties.ts`,
///   `excessivelyLargeArrayLiteralCompletions.ts`, both >99.9% single-line)
///   and still parse in single-digit milliseconds. Total file size doesn't
///   separate the populations either -- dotnet-runtime's legitimately-slow
///   `hugeexpr1.cs` cluster is up to 24MB, *larger* than the 9.5MB
///   pathological file. **The pathology is grammar-specific (XML's
///   scanner on this input), not a generic function of content shape** --
///   this predicate is a coarse, conservative proxy for it, not a proof.
///
/// [`PATHOLOGICAL_LINE_THRESHOLD`] is set at the geometric mean of the
/// widest legitimate line measured (TypeScript-monster's 4,194,306 bytes)
/// and the one confirmed pathological line (8,441,855 bytes): **6 MiB**,
/// giving both populations a ~1.4x margin -- thinner than an ideal
/// discriminator would have, disclosed rather than overstated. Because the
/// margin is thin and the underlying pathology is grammar-dependent rather
/// than structurally proven, this predicate is deliberately *not* the sole
/// line of defense: see [`is_pathological_large_file`]'s doc comment for
/// why [`parse_tree_within_budget`]'s wall-clock ceiling stays in place as
/// a fallback for whatever this predicate doesn't catch.
pub const PATHOLOGICAL_LINE_THRESHOLD: u64 = 6 * 1024 * 1024;

/// Longest single `\n`-delimited run in `content`, in bytes. O(n), one pass
/// over the bytes already in hand, no allocation -- see
/// [`is_pathological_large_file`] for why this is cheap enough to run
/// unconditionally rather than gating it behind a coarser pre-check.
fn max_line_len(content: &str) -> usize {
    content
        .as_bytes()
        .split(|&b| b == b'\n')
        .map(<[u8]>::len)
        .max()
        .unwrap_or(0)
}

/// Deterministic, pure-per-file pre-filter that removes the one
/// *confirmed* source of [`parse_tree_within_budget`]'s wall-clock race from
/// pass-1's and pass-2's large-file give-up decision, before that race ever
/// starts.
///
/// ## The bug this targets
///
/// The give-up decision used to be entirely the outcome of racing a
/// spawned-thread parse against [`PARSE_TIME_BUDGET`]'s 10s wall clock.
/// Which files finished inside that window depended on how many *other*
/// parses -- including other budget-racing supervisor threads -- were in
/// flight on other threads at the same moment, itself a function of
/// `SCOPE_RESOLVE_FILE_CHUNK_SIZE` (a different chunk size batches `rayon`
/// work differently, changing how many large files land in the same chunk
/// together, changing contention). This was not hypothetical: shrinking
/// dotnet-runtime's chunk size from 5,000 to 1,000 files moved
/// the corpus's resolved edge count by exactly +1, reproducibly, with no
/// other change to the input.
///
/// ## The fix, and its honest scope
///
/// `EncryptedXmlSample4.xml` is dotnet-runtime's single heaviest
/// contributor to that contention -- a 92-second supervisor thread pinning
/// a core in *every* chunk it lands in, every build, deterministically
/// present regardless of outcome (it always exceeds budget; the doc comment
/// on [`PATHOLOGICAL_LINE_THRESHOLD`] has the measurement). This function
/// removes it from the race entirely: content this returns `true` for is
/// classified unparseable *before any parse is attempted and before any
/// thread is spawned* -- a pure function of the file's own bytes, identical
/// on every call, on every machine, under any concurrent load, forever.
/// Removing dotnet-runtime's one heaviest, always-present contention source
/// is expected to remove the specific +1-edge nondeterminism
/// measured (that borderline file's own solo parse time is nowhere near
/// 10s -- see `parse_time_probe`'s measurements
/// -- so its flip was contention-driven, not intrinsic).
///
/// This is *not* a claim that every conceivable pathological file is now
/// classified without a clock: [`PATHOLOGICAL_LINE_THRESHOLD`]'s doc
/// comment shows the underlying pathology is grammar-specific, not a
/// provable function of content shape, so [`parse_tree_within_budget`]
/// stays in place as a fallback at both call sites for whatever this
/// coarse predicate doesn't catch -- disclosed residual risk, not silently
/// dropped protection.
pub fn is_pathological_large_file(content: &str) -> bool {
    content.len() as u64 > PATHOLOGICAL_LINE_THRESHOLD
        && max_line_len(content) as u64 > PATHOLOGICAL_LINE_THRESHOLD
}

/// Parse `content`, optionally reusing `old_tree` for an incremental reparse.
///
/// The caller is responsible for having already applied the matching
/// [`tree_sitter::InputEdit`]s to `old_tree`; passing an un-edited or stale
/// tree yields a wrong parse, which is exactly why sem-core's own entry points
/// never do this. It is public so callers that *do* track edits (and the
/// incremental benchmark) can measure and use it.
pub fn parse_tree_incremental(
    config: &'static languages::LanguageConfig,
    content: &str,
    old_tree: Option<&tree_sitter::Tree>,
) -> Option<tree_sitter::Tree> {
    let language = (config.get_language)()?;

    PARSER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let parser = cache.entry(config.id).or_insert_with(|| {
            let mut p = tree_sitter::Parser::new();
            let _ = p.set_language(&language);
            p
        });

        parser.parse(content.as_bytes(), old_tree)
    })
}

fn has_non_comment_content(node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut worklist = Vec::new();
    let mut cursor = node.walk();
    worklist.extend(node.children(&mut cursor));

    while let Some(node) = worklist.pop() {
        if is_comment_node(node.kind()) {
            continue;
        }

        if node.child_count() == 0 {
            let start = node.start_byte();
            let end = node.end_byte();
            if start < end
                && end <= source.len()
                && source[start..end].iter().any(|b| !b.is_ascii_whitespace())
            {
                return true;
            }
            continue;
        }

        let mut cursor = node.walk();
        worklist.extend(node.children(&mut cursor));
    }

    false
}

fn is_comment_node(kind: &str) -> bool {
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "doc_comment" | "tag_comment"
    )
}

fn shebang_line(content: &str) -> Option<&str> {
    content
        .strip_prefix("#!")
        .map(|rest| rest.lines().next().unwrap_or(""))
}

impl SemanticParserPlugin for CodeParserPlugin {
    fn id(&self) -> &str {
        "code"
    }

    fn extensions(&self) -> &[&str] {
        get_all_code_extensions()
    }

    /// Content-addressed: identical `(content, file_path)` is served from the
    /// process-local cache instead of re-parsing. See [`crate::parser::cache`]
    /// for the budget and the env switches that turn it off.
    ///
    /// `extract_entities_with_tree` is deliberately *not* cached — it hands the
    /// caller the `Tree`, which is neither cheap to keep nor `Sync`.
    ///
    /// This is also the one seam an installed
    /// [`FastExtractor`](crate::parser::fast_extractor::FastExtractor) sits
    /// behind: it is the entities-only API, so a parser with no tree-sitter
    /// `Tree` to hand back can answer it in full. `extract_entities_with_tree`
    /// is deliberately *not* routed through the fast path — its callers (pass
    /// 1 of `EntityGraph::build`) need the tree itself, and a fast path there
    /// would trade a parallel parse for a serial pass-2 re-parse. A decline
    /// (`None`) falls through to tree-sitter with no observable difference
    /// beyond timing.
    fn extract_entities(&self, content: &str, file_path: &str) -> Vec<SemanticEntity> {
        cache::get_or_extract(
            "code",
            file_path,
            content,
            || match fast_extractor::try_extract(file_path, content) {
                Some(entities) => entities,
                None => self.extract_entities_with_tree(content, file_path).0,
            },
        )
    }

    fn extract_entities_with_tree(
        &self,
        content: &str,
        file_path: &str,
    ) -> (Vec<SemanticEntity>, Option<tree_sitter::Tree>) {
        let Some(config) = language_config_for_content(content, file_path) else {
            return (Vec::new(), None);
        };

        // gave pass 1 a wall-clock ceiling for large files (a single
        // pathological file could otherwise pin pass 1 for tens of seconds).
        // replaced it outright with a deterministic, pure-per-file
        // predicate -- see `is_pathological_large_file`'s doc comment for why
        // a hybrid (predicate-then-budget-fallback) was tried first and
        // measurement disproved it: with the fallback still in place, the
        // exact dotnet-runtime chunk-boundary edge-count flip found
        // still reproduced (981,283 at a 5,000-file chunk vs 981,284 at a
        // 1,000-file chunk, stable across repeat runs at each size) --
        // proof the flipping file was never `EncryptedXmlSample4.xml`, and
        // that *any* file still going through `parse_tree_within_budget`'s
        // wall-clock race keeps the bug alive regardless of the predicate.
        // With the fallback removed entirely, both chunk sizes produce
        // identical entities/edges. No
        // thread is spawned on this path any more for any file: flagged
        // content is rejected before a parse is attempted, and every other
        // file -- including every large-but-healthy file the old budget
        // mechanism used to race against a 10s clock -- goes through the
        // same plain, unconditional `parse_tree` a small file always has.
        let tree = if is_pathological_large_file(content) {
            None
        } else {
            parse_tree(config, content)
        };
        let Some(tree) = tree else {
            return (Vec::new(), None);
        };

        let entities = extract_entities(&tree, file_path, config, content);
        (entities, Some(tree))
    }

    /// Parses from scratch, not through the entity cache: the error count
    /// needs the tree, which the cache does not keep.
    fn parse_stats(&self, content: &str, file_path: &str) -> ParseStats {
        let (entities, tree) = self.extract_entities_with_tree(content, file_path);
        let fallback_entity_count = entities
            .iter()
            .filter(|e| {
                e.metadata
                    .as_ref()
                    .and_then(|m| m.get("source"))
                    .is_some_and(|source| source == "abap-fallback")
            })
            .count();
        ParseStats {
            entity_count: entities.len(),
            grammar_entity_count: entities.len() - fallback_entity_count,
            fallback_entity_count,
            error_node_count: tree.map_or(0, |t| count_error_nodes(t.root_node())),
        }
    }

    /// Also content-addressed — it otherwise re-parses the file from scratch.
    fn structural_hash_content(&self, content: &str, file_path: &str) -> Option<String> {
        cache::get_or_structural_hash("code", file_path, content, || {
            let config = language_config_for_content(content, file_path)?;
            let tree = parse_tree(config, content)?;
            let shebang = shebang_line(content);
            if shebang.is_none() && !has_non_comment_content(tree.root_node(), content.as_bytes()) {
                return Some(String::new());
            }
            let structural = structural_hash(tree.root_node(), content.as_bytes());
            match shebang {
                Some(shebang) => Some(content_hash(&format!("shebang:{shebang}\n{structural}"))),
                None => Some(structural),
            }
        })
    }
}

use crate::parser::registry::detect_ext_from_content;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_entity_extraction() {
        let code = r#"
package com.example;

import java.util.List;

public class UserService {
    private String name;

    public UserService(String name) {
        this.name = name;
    }

    public List<User> getUsers() {
        return db.findAll();
    }

    public void createUser(User user) {
        db.save(user);
    }
}

interface Repository<T> {
    T findById(String id);
    List<T> findAll();
}

enum Status {
    ACTIVE,
    INACTIVE,
    DELETED
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "UserService.java");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "Java entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"UserService"),
            "Should find class UserService, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Repository"),
            "Should find interface Repository, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Status"),
            "Should find enum Status, got: {:?}",
            names
        );

        // A field is named by its declarator, not its type: `private String name;`
        // is the field `name`, not `String`.
        let field = entities
            .iter()
            .find(|e| e.entity_type == "field")
            .expect("should extract the field entity");
        assert_eq!(
            field.name, "name",
            "field should be named by its declarator, got: {:?}",
            field.name
        );
    }

    #[test]
    fn test_java_nested_methods() {
        let code = r#"
public class Calculator {
    public int add(int a, int b) {
        return a + b;
    }

    public int subtract(int a, int b) {
        return a - b;
    }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Calculator.java");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Java nested: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"Calculator"),
            "Should find Calculator class"
        );
        assert!(
            names.contains(&"add"),
            "Should find add method, got: {:?}",
            names
        );
        assert!(
            names.contains(&"subtract"),
            "Should find subtract method, got: {:?}",
            names
        );

        // Methods should have Calculator as parent
        let add = entities.iter().find(|e| e.name == "add").unwrap();
        assert!(add.parent_id.is_some(), "add should have parent_id");
    }

    #[test]
    fn test_c_entity_extraction() {
        let code = r#"
#include <stdio.h>

struct Point {
    int x;
    int y;
};

enum Color {
    RED,
    GREEN,
    BLUE
};

typedef struct {
    char name[50];
    int age;
} Person;

void greet(const char* name) {
    printf("Hello, %s!\n", name);
}

int add(int a, int b) {
    return a + b;
}

int main() {
    greet("world");
    return 0;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.c");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "C entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"greet"),
            "Should find greet function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"add"),
            "Should find add function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"main"),
            "Should find main function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Point"),
            "Should find Point struct, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Color"),
            "Should find Color enum, got: {:?}",
            names
        );
    }

    #[test]
    fn test_c_function_locals_not_extracted() {
        let code = r#"
int global_count = 0;
int helper(void);

int main(void) {
    int local = helper();
    const char *message = "hello";
    return local + global_count;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.c");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"global_count"), "got: {:?}", names);
        assert!(names.contains(&"helper"), "got: {:?}", names);
        assert!(names.contains(&"main"), "got: {:?}", names);
        assert!(!names.contains(&"local"), "got: {:?}", names);
        assert!(!names.contains(&"message"), "got: {:?}", names);
    }

    #[test]
    fn test_cpp_entity_extraction() {
        let code = "namespace math {\nclass Vector3 {\npublic:\n    float length() const { return 0; }\n};\n}\nvoid greet() {}\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.cpp");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"math"), "got: {:?}", names);
        assert!(names.contains(&"Vector3"), "got: {:?}", names);
        assert!(names.contains(&"greet"), "got: {:?}", names);
    }

    #[test]
    fn test_cpp_function_locals_not_extracted() {
        let code = r#"
int global_value = 1;
int helper();

int main() {
    int local = helper();
    auto lambda = []() {
        int lambda_local = 3;
        return lambda_local;
    };
    return local + lambda();
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.cpp");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"global_value"), "got: {:?}", names);
        assert!(names.contains(&"helper"), "got: {:?}", names);
        assert!(names.contains(&"main"), "got: {:?}", names);
        assert!(!names.contains(&"local"), "got: {:?}", names);
        assert!(!names.contains(&"lambda"), "got: {:?}", names);
        assert!(!names.contains(&"lambda_local"), "got: {:?}", names);
    }

    #[test]
    fn test_ruby_entity_extraction() {
        let code = "module Auth\n  class User\n    def greet\n      \"hi\"\n    end\n  end\nend\ndef helper(x)\n  x * 2\nend\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "auth.rb");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"Auth"), "got: {:?}", names);
        assert!(names.contains(&"User"), "got: {:?}", names);
        assert!(names.contains(&"helper"), "got: {:?}", names);
    }

    #[test]
    fn test_csharp_entity_extraction() {
        let code = "namespace MyApp {\npublic class User {\n    public string GetName() { return \"\"; }\n}\npublic enum Role { Admin, User }\n}\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Models.cs");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"MyApp"), "got: {:?}", names);
        assert!(names.contains(&"User"), "got: {:?}", names);
        assert!(names.contains(&"Role"), "got: {:?}", names);
    }

    #[test]
    fn test_swift_entity_extraction() {
        let code = r#"
import Foundation

typealias Handler = (Int) -> Void

prefix operator ~~~

class UserService {
    var name: String

    init(name: String) {
        self.name = name
    }

    deinit {
        print("freed")
    }

    func getUsers() -> [User] {
        return db.findAll()
    }
}

struct Point {
    var x: Double
    var y: Double

    subscript(index: Int) -> Double {
        return x + y + Double(index)
    }
}

enum Status {
    case active
    case inactive
    case deleted
}

protocol Repository {
    associatedtype Canvas
    func findById(id: String) -> Canvas?
    func findAll() -> [Canvas]
}

func helper(x: Int) -> Int {
    return x * 2
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "UserService.swift");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Swift entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"UserService"),
            "Should find class UserService, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Point"),
            "Should find struct Point, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Status"),
            "Should find enum Status, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Repository"),
            "Should find protocol Repository, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Canvas"),
            "Should find associatedtype Canvas, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Handler"),
            "Should find typealias Handler, got: {:?}",
            names
        );
        assert!(
            names.contains(&"~~~"),
            "Should find custom operator ~~~, got: {:?}",
            names
        );
        assert!(
            names.contains(&"init"),
            "Should find initializer init, got: {:?}",
            names
        );
        assert!(
            names.contains(&"deinit"),
            "Should find deinitializer deinit, got: {:?}",
            names
        );
        assert!(
            names.contains(&"subscript"),
            "Should find subscript, got: {:?}",
            names
        );
        assert!(
            names.contains(&"helper"),
            "Should find function helper, got: {:?}",
            names
        );

        let handler = entities.iter().find(|e| e.name == "Handler").unwrap();
        assert_eq!(handler.entity_type, "type");
        assert!(handler.parent_id.is_none());

        let operator = entities.iter().find(|e| e.name == "~~~").unwrap();
        assert_eq!(operator.entity_type, "operator");
        assert!(operator.parent_id.is_none());

        let user_service = entities.iter().find(|e| e.name == "UserService").unwrap();
        assert_eq!(user_service.entity_type, "class");

        let initializer = entities.iter().find(|e| e.name == "init").unwrap();
        assert_eq!(initializer.entity_type, "init");
        assert_eq!(
            initializer.parent_id.as_deref(),
            Some(user_service.id.as_str())
        );
        assert_eq!(
            initializer.id,
            "UserService.swift::class::UserService::init"
        );

        let deinitializer = entities.iter().find(|e| e.name == "deinit").unwrap();
        assert_eq!(deinitializer.entity_type, "deinit");
        assert_eq!(
            deinitializer.parent_id.as_deref(),
            Some(user_service.id.as_str())
        );
        assert_eq!(
            deinitializer.id,
            "UserService.swift::class::UserService::deinit"
        );

        let point = entities.iter().find(|e| e.name == "Point").unwrap();
        assert_eq!(point.entity_type, "struct");

        let subscript = entities.iter().find(|e| e.name == "subscript").unwrap();
        assert_eq!(subscript.entity_type, "subscript");
        assert_eq!(subscript.parent_id.as_deref(), Some(point.id.as_str()));
        assert_eq!(subscript.id, "UserService.swift::struct::Point::subscript");

        let status = entities.iter().find(|e| e.name == "Status").unwrap();
        assert_eq!(status.entity_type, "enum");

        let repository = entities.iter().find(|e| e.name == "Repository").unwrap();
        assert_eq!(repository.entity_type, "protocol");
        assert_eq!(repository.id, "UserService.swift::protocol::Repository");

        let canvas = entities.iter().find(|e| e.name == "Canvas").unwrap();
        assert_eq!(canvas.entity_type, "associatedtype");
        assert_eq!(canvas.parent_id.as_deref(), Some(repository.id.as_str()));
        assert_eq!(canvas.id, "UserService.swift::protocol::Repository::Canvas");
    }

    #[test]
    fn test_swift_multi_binding_property_extraction() {
        let code = r#"
struct Point {
    var x, y: Int
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Point.swift");
        let point = entities.iter().find(|e| e.name == "Point").unwrap();
        let properties: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();

        assert_eq!(
            properties
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        assert!(properties
            .iter()
            .all(|property| property.parent_id.as_deref() == Some(point.id.as_str())));
        assert_eq!(properties[0].content, "var x: Int");
        assert_eq!(properties[1].content, "var y: Int");
    }

    #[test]
    fn test_swift_multi_binding_property_content_is_per_binding() {
        let typed_code = r#"
struct Types {
    var x: Int, y: String
}
"#;
        let plugin = CodeParserPlugin;
        let typed_entities = plugin.extract_entities(typed_code, "Types.swift");
        let typed_properties: Vec<_> = typed_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert_eq!(typed_properties[0].content, "var x: Int");
        assert_eq!(typed_properties[1].content, "var y: String");

        let mixed_code = r#"
struct Mixed {
    var x, y: Int, z: String
}
"#;
        let mixed_entities = plugin.extract_entities(mixed_code, "Mixed.swift");
        let mixed_properties: Vec<_> = mixed_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert_eq!(mixed_properties[0].content, "var x: Int");
        assert_eq!(mixed_properties[1].content, "var y: Int");
        assert_eq!(mixed_properties[2].content, "var z: String");

        let generic_code = r#"
struct GenericTypes {
    var lookup: Dictionary<String, Int>, count: Int
}
"#;
        let generic_entities = plugin.extract_entities(generic_code, "GenericTypes.swift");
        let generic_properties: Vec<_> = generic_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert_eq!(
            generic_properties[0].content,
            "var lookup: Dictionary<String, Int>"
        );
        assert_eq!(generic_properties[1].content, "var count: Int");

        let initializer_code = r#"
struct Initializers {
    var a = Foo(), b = Bar()
}
"#;
        let initializer_entities = plugin.extract_entities(initializer_code, "Initializers.swift");
        let initializer_properties: Vec<_> = initializer_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert!(initializer_properties[0].content.contains("Foo()"));
        assert!(!initializer_properties[0].content.contains("Bar()"));
        assert!(initializer_properties[1].content.contains("Bar()"));
        assert!(!initializer_properties[1].content.contains("Foo()"));

        let constants_code = r#"
struct Constants {
    let first, second, third: Int
}
"#;
        let constants_entities = plugin.extract_entities(constants_code, "Constants.swift");
        let constants_properties: Vec<_> = constants_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert_eq!(
            constants_properties
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        assert_eq!(constants_properties[0].content, "let first: Int");
        assert_eq!(constants_properties[1].content, "let second: Int");
        assert_eq!(constants_properties[2].content, "let third: Int");

        let semicolon_code = r#"
struct Semicolons {
    var left, right: Int; var next: Int
}
"#;
        let semicolon_entities = plugin.extract_entities(semicolon_code, "Semicolons.swift");
        let semicolon_properties: Vec<_> = semicolon_entities
            .iter()
            .filter(|e| e.entity_type == "property")
            .collect();
        assert_eq!(semicolon_properties[0].content, "var left: Int");
        assert_eq!(semicolon_properties[1].content, "var right: Int");
        assert_eq!(semicolon_properties[2].content, "var next: Int");
    }

    #[test]
    fn test_swift_body_locals_not_extracted_as_properties() {
        let code = r#"
class Cache {
    var stored: Int

    var computed: Int {
        let computedLocal = stored + 1
        func computedNested() -> Int {
            return computedLocal
        }
        return computedNested()
    }

    var explicit: Int {
        get {
            let getterLocal = stored
            func getterNested() -> Int {
                return getterLocal
            }
            return getterNested()
        }
    }

    init(seed: Int) {
        let initial = seed
        self.stored = initial
    }

    func value() -> Int {
        let doubled = stored * 2
        var offset = doubled + 1
        func nested() -> Int {
            let insideNested = offset
            return insideNested
        }
        return nested()
    }

    subscript(index: Int) -> Int {
        let shifted = index + stored
        func subscriptNested() -> Int {
            return shifted
        }
        return subscriptNested()
    }

    deinit {
        let closing = stored
        _ = closing
    }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Cache.swift");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"Cache"), "got: {:?}", names);
        assert!(names.contains(&"stored"), "got: {:?}", names);
        assert!(names.contains(&"computed"), "got: {:?}", names);
        assert!(names.contains(&"explicit"), "got: {:?}", names);
        assert!(names.contains(&"init"), "got: {:?}", names);
        assert!(names.contains(&"value"), "got: {:?}", names);
        assert!(names.contains(&"computedNested"), "got: {:?}", names);
        assert!(names.contains(&"getterNested"), "got: {:?}", names);
        assert!(names.contains(&"nested"), "got: {:?}", names);
        assert!(names.contains(&"subscriptNested"), "got: {:?}", names);
        assert!(names.contains(&"subscript"), "got: {:?}", names);
        assert!(names.contains(&"deinit"), "got: {:?}", names);
        assert!(!names.contains(&"Int"), "got: {:?}", names);

        for local in [
            "computedLocal",
            "getterLocal",
            "initial",
            "doubled",
            "offset",
            "insideNested",
            "shifted",
            "closing",
        ] {
            assert!(
                !names.contains(&local),
                "{local} should not be an entity. Got: {:?}",
                names
            );
        }
    }

    #[test]
    fn test_swift_suppressed_multi_binding_initializers_are_traversed() {
        let code = r#"
func outer() {
    let a = { func innerA() -> Int { 1 } },
        b = { func innerB() -> Int { 2 } }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Locals.swift");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"outer"), "got: {:?}", names);
        assert!(names.contains(&"innerA"), "got: {:?}", names);
        assert!(names.contains(&"innerB"), "got: {:?}", names);
        assert!(
            !names.contains(&"a"),
            "local binding should stay suppressed: {:?}",
            names
        );
        assert!(
            !names.contains(&"b"),
            "local binding should stay suppressed: {:?}",
            names
        );
    }

    #[test]
    fn test_swift_conditional_compilation_inside_struct() {
        let code = r#"
import ArgumentParser

public struct TuistCommand: AsyncParsableCommand {
    public init() {}

    public static var configuration: CommandConfiguration {
        let comment = "brace in string }"
        let multiline = """
        brace in multiline }
        escaped \"""
        """
        /* brace in comment } */
        CommandConfiguration(commandName: "tuist")
    }

    #if os(macOS)
        public static var groupedSubcommands: [ParsableCommand.Type] {
            [InstallCommand.self]
        }
    #else
        public static var groupedSubcommands: [ParsableCommand.Type] {
            []
        }
    #endif

    public func run() async throws {}
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "TuistCommand.swift");
        eprintln!(
            "Swift conditional entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        let command = entities
            .iter()
            .find(|e| e.name == "TuistCommand")
            .expect("Should recover TuistCommand struct");
        assert_eq!(command.entity_type, "struct");
        assert!(command.parent_id.is_none());

        let renamed_code = code.replace("TuistCommand", "RenamedCommand");
        let renamed_entities = plugin.extract_entities(&renamed_code, "TuistCommand.swift");
        let renamed_command = renamed_entities
            .iter()
            .find(|e| e.name == "RenamedCommand")
            .expect("Should recover renamed command struct");
        assert_eq!(command.structural_hash, renamed_command.structural_hash);

        for member in ["init", "configuration", "run"] {
            let entity = entities
                .iter()
                .find(|e| e.name == member)
                .unwrap_or_else(|| panic!("Should find {member}"));
            assert_eq!(entity.parent_id.as_deref(), Some(command.id.as_str()));
        }

        let grouped_subcommands: Vec<_> = entities
            .iter()
            .filter(|e| e.name == "groupedSubcommands")
            .collect();
        assert_eq!(grouped_subcommands.len(), 2);
        assert!(grouped_subcommands
            .iter()
            .all(|entity| entity.parent_id.as_deref() == Some(command.id.as_str())));
    }

    #[test]
    fn test_swift_conditional_compilation_with_interpolated_brace_string() {
        let plugin = CodeParserPlugin;
        for (container_name, code) in [
            (
                "Config",
                r#"
class Config {
    let tpl = "prefix \("}") suffix"
#if DEBUG
    func dump() { print(tpl) }
#endif
    func render() -> String { return tpl }
}

struct Tail { let q: Int }
"#,
            ),
            (
                "RawConfig",
                r##"
class RawConfig {
    let tpl = #"prefix \#("{") suffix"#
#if DEBUG
    func dump() { print(tpl) }
#endif
    func render() -> String { return tpl }
}
"##,
            ),
            (
                "MultilineConfig",
                r#"
class MultilineConfig {
    let tpl = """
    prefix \("}") suffix
    """
#if DEBUG
    func dump() { print(tpl) }
#endif
    func render() -> String { return tpl }
}
"#,
            ),
            (
                "ClosureConfig",
                r#"
class ClosureConfig {
    let tpl = "prefix \(["}"].map { $0 }.joined()) suffix"
#if DEBUG
    func dump() { print(tpl) }
#endif
    func render() -> String { return tpl }
}
"#,
            ),
        ] {
            let file_path = format!("{container_name}.swift");
            let entities = plugin.extract_entities(code, &file_path);
            let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
            let container = entities
                .iter()
                .find(|e| e.name == container_name)
                .unwrap_or_else(|| {
                    panic!("Should recover {container_name}, got: {names:?}");
                });
            assert_eq!(container.entity_type, "class");
            assert!(container.parent_id.is_none());

            for member in ["tpl", "dump", "render"] {
                let entity = entities
                    .iter()
                    .find(|e| e.name == member)
                    .unwrap_or_else(|| {
                        panic!("Should find {member} in {container_name}, got: {names:?}");
                    });
                assert_eq!(entity.parent_id.as_deref(), Some(container.id.as_str()));
            }
        }
    }

    #[test]
    fn test_elixir_entity_extraction() {
        let code = r#"
defmodule MyApp.Accounts do
  def create_user(attrs) do
    %User{}
    |> User.changeset(attrs)
    |> Repo.insert()
  end

  defp validate(attrs) do
    # private helper
    :ok
  end

  defmacro is_admin(user) do
    quote do
      unquote(user).role == :admin
    end
  end

  defguard is_positive(x) when is_integer(x) and x > 0
end

defprotocol Printable do
  def to_string(data)
end

defimpl Printable, for: Integer do
  def to_string(i), do: Integer.to_string(i)
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "accounts.ex");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "Elixir entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"MyApp.Accounts"),
            "Should find module, got: {:?}",
            names
        );
        assert!(
            names.contains(&"create_user"),
            "Should find def, got: {:?}",
            names
        );
        assert!(
            names.contains(&"validate"),
            "Should find defp, got: {:?}",
            names
        );
        assert!(
            names.contains(&"is_admin"),
            "Should find defmacro, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Printable"),
            "Should find defprotocol, got: {:?}",
            names
        );

        // Verify nesting: create_user should have MyApp.Accounts as parent
        let create_user = entities.iter().find(|e| e.name == "create_user").unwrap();
        assert!(
            create_user.parent_id.is_some(),
            "create_user should be nested under module"
        );
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_entity_extraction() {
        let code = r#"
(ns my.app.core
  (:require [clojure.string :as str]))

(def my-var 42)

(def ^:private secret "hunter2")

(defonce connection (atom nil))

(defn greet
  "Returns a greeting string."
  [name]
  (str "Hello, " name "!"))

(defmacro unless [pred & body]
  `(when (not ~pred) ~@body))

(defprotocol Greeter
  (greet! [this name]))

(defrecord Person [name age])

(defmulti area :shape)

(defmethod area :circle [{:keys [radius]}]
  (* Math/PI radius radius))

(defmethod area :rectangle [{:keys [width height]}]
  (* width height))
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "core.clj");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Clojure entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        assert!(
            !names.contains(&"my.app.core"),
            "Should not extract ns form as entity, got: {:?}",
            names
        );
        assert!(
            names.contains(&"my-var"),
            "Should find def, got: {:?}",
            names
        );
        assert!(
            names.contains(&"secret"),
            "Should strip ^:private metadata from name, got: {:?}",
            names
        );
        assert!(
            names.contains(&"connection"),
            "Should find defonce, got: {:?}",
            names
        );
        assert!(
            names.contains(&"greet"),
            "Should find defn, got: {:?}",
            names
        );
        assert!(
            names.contains(&"unless"),
            "Should find defmacro, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Greeter"),
            "Should find defprotocol, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Person"),
            "Should find defrecord, got: {:?}",
            names
        );
        assert!(
            names.contains(&"area"),
            "Should find defmulti, got: {:?}",
            names
        );
        // defmethods get dispatch-qualified names so two methods on the same multimethod are distinct
        assert!(
            names.contains(&"area/:circle"),
            "Should find defmethod area :circle, got: {:?}",
            names
        );
        assert!(
            names.contains(&"area/:rectangle"),
            "Should find defmethod area :rectangle, got: {:?}",
            names
        );
        let ids: Vec<&str> = entities.iter().map(|e| e.id.as_str()).collect();
        assert!(
            ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len(),
            "All entity IDs must be unique, got: {:?}",
            ids
        );
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_defn_private() {
        let code = r#"
(ns my.app)

(defn- private-helper [x]
  (* x 2))
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "app.clj");
        let entity = entities
            .iter()
            .find(|e| e.name == "private-helper")
            .expect("Should extract defn- as a function entity");
        assert_eq!(entity.entity_type, "function");
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_predicate_and_bang_functions() {
        let code = r#"
(ns my.app.validators)

(defn empty? [coll]
  (= 0 (count coll)))

(defn reset! [state new-val]
  (compare-and-set! state @state new-val))
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "validators.clj");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"empty?"),
            "Should extract predicate fn empty?, got: {:?}",
            names
        );
        assert!(
            names.contains(&"reset!"),
            "Should extract bang fn reset!, got: {:?}",
            names
        );
        let empty_entity = entities.iter().find(|e| e.name == "empty?").unwrap();
        let reset_entity = entities.iter().find(|e| e.name == "reset!").unwrap();
        assert_eq!(empty_entity.entity_type, "function");
        assert_eq!(reset_entity.entity_type, "function");
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_dynamic_vars_and_equality_fns() {
        let code = r#"
(ns my.app.core)

(def *db* (atom nil))

(defn not= [a b]
  (not (= a b)))
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "core.clj");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"*db*"),
            "Should extract dynamic var *db*, got: {:?}",
            names
        );
        assert!(
            names.contains(&"not="),
            "Should extract fn not=, got: {:?}",
            names
        );
        let db_entity = entities.iter().find(|e| e.name == "*db*").unwrap();
        let noteq_entity = entities.iter().find(|e| e.name == "not=").unwrap();
        assert_eq!(db_entity.entity_type, "var");
        assert_eq!(noteq_entity.entity_type, "function");
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_deftype_definterface_defstruct() {
        let code = r#"
(ns my.app)

(deftype MyType [field])

(definterface IFoo
  (foo [this]))

(defstruct point :x :y)
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "app.clj");
        let by_name = |name: &str| entities.iter().find(|e| e.name == name);

        assert!(
            by_name("MyType").is_some(),
            "Should extract deftype, got: {:?}",
            entities.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
        assert_eq!(by_name("MyType").unwrap().entity_type, "type");

        assert!(
            by_name("IFoo").is_some(),
            "Should extract definterface, got: {:?}",
            entities.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
        assert_eq!(by_name("IFoo").unwrap().entity_type, "interface");

        assert!(
            by_name("point").is_some(),
            "Should extract defstruct, got: {:?}",
            entities.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
        assert_eq!(by_name("point").unwrap().entity_type, "struct");
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_cljc_extension() {
        let code = r#"
(ns my.app.shared)

(defn platform-key [] :default)

(def shared-value 99)
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "shared.cljc");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"platform-key"),
            "Should extract defn from .cljc, got: {:?}",
            names
        );
        assert!(
            names.contains(&"shared-value"),
            "Should extract def from .cljc, got: {:?}",
            names
        );
    }

    #[test]
    #[cfg(feature = "lang-clojure")]
    fn test_clojure_defmethod_non_keyword_dispatch() {
        let code = r#"
(ns my.app)

(defmulti process identity)

(defmethod process nil [_] :nothing)

(defmethod process "string" [s] s)

(defmethod process 42 [n] n)
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "app.clj");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"process"),
            "Should extract defmulti, got: {:?}",
            names
        );
        assert!(
            names.contains(&"process/nil"),
            "Should extract defmethod with nil dispatch, got: {:?}",
            names
        );
        assert!(
            names.contains(&"process/\"string\""),
            "Should extract defmethod with string dispatch, got: {:?}",
            names
        );
        assert!(
            names.contains(&"process/42"),
            "Should extract defmethod with integer dispatch, got: {:?}",
            names
        );
        let ids: Vec<&str> = entities.iter().map(|e| e.id.as_str()).collect();
        assert!(
            ids.iter().collect::<std::collections::HashSet<_>>().len() == ids.len(),
            "All entity IDs must be unique, got: {:?}",
            ids
        );
    }

    #[test]
    fn test_bash_entity_extraction() {
        let code = r#"#!/bin/bash

greet() {
    echo "Hello, $1!"
}

function deploy {
    echo "deploying..."
}

# not a function
echo "main script"
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "deploy.sh");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "Bash entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"greet"),
            "Should find greet(), got: {:?}",
            names
        );
        assert!(
            names.contains(&"deploy"),
            "Should find function deploy, got: {:?}",
            names
        );
        assert_eq!(
            entities.len(),
            2,
            "Should only find functions, got: {:?}",
            names
        );
    }

    #[test]
    fn test_entity_byte_offsets_slice_source_exactly() {
        // Byte offsets must let a consumer slice the exact original bytes of an
        // entity out of the source given only file_path + the span (#requested
        // by a sem-core user: pull exact content from git by file + entity id).
        let code =
            "import os\n\ndef first(a):\n    return a + 1\n\ndef second(b):\n    return b * 2\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "demo.py");
        let bytes = code.as_bytes();
        let funcs: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == "function")
            .collect();
        assert_eq!(funcs.len(), 2, "expected 2 functions, got {:?}", funcs);
        for e in funcs {
            let sb = e.start_byte.expect("function entity must carry start_byte");
            let eb = e.end_byte.expect("function entity must carry end_byte");
            let sliced = std::str::from_utf8(&bytes[sb..eb]).unwrap();
            assert!(
                sliced.starts_with(&format!("def {}", e.name)),
                "bytes[{sb}..{eb}] = {sliced:?} should be the body of {}",
                e.name
            );
        }
    }

    #[test]
    #[cfg(feature = "lang-lua")]
    fn test_lua_entity_extraction() {
        let code = r#"local M = {}

function greet(name)
    return "hello " .. name
end

local function helper(x)
    return x * 2
end

function M.compute(a, b)
    return helper(a) + helper(b)
end

function M:method(v)
    return v
end

return M
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "demo.lua");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        // global, local, table (dot) and method (colon) forms all extract
        assert!(
            names.contains(&"greet"),
            "global function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"helper"),
            "local function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"M.compute"),
            "table function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"M:method"),
            "method function, got: {:?}",
            names
        );
        assert_eq!(entities.len(), 4, "only functions, got: {:?}", names);
    }

    #[test]
    #[cfg(feature = "lang-bsl")]
    fn test_bsl_entity_extraction() {
        // BSL / 1C:Enterprise (issue #132). Procedures and functions are the
        // module-level entities; both normalize to the "function" type.
        let code = "Процедура ВывестиСообщение(Текст) Экспорт\n    Сообщить(Текст);\nКонецПроцедуры\n\nФункция Сложить(А, Б)\n    Возврат А + Б;\nКонецФункции\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "mod.bsl");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"ВывестиСообщение"),
            "procedure, got: {names:?}"
        );
        assert!(names.contains(&"Сложить"), "function, got: {names:?}");
        assert_eq!(entities.len(), 2, "two top-level entities, got: {names:?}");
        assert!(
            entities.iter().all(|e| e.entity_type == "function"),
            "procedures and functions normalize to `function`, got: {:?}",
            entities
                .iter()
                .map(|e| (e.name.as_str(), e.entity_type.as_str()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_entity_extraction() {
        // ABAP, as abapGit serializes a class (`.clas.abap`). The definition and
        // the implementation are separate top-level blocks of one class; the
        // METHOD blocks sit directly under the implementation and nest under
        // the class.
        let code = "CLASS zcl_demo DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS run.\n    METHODS helper IMPORTING iv_x TYPE i.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD run.\n    helper( 1 ).\n  ENDMETHOD.\n  METHOD helper.\n    \" a comment with helper( ) in it\n  ENDMETHOD.\nENDCLASS.\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "zcl_demo.clas.abap");
        let names: Vec<(&str, &str)> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.entity_type.as_str()))
            .collect();

        assert_eq!(
            names,
            vec![("zcl_demo", "class"), ("run", "method"), ("helper", "method")],
            "one class and its methods, no impl"
        );

        let class = &entities[0];
        assert_eq!(class.id, "zcl_demo.clas.abap::class::zcl_demo");
        assert_eq!((class.start_line, class.end_line), (1, 14));
        for method in &entities[1..] {
            assert_eq!(method.parent_id.as_deref(), Some(class.id.as_str()));
            assert_eq!(method.id, format!("{}::{}", class.id, method.name));
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_class_two_ranges() {
        // Spec 1.4: the class spans its definition and its implementation, and
        // records each block's lines. The text between them is blanked, so the
        // content keeps the file's lines and leaves the comment out.
        let code = "CLASS zcl_r DEFINITION.\n  PUBLIC SECTION.\n    METHODS run.\nENDCLASS.\n\n* between the blocks\n\nCLASS zcl_r IMPLEMENTATION.\n  METHOD run.\n  ENDMETHOD.\nENDCLASS.\n";
        let entities = CodeParserPlugin.extract_entities(code, "zcl_r.clas.abap");
        let class = &entities[0];
        assert_eq!((class.entity_type.as_str(), class.start_line, class.end_line), ("class", 1, 11));
        let metadata = class.metadata.as_ref().expect("ranges in metadata");
        assert_eq!(metadata.get("range.definition").map(String::as_str), Some("1-4"));
        assert_eq!(metadata.get("range.implementation").map(String::as_str), Some("8-11"));
        assert!(!class.content.contains("between"), "{:?}", class.content);
        assert_eq!(class.content.lines().count(), 11);
        assert_eq!(class.content.len(), code.trim_end().len());

        // The hash does not see the gap, nor which block comes first.
        let wider = code.replace("* between the blocks", "* another comment\n\n");
        let swapped = "CLASS zcl_r IMPLEMENTATION.\n  METHOD run.\n  ENDMETHOD.\nENDCLASS.\nCLASS zcl_r DEFINITION.\n  PUBLIC SECTION.\n    METHODS run.\nENDCLASS.\n";
        for other in [wider.as_str(), swapped] {
            let other = CodeParserPlugin.extract_entities(other, "zcl_r.clas.abap");
            assert_eq!(other[0].entity_type, "class");
            assert_eq!(other[0].content_hash, class.content_hash);
            assert_eq!(other[0].structural_hash, class.structural_hash);
        }

        // An edit to either block changes it.
        for edited in [
            code.replace("METHODS run.", "METHODS run IMPORTING iv TYPE i."),
            code.replace("  METHOD run.\n", "  METHOD run.\n    WRITE 1.\n"),
        ] {
            let edited = CodeParserPlugin.extract_entities(&edited, "zcl_r.clas.abap");
            assert_ne!(edited[0].content_hash, class.content_hash);
        }

        // A definition on its own is one range, and needs no metadata.
        let definition_only = CodeParserPlugin
            .extract_entities("CLASS zcl_a DEFINITION ABSTRACT.\nENDCLASS.\n", "zcl_a.clas.abap");
        assert_eq!(definition_only.len(), 1);
        assert_eq!(definition_only[0].entity_type, "class");
        assert!(definition_only[0].metadata.is_none());
    }

    // ---- ABAP fixture repository: tests/fixtures/abap/ ----
    //
    // One test per `.abap` fixture file, each asserting the exact entity list
    // (type, name, parent name) the spec expects, then `abap_fixture_<story>_<topic>`
    // tests for each story's behaviour. None is `#[ignore]`d: every story 1.1 to
    // 1.7 has landed. tests/fixtures/abap/README.md maps files to tests to stories.

    #[cfg(feature = "lang-abap")]
    fn abap_fixture_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/abap")
    }

    /// Extract a fixture file as (type, name, parent name) triples, in source order.
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_entities(file: &str) -> Vec<(String, String, Option<String>)> {
        let code = std::fs::read_to_string(abap_fixture_dir().join(file))
            .unwrap_or_else(|e| panic!("fixture {file}: {e}"));
        let entities = CodeParserPlugin.extract_entities(&code, file);
        eprintln!(
            "ABAP fixture {file}: {:?}",
            entities.iter().map(|e| (&e.entity_type, &e.name, &e.parent_id)).collect::<Vec<_>>()
        );
        entities
            .iter()
            .map(|e| {
                let parent = e.parent_id.as_ref().map(|pid| {
                    entities
                        .iter()
                        .find(|p| &p.id == pid)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| pid.clone())
                });
                (e.entity_type.clone(), e.name.clone(), parent)
            })
            .collect()
    }

    #[cfg(feature = "lang-abap")]
    fn abap_expect(rows: &[(&str, &str, Option<&str>)]) -> Vec<(String, String, Option<String>)> {
        rows.iter()
            .map(|(t, n, p)| (t.to_string(), n.to_string(), p.map(|s| s.to_string())))
            .collect()
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_intf() {
        assert_eq!(
            abap_fixture_entities("zif_fx_order.intf.abap"),
            abap_expect(&[("interface", "zif_fx_order", None)])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas() {
        assert_eq!(
            abap_fixture_entities("zcl_fx_order.clas.abap"),
            abap_expect(&[
                ("class", "zcl_fx_order", None),
                ("variable", "mv_id", Some("zcl_fx_order")),
                ("variable", "mt_names", Some("zcl_fx_order")),
                ("variable", "mv_total", Some("zcl_fx_order")),
                ("method", "constructor", Some("zcl_fx_order")),
                ("method", "create", Some("zcl_fx_order")),
                ("method", "describe", Some("zcl_fx_order")),
                ("method", "zif_fx_order~add_item", Some("zcl_fx_order")),
                ("method", "zif_fx_order~get_total", Some("zcl_fx_order")),
            ])
        );
    }

    /// Spec 1.3: a local or test class's parent is the global class of its file
    /// name, whose entity is in another file, so the parent shows as its id.
    #[cfg(feature = "lang-abap")]
    const ABAP_FIXTURE_GLOBAL_CLASS: &str = "zcl_fx_order.clas.abap::class::zcl_fx_order";

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas_locals_def() {
        assert_eq!(
            abap_fixture_entities("zcl_fx_order.clas.locals_def.abap"),
            abap_expect(&[("class", "lcl_helper", Some(ABAP_FIXTURE_GLOBAL_CLASS))])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas_locals_imp() {
        assert_eq!(
            abap_fixture_entities("zcl_fx_order.clas.locals_imp.abap"),
            abap_expect(&[
                ("class", "lcl_helper", Some(ABAP_FIXTURE_GLOBAL_CLASS)),
                ("method", "tag", Some("lcl_helper")),
            ])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas_testclasses() {
        assert_eq!(
            abap_fixture_entities("zcl_fx_order.clas.testclasses.abap"),
            abap_expect(&[
                ("class", "ltc_order", Some(ABAP_FIXTURE_GLOBAL_CLASS)),
                ("variable", "mo_cut", Some("ltc_order")),
                ("method", "setup", Some("ltc_order")),
                ("method", "total_starts_at_zero", Some("ltc_order")),
                ("method", "describe_mentions_id", Some("ltc_order")),
            ])
        );
    }

    // Assumed schema, to be confirmed with the spec: FOR TESTING methods are
    // reported as entity type `test_method`; `setup` stays a plain `method`.
    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas_testclasses_detection() {
        let file = "zcl_fx_order.clas.testclasses.abap";
        let code = std::fs::read_to_string(abap_fixture_dir().join(file))
            .unwrap_or_else(|e| panic!("fixture {file}: {e}"));
        let entities = CodeParserPlugin.extract_entities(&code, file);
        let tests: Vec<&str> = entities
            .iter()
            .filter(|e| crate::parser::graph::is_test_entity(e, &[]))
            .map(|e| e.name.as_str())
            .collect();
        assert!(tests.contains(&"ltc_order"), "got: {:?}", tests);
        for m in ["setup", "total_starts_at_zero", "describe_mentions_id"] {
            assert!(tests.contains(&m), "{m} not a test, got: {:?}", tests);
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_clas_sub() {
        assert_eq!(
            abap_fixture_entities("zcl_fx_order_sub.clas.abap"),
            abap_expect(&[
                ("class", "zcl_fx_order_sub", None),
                ("method", "describe", Some("zcl_fx_order_sub")),
            ])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_prog() {
        assert_eq!(
            abap_fixture_entities("zfx_report.prog.abap"),
            abap_expect(&[
                ("report", "zfx_report", None),
                ("macro", "_log", None),
                ("form", "show_order", None),
            ])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_prog_include() {
        assert_eq!(
            abap_fixture_entities("zfx_report_f01.prog.abap"),
            abap_expect(&[("form", "format_total", None)])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_fugr_function_module() {
        assert_eq!(
            abap_fixture_entities("zfx_fg.fugr.zfx_fm.abap"),
            abap_expect(&[("function", "zfx_fm", None)])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_fugr_main_program() {
        // Only INCLUDE statements: nothing to extract.
        assert_eq!(abap_fixture_entities("zfx_fg.fugr.saplzfx_fg.abap"), abap_expect(&[]));
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_fugr_top_include() {
        // Story 2.4 changes this from no entities. `DATA` outside a class is
        // not an entity on purpose (a program's or a form's `DATA` is a local
        // of the unit, `is_abap_local_data`), but a `TOP` include's top-level
        // `DATA` is the global data every module and form of the function group
        // reads, and `calc_extra`'s use of `gv_extra` needs an entity to point
        // at. `FUNCTION-POOL` is still not an entity.
        assert_eq!(
            abap_fixture_entities("zfx_fg.fugr.lzfx_fgtop.abap"),
            abap_expect(&[("variable", "gv_extra", None)])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_fugr_form_include() {
        assert_eq!(
            abap_fixture_entities("zfx_fg.fugr.lzfx_fgf01.abap"),
            abap_expect(&[("form", "calc_extra", None)])
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_fugr_pbo_include() {
        assert_eq!(
            abap_fixture_entities("zfx_fg.fugr.lzfx_fgo01.abap"),
            abap_expect(&[("module", "status_0100", None)])
        );
    }

    // Spec 1.3: the full entity set, each with its name and line range. The
    // fixture has no class-level TYPES and no PROGRAM statement, so those two
    // read small sources of their own.

    /// Extract `code` as (type, name, start line, end line) rows, in source order.
    #[cfg(feature = "lang-abap")]
    fn abap_rows(code: &str, file: &str) -> Vec<(String, String, usize, usize)> {
        let entities = CodeParserPlugin.extract_entities(code, file);
        eprintln!(
            "ABAP {file}: {:?}",
            entities
                .iter()
                .map(|e| (&e.entity_type, &e.name, e.start_line, e.end_line, &e.parent_id))
                .collect::<Vec<_>>()
        );
        entities
            .iter()
            .map(|e| (e.entity_type.clone(), e.name.clone(), e.start_line, e.end_line))
            .collect()
    }

    #[cfg(feature = "lang-abap")]
    fn abap_fixture_rows(file: &str) -> Vec<(String, String, usize, usize)> {
        abap_rows(&abap_fixture_text(file), file)
    }

    #[cfg(feature = "lang-abap")]
    fn abap_row(t: &str, n: &str, start: usize, end: usize) -> (String, String, usize, usize) {
        (t.to_string(), n.to_string(), start, end)
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_report() {
        let rows = abap_fixture_rows("zfx_report.prog.abap");
        assert!(rows.contains(&abap_row("report", "zfx_report", 2, 2)), "got: {rows:?}");

        // `PROGRAM` has no node of its own: it comes from the fallback, tagged.
        let entities = CodeParserPlugin.extract_entities("PROGRAM zfoo.\n\nWRITE 'x'.\n", "zfoo.prog.abap");
        assert_eq!(entities.len(), 1, "got: {entities:?}");
        assert_eq!((entities[0].entity_type.as_str(), entities[0].name.as_str()), ("report", "zfoo"));
        assert_eq!(entities[0].content, "PROGRAM zfoo.");
        assert_eq!(
            entities[0].metadata.as_ref().and_then(|m| m.get("source")).map(String::as_str),
            Some("abap-fallback")
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_form() {
        assert_eq!(
            abap_fixture_rows("zfx_report_f01.prog.abap"),
            vec![abap_row("form", "format_total", 5, 7)]
        );
        assert_eq!(
            abap_fixture_rows("zfx_fg.fugr.lzfx_fgf01.abap"),
            vec![abap_row("form", "calc_extra", 5, 7)]
        );
        // The grammar folds this ENDFORM into an ERROR with the body before it.
        let code = abap_fixture_text("zfx_report.prog.abap");
        let entities = CodeParserPlugin.extract_entities(&code, "zfx_report.prog.abap");
        let form = entities.iter().find(|e| e.entity_type == "form").expect("form");
        assert_eq!((form.name.as_str(), form.start_line, form.end_line), ("show_order", 21, 24));
        assert!(form.content.starts_with("FORM show_order USING"), "{}", form.content);
        assert!(form.content.ends_with("ENDFORM."), "{}", form.content);

        // Error recovery can also swallow the next FORM and a MODULE whole, and
        // the keywords can be lower case.
        let code = "form a.\n  DATA(x) = 1.\nendform.\nFORM b.\nENDFORM.\nMODULE user_command_0100 INPUT.\n  CASE sy-ucomm.\n  ENDCASE.\nENDMODULE.\n";
        assert_eq!(
            abap_rows(code, "zfoo.prog.abap"),
            vec![
                abap_row("form", "a", 1, 3),
                abap_row("form", "b", 4, 5),
                abap_row("module", "user_command_0100", 6, 9),
            ]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_function() {
        assert_eq!(
            abap_fixture_rows("zfx_fg.fugr.zfx_fm.abap"),
            vec![abap_row("function", "zfx_fm", 1, 14)]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_module() {
        assert_eq!(
            abap_fixture_rows("zfx_fg.fugr.lzfx_fgo01.abap"),
            vec![abap_row("module", "status_0100", 5, 8)]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_method() {
        let entities = abap_fixture_entities("zcl_fx_order.clas.abap");
        let methods: Vec<_> = entities.iter().filter(|(t, _, _)| t == "method").collect();
        assert_eq!(
            methods
                .iter()
                .map(|(_, n, p)| (n.as_str(), p.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("constructor", Some("zcl_fx_order")),
                ("create", Some("zcl_fx_order")),
                ("describe", Some("zcl_fx_order")),
                ("zif_fx_order~add_item", Some("zcl_fx_order")),
                ("zif_fx_order~get_total", Some("zcl_fx_order")),
            ]
        );
        let rows = abap_fixture_rows("zcl_fx_order.clas.abap");
        assert!(rows.contains(&abap_row("method", "constructor", 20, 22)), "got: {rows:?}");
        assert!(rows.contains(&abap_row("method", "zif_fx_order~get_total", 38, 42)), "got: {rows:?}");
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_class() {
        let rows = abap_fixture_rows("zcl_fx_order.clas.abap");
        // Spec 1.4: one class from the definition's start to the implementation's end.
        assert!(rows.contains(&abap_row("class", "zcl_fx_order", 2, 44)), "got: {rows:?}");
        assert!(!rows.iter().any(|(t, _, _, _)| t == "impl"), "got: {rows:?}");
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_interface() {
        assert_eq!(
            abap_fixture_rows("zif_fx_order.intf.abap"),
            vec![abap_row("interface", "zif_fx_order", 1, 14)]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_1_8_interface_name_is_not_a_later_token() {
        // Found by the Gate 1 census (PR #4432, `zif_abapgit_popups`): a pragma
        // after the first statement put the interface name in an ERROR and the
        // grammar's `name` field on `NO_TEXT`. 48 of abapGit's 113 global
        // interfaces were named after a later token this way.
        let code = "INTERFACE zif_demo\n  PUBLIC .\n\n  CONSTANTS c_label TYPE string VALUE 'x' ##NO_TEXT.\n\n  METHODS run.\nENDINTERFACE.\n";
        assert_eq!(
            abap_rows(code, "zif_demo.intf.abap"),
            vec![abap_row("interface", "zif_demo", 1, 7)]
        );

        // The same without the pragma parses cleanly and keeps its name.
        let code = "INTERFACE zif_demo\n  PUBLIC .\n\n  CONSTANTS c_label TYPE string VALUE 'x'.\n\n  METHODS run.\nENDINTERFACE.\n";
        assert_eq!(
            abap_rows(code, "zif_demo.intf.abap"),
            vec![abap_row("interface", "zif_demo", 1, 7)]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_types_data() {
        // TYPES has no node (an ERROR, or the tail of the METHODS or DATA before
        // it); DATA is a variable_declaration, and an entity only in a section.
        let code = "\
CLASS zcl_t DEFINITION PUBLIC.
  PUBLIC SECTION.
    TYPES ty_id TYPE i.
    TYPES: BEGIN OF ty_s,
             a TYPE i,
           END OF ty_s.
    METHODS run.
    TYPES: ty_a TYPE i,
           ty_b TYPE string.
    DATA mv_x TYPE i.
    types ty_c type i.
  PRIVATE SECTION.
    TYPES BEGIN OF ty_old.
    TYPES   f TYPE i.
    TYPES END OF ty_old.
    DATA mv_y TYPE ty_s.
ENDCLASS.

CLASS zcl_t IMPLEMENTATION.
  METHOD run.
    TYPES ty_local TYPE i.
    DATA lv_local TYPE ty_local.
  ENDMETHOD.
ENDCLASS.

TYPES ty_global TYPE i.
DATA gv_global TYPE i.
";
        let rows = abap_rows(code, "zcl_t.clas.abap");
        assert_eq!(
            rows,
            vec![
                abap_row("class", "zcl_t", 1, 24),
                abap_row("type", "ty_id", 3, 3),
                abap_row("type", "ty_s", 4, 6),
                abap_row("type", "ty_a", 8, 8),
                abap_row("type", "ty_b", 9, 9),
                abap_row("variable", "mv_x", 10, 10),
                abap_row("type", "ty_c", 11, 11),
                abap_row("type", "ty_old", 13, 15),
                abap_row("variable", "mv_y", 16, 16),
                abap_row("method", "run", 20, 23),
            ]
        );
        let entities = CodeParserPlugin.extract_entities(code, "zcl_t.clas.abap");
        let class_id = &entities[0].id;
        for e in entities.iter().filter(|e| matches!(e.entity_type.as_str(), "type" | "variable")) {
            assert_eq!(e.parent_id.as_ref(), Some(class_id), "{}", e.name);
        }
        let by_name = |n: &str| entities.iter().find(|e| e.name == n).unwrap();
        // The grammar folds `types ty_c ...` into mv_x's node; each keeps its own text.
        assert_eq!(by_name("mv_x").content, "DATA mv_x TYPE i.");
        assert_eq!(by_name("ty_c").content, "types ty_c type i.");
        assert_eq!(by_name("ty_a").content, "TYPES: ty_a TYPE i,");
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_macro() {
        let rows = abap_fixture_rows("zfx_report.prog.abap");
        assert!(rows.contains(&abap_row("macro", "_log", 6, 8)), "got: {rows:?}");

        // A macro defined inside a form nests under it.
        let code = "FORM f.\n  DEFINE m.\n    WRITE &1.\n  END-OF-DEFINITION.\n  m 'x'.\nENDFORM.\n";
        let entities = CodeParserPlugin.extract_entities(code, "zfoo.prog.abap");
        assert_eq!(
            abap_rows(code, "zfoo.prog.abap"),
            vec![abap_row("form", "f", 1, 6), abap_row("macro", "m", 2, 4)]
        );
        assert_eq!(entities[1].parent_id.as_ref(), Some(&entities[0].id));
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_3_local_classes_attach() {
        for file in [
            "zcl_fx_order.clas.locals_def.abap",
            "zcl_fx_order.clas.locals_imp.abap",
            "zcl_fx_order.clas.testclasses.abap",
        ] {
            let code = abap_fixture_text(file);
            let entities = CodeParserPlugin.extract_entities(&code, &format!("src/{file}"));
            let top: Vec<_> = entities
                .iter()
                .filter(|e| matches!(e.entity_type.as_str(), "class" | "impl"))
                .collect();
            assert!(!top.is_empty(), "{file}");
            for e in top {
                assert_eq!(
                    e.parent_id.as_deref(),
                    Some("src/zcl_fx_order.clas.abap::class::zcl_fx_order"),
                    "{file}: {}",
                    e.name
                );
                // The id itself is unchanged: it carries the file, so a local
                // class's definition and implementation in two files stay apart.
                assert_eq!(e.id, format!("src/{file}::{}::{}", e.entity_type, e.name));
            }
        }
        // The global class's own file attaches nothing.
        let entities = CodeParserPlugin
            .extract_entities(&abap_fixture_text("zcl_fx_order.clas.abap"), "src/zcl_fx_order.clas.abap");
        assert!(entities.iter().filter(|e| e.entity_type == "class").all(|e| e.parent_id.is_none()));
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn test_abap_fixture_layout() {
        // abapGit layout (spec 1.5): every `<name>.<type>.abap` of a clas/intf/prog
        // object has its `.xml` envelope next to it, every file stays under 60
        // lines, and every fixture file follows `<name>.<type>[.<part>].<ext>`.
        let dir = abap_fixture_dir();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            // The directory's README is documentation, not an abapGit object.
            if entry.path().is_dir() || name == "README.md" {
                continue;
            }
            let text = std::fs::read_to_string(entry.path()).unwrap();
            assert!(text.lines().count() < 60, "{name} has 60+ lines");
            let parts: Vec<&str> = name.split('.').collect();
            assert!(parts.len() >= 3, "{name}: not <name>.<type>.<ext>");
            if name.ends_with(".abap") && matches!(parts[1], "clas" | "intf" | "prog") && parts.len() == 3 {
                let xml = dir.join(format!("{}.{}.xml", parts[0], parts[1]));
                assert!(xml.exists(), "{name}: missing abapGit .xml envelope");
            }
            if name.ends_with(".xml") {
                assert!(text.contains("<abapGit version=\"v1.0.0\""), "{name}: no abapGit envelope");
            }
        }
    }

    /// Names each ABAP fixture entity in `file` depends on in the reference graph, by entity
    /// name: (entity name, sorted dependency names).
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_dependencies(files: &[&str]) -> Vec<(String, Vec<String>)> {
        let dir = tempfile::TempDir::new().unwrap();
        for file in files {
            std::fs::copy(abap_fixture_dir().join(file), dir.path().join(file)).unwrap();
        }
        let registry = crate::parser::plugins::create_default_registry();
        let file_paths: Vec<String> = files.iter().map(|f| f.to_string()).collect();
        let (graph, _) = crate::parser::graph::EntityGraph::build(dir.path(), &file_paths, &registry);
        let mut rows: Vec<(String, Vec<String>)> = graph
            .entities
            .iter()
            .map(|(id, entity)| {
                let mut deps: Vec<String> =
                    graph.get_dependencies(id).iter().map(|d| d.name.clone()).collect();
                deps.sort();
                (entity.name.clone(), deps)
            })
            .collect();
        eprintln!("ABAP fixture dependencies {files:?}: {rows:?}");
        rows.sort();
        rows
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_2_comment_not_reference() {
        // Spec 1.2: the trailing `" ... describe( ) ..."` comment in `constructor`
        // names a method of the same class, and a comment is not a reference.
        let rows = abap_fixture_dependencies(&["zcl_fx_order.clas.abap"]);
        let (_, deps) = rows.iter().find(|(name, _)| name == "constructor").expect("constructor");
        assert!(!deps.contains(&"describe".to_string()), "got: {:?}", deps);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_2_string_not_reference() {
        // Spec 1.2: the `'calls get_total( ) and describe( ) in a literal'` literal
        // is not a reference, while the `{ mv_id }` expression inside `describe`'s
        // template is (`mv_id` appears nowhere else in `describe`).
        let rows = abap_fixture_dependencies(&["zif_fx_order.intf.abap", "zcl_fx_order.clas.abap"]);
        let (_, deps) = rows
            .iter()
            .find(|(name, _)| name == "zif_fx_order~get_total")
            .expect("zif_fx_order~get_total");
        assert!(!deps.contains(&"describe".to_string()), "got: {:?}", deps);
        let (_, deps) = rows.iter().find(|(name, _)| name == "describe").expect("describe");
        assert!(deps.contains(&"mv_id".to_string()), "got: {:?}", deps);
    }

    // Spec 1.1: ABAP names are case-insensitive. The fixture is all lowercase;
    // these tests read it as written and in case-shifted copies.

    #[cfg(feature = "lang-abap")]
    fn abap_fixture_text(file: &str) -> String {
        std::fs::read_to_string(abap_fixture_dir().join(file))
            .unwrap_or_else(|e| panic!("fixture {file}: {e}"))
    }

    /// Build the entity graph of `(file name, content)` pairs in a scratch directory.
    #[cfg(feature = "lang-abap")]
    fn abap_graph(files: &[(&str, String)]) -> crate::parser::graph::EntityGraph {
        let dir = tempfile::TempDir::new().unwrap();
        for (name, content) in files {
            std::fs::write(dir.path().join(name), content).unwrap();
        }
        let paths: Vec<String> = files.iter().map(|(name, _)| name.to_string()).collect();
        let registry = crate::parser::plugins::create_default_registry();
        crate::parser::graph::EntityGraph::build(dir.path(), &paths, &registry).0
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_1_find_any_case() {
        // `sem find` resolves a name through the index's `lookup`: every
        // spelling answers the same entities in the same order, named as
        // written in source. A Python name next to them stays exact.
        let mut files: Vec<(&str, String)> = [
            "zcl_fx_order.clas.abap",
            "zcl_fx_order_sub.clas.abap",
            "zif_fx_order.intf.abap",
        ]
        .iter()
        .map(|file| (*file, abap_fixture_text(file)))
        .collect();
        files.push(("order.py", "class Order:\n    pass\n".to_string()));
        let graph = abap_graph(&files);
        let index = crate::index::QueryIndex::from_bytes(crate::index::build(&graph, &[])).unwrap();
        let hits = |name: &str| -> Vec<(String, String)> {
            index
                .lookup(name)
                .iter()
                .map(|e| (e.id(), e.name().to_string()))
                .collect()
        };

        let lower = hits("zcl_fx_order");
        assert!(!lower.is_empty());
        assert!(
            lower.iter().all(|(_, name)| name == "zcl_fx_order"),
            "name as written, got: {lower:?}"
        );
        assert_eq!(hits("ZCL_FX_ORDER"), lower);
        assert_eq!(hits("Zcl_Fx_Order"), lower);

        // Types still filter: `method DESCRIBE` is both `describe` methods.
        let methods: Vec<String> = index
            .lookup("DESCRIBE")
            .iter()
            .filter(|e| e.entity_type() == "method")
            .map(|e| e.id())
            .collect();
        assert_eq!(methods.len(), 2, "got: {methods:?}");

        assert_eq!(hits("Order").len(), 1);
        assert!(hits("order").is_empty(), "Python names stay case-sensitive");
        assert!(hits("ORDER").is_empty(), "Python names stay case-sensitive");

        // Source written in uppercase: any spelling finds it, shown uppercase.
        let upper = abap_graph(&[(
            "zcl_fx_order.clas.abap",
            abap_fixture_text("zcl_fx_order.clas.abap").to_ascii_uppercase(),
        )]);
        let index = crate::index::QueryIndex::from_bytes(crate::index::build(&upper, &[])).unwrap();
        for query in ["zcl_fx_order", "ZCL_FX_ORDER", "Zcl_Fx_Order"] {
            let names: Vec<&str> = index.lookup(query).iter().map(|e| e.name()).collect();
            assert!(!names.is_empty(), "{query}");
            assert!(names.iter().all(|name| *name == "ZCL_FX_ORDER"), "{query}: {names:?}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_1_refs_any_case() {
        // The class definition declares the attributes (`DATA mv_id ...`) and
        // the implementation's methods use them (`mv_id = iv_id.`). Shifting
        // the case of either half must give back the same edges.
        let source = abap_fixture_text("zcl_fx_order.clas.abap");
        let (definition, implementation) =
            source.split_at(source.find("CLASS zcl_fx_order IMPLEMENTATION").unwrap());
        let edges = |content: String| -> Vec<(String, String)> {
            let graph = abap_graph(&[("zcl_fx_order.clas.abap", content)]);
            let name = |id: &str| graph.entities[id].name.to_ascii_lowercase();
            let mut pairs: Vec<(String, String)> = graph
                .edges
                .iter()
                .map(|edge| (name(edge.from_entity.as_str()), name(edge.to_entity.as_str())))
                .collect();
            pairs.sort();
            pairs
        };

        let as_written = edges(source.clone());
        assert!(
            as_written.contains(&("constructor".to_string(), "mv_id".to_string())),
            "got: {as_written:?}"
        );
        // Uppercase use, lowercase definition.
        assert_eq!(
            edges(definition.to_ascii_uppercase() + implementation),
            as_written
        );
        // Lowercase use, uppercase definition.
        assert_eq!(
            edges(definition.to_string() + &implementation.to_ascii_uppercase()),
            as_written
        );
    }

    // Spec 1.6: a file the grammar cannot fully parse still yields what it can,
    // and says how much it could not read.

    /// `zcl_fx_order.clas.abap` with one deliberate break each: an `ENDMETHOD`
    /// missing, a string literal never closed, modern syntax the grammar rejects.
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_broken_copies() -> Vec<(&'static str, String)> {
        let clean = abap_fixture_text("zcl_fx_order.clas.abap");
        let broken = |from: &str, to: &str| {
            assert!(clean.contains(from), "fixture changed: {from}");
            clean.replacen(from, to, 1)
        };
        vec![
            (
                "missing ENDMETHOD",
                broken("  ENDMETHOD.\n\n  METHOD create.", "\n  METHOD create."),
            ),
            (
                "unterminated string",
                broken(
                    "'calls get_total( ) and describe( ) in a literal'",
                    "'calls get_total( ) and describe( ) in a literal",
                ),
            ),
            (
                "modern syntax",
                broken(
                    "ro_order = NEW #( iv_id = iv_id ).",
                    "ro_order = COND #( WHEN iv_id > 0 THEN NEW #( iv_id = iv_id ) ELSE THROW zcx_x( ) ).\n    DATA(lt) = REDUCE i( INIT s = 0 FOR w IN mt_names NEXT s = s + 1 ).\n    FINAL(x) = SWITCH #( iv_id WHEN 1 THEN `a` ).",
                ),
            ),
        ]
    }

    #[cfg(feature = "lang-abap")]
    fn abap_stats(code: &str, file: &str) -> crate::parser::plugin::ParseStats {
        let stats = CodeParserPlugin.parse_stats(code, file);
        eprintln!("ABAP {file}: {stats:?}");
        stats
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_6_errors_still_yield_entities() {
        // The method bodies hold ERROR nodes (and, before the fork's grammar
        // read them, `FOR TESTING` and `DURATION SHORT RISK LEVEL` in the
        // definition did too); the class and every method are still there.
        let rows = abap_fixture_rows("zcl_fx_order.clas.testclasses.abap");
        let names = |kind: &str| -> Vec<&str> {
            rows.iter().filter(|r| r.0 == kind).map(|r| r.1.as_str()).collect()
        };
        assert!(names("class").contains(&"ltc_order"), "got: {rows:?}");
        assert_eq!(
            names("method"),
            vec!["setup", "total_starts_at_zero", "describe_mentions_id"],
            "got: {rows:?}"
        );

        // An ERROR swallows ENDFORM and the grammar has no `form` node at all.
        let rows = abap_fixture_rows("zfx_report.prog.abap");
        assert!(rows.contains(&abap_row("report", "zfx_report", 2, 2)), "got: {rows:?}");
        assert!(rows.contains(&abap_row("form", "show_order", 21, 24)), "got: {rows:?}");

        // Broken copies of the global class keep the class and every method that
        // has its ENDMETHOD, the ones the parse loses included (story 1.10). The
        // grammar reads the method after a missing `ENDMETHOD` as part of the one
        // before it (`create` inside `constructor`); `create` is its own again,
        // and `constructor`, which has no ENDMETHOD, is dropped, as a FORM is.
        // The unterminated literal ends at its line, as ABAP's do, so the
        // `get_total` the grammar loses after it is there.
        let expected: [(&str, &[&str]); 3] = [
            (
                "missing ENDMETHOD",
                &["create", "describe", "zif_fx_order~add_item", "zif_fx_order~get_total"],
            ),
            (
                "unterminated string",
                &["constructor", "create", "describe", "zif_fx_order~add_item", "zif_fx_order~get_total"],
            ),
            (
                "modern syntax",
                &["constructor", "create", "describe", "zif_fx_order~add_item", "zif_fx_order~get_total"],
            ),
        ];
        for ((label, code), (expected_label, methods)) in
            abap_fixture_broken_copies().iter().zip(expected)
        {
            assert_eq!(*label, expected_label);
            let rows = abap_rows(code, "zcl_fx_order.clas.abap");
            assert!(
                rows.iter().any(|r| r.0 == "class" && r.1 == "zcl_fx_order"),
                "{label}: class, got: {rows:?}"
            );
            let found: Vec<&str> = rows.iter().filter(|r| r.0 == "method").map(|r| r.1.as_str()).collect();
            assert_eq!(found, methods, "{label}: methods, got: {rows:?}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_6_error_count_reported() {
        // A file the grammar reads whole has no error nodes: one class (its two
        // blocks collapsed) and one method.
        let clean = "CLASS zcl_demo DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS run.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD run.\n    WRITE 'x'.\n  ENDMETHOD.\nENDCLASS.\n";
        let stats = abap_stats(clean, "zcl_demo.clas.abap");
        assert_eq!((stats.entity_count, stats.fallback_entity_count, stats.error_node_count), (2, 0, 0));

        // The fixture report has error nodes, and its form and report are the
        // fallback's: counted apart from the grammar's own entities.
        let stats = abap_stats(&abap_fixture_text("zfx_report.prog.abap"), "zfx_report.prog.abap");
        assert!(stats.error_node_count > 0, "got: {stats:?}");
        assert_eq!(
            (stats.entity_count, stats.grammar_entity_count, stats.fallback_entity_count),
            (3, 1, 2)
        );

        // Each break adds error nodes over the unbroken fixture, and entities
        // are never counted twice.
        let fixture = abap_stats(&abap_fixture_text("zcl_fx_order.clas.abap"), "zcl_fx_order.clas.abap");
        assert_eq!(fixture.entity_count, fixture.grammar_entity_count + fixture.fallback_entity_count);
        for (label, code) in abap_fixture_broken_copies() {
            if label == "missing ENDMETHOD" {
                continue; // read as a call to a macro named METHOD: no error node
            }
            let stats = abap_stats(&code, "zcl_fx_order.clas.abap");
            assert!(stats.error_node_count > fixture.error_node_count, "{label}: {stats:?}");
            assert_eq!(stats.entity_count, stats.grammar_entity_count + stats.fallback_entity_count);
        }

        // A file with nothing extractable is still counted, as zero.
        let stats = abap_stats("WRITE 'never closed.\n", "zbroken.prog.abap");
        assert_eq!(stats.entity_count, 0);
        assert!(stats.error_node_count > 0, "got: {stats:?}");
    }

    // Story 1.10: METHOD blocks the grammar's error recovery loses or stretches
    // are read off the statements, like FORM.

    /// The `source` metadata of each method entity, by name: `None` for the
    /// grammar's own, `Some("abap-fallback")` for a recovered one.
    #[cfg(feature = "lang-abap")]
    fn abap_method_sources(code: &str, file: &str) -> Vec<(String, Option<String>)> {
        CodeParserPlugin
            .extract_entities(code, file)
            .iter()
            .filter(|e| e.entity_type == "method")
            .map(|e| (e.name.clone(), e.metadata.as_ref().and_then(|m| m.get("source").cloned())))
            .collect()
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_10_unparseable_statement_keeps_later_methods() {
        // Found by the Gate 1 census (#1928, #4432, #5072, ...): a statement the
        // grammar cannot parse loses every method after it; here the whole
        // implementation is. The census cases were mostly a `''` the grammar
        // read on over many lines, which the fork's grammar now ends at its line
        // (see `abap_grammar_ends_literals_at_end_of_line`); this body, cut down
        // from abapGit's zcl_abapgit_xml, still loses it. Both methods come
        // back, each with its own range, under the class.
        let code = "CLASS zcl_demo DEFINITION PUBLIC.\n  PUBLIC SECTION.\n    METHODS one.\n    METHODS two.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD one.\n    li_element = mi_xml_doc->find_from_name_ns( depth = 0\n                                                name = c_abapgit_tag ).\n    IF li_element IS NOT BOUND.\n    ENDIF.\n    li_version = li_element->if_ixml_node~get_attributes(\n      )->get_named_item_ns( c_attr_version ).\n    IF li_version->get_value( ) <> zif_abapgit_version=>c_xml_version.\n    ENDIF.\n  ENDMETHOD.\n\n  METHOD two.\n    WRITE 'y'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.abap"),
            vec![
                abap_row("class", "zcl_demo", 1, 22),
                abap_row("method", "one", 8, 17),
                abap_row("method", "two", 19, 21),
            ]
        );
        let entities = CodeParserPlugin.extract_entities(code, "zcl_demo.clas.abap");
        for method in entities.iter().filter(|e| e.entity_type == "method") {
            assert_eq!(method.parent_id.as_deref(), Some("zcl_demo.clas.abap::class::zcl_demo"));
            assert!(method.content.starts_with("  METHOD "), "{}", method.content);
            assert!(method.content.ends_with("ENDMETHOD."), "{}", method.content);
        }

        // The statement in a later method: the grammar keeps the methods before
        // it, which stay its own, and the fallback adds the rest.
        let code = "CLASS zcl_demo DEFINITION PUBLIC.\n  PUBLIC SECTION.\n    METHODS one.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD one.\n    WRITE 'x'.\n  ENDMETHOD.\n\n  METHOD two.\n    li_element = mi_xml_doc->find_from_name_ns( depth = 0\n                                                name = c_abapgit_tag ).\n    IF li_element IS NOT BOUND.\n    ENDIF.\n    li_version = li_element->if_ixml_node~get_attributes(\n      )->get_named_item_ns( c_attr_version ).\n    IF li_version->get_value( ) <> zif_abapgit_version=>c_xml_version.\n    ENDIF.\n  ENDMETHOD.\n\n  METHOD three.\n    WRITE 'z'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_method_sources(code, "zcl_demo.clas.abap"),
            vec![
                ("one".to_string(), None),
                ("two".to_string(), Some("abap-fallback".to_string())),
                ("three".to_string(), Some("abap-fallback".to_string())),
            ]
        );
        assert!(
            abap_rows(code, "zcl_demo.clas.abap").contains(&abap_row("class", "zcl_demo", 1, 25))
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_10_stretched_method_ends_at_its_endmethod() {
        // Found by the Gate 1 census (#3185, #3891, #7644): a statement the
        // grammar misreads runs `one`'s node over `two`, which is lost. In the
        // census it was mostly a template like `|{ a }*|`, which the fork's
        // grammar now reads as one token; here it is a `*` the grammar takes for
        // a comment to the end of the line, period included. `one` is cut back
        // to its own ENDMETHOD and stays the grammar's; `two` is its own entity
        // again.
        let code = "CLASS zcl_demo DEFINITION PUBLIC.\n  PUBLIC SECTION.\n    METHODS one.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD one.\n    lv = lines( lt ) * 2.\n  ENDMETHOD.\n\n  METHOD two.\n    WRITE 'y'.\n  ENDMETHOD.\n\n  METHOD three.\n    WRITE 'z'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.abap"),
            vec![
                abap_row("class", "zcl_demo", 1, 18),
                abap_row("method", "one", 7, 9),
                abap_row("method", "two", 11, 13),
                abap_row("method", "three", 15, 17),
            ]
        );
        let entities = CodeParserPlugin.extract_entities(code, "zcl_demo.clas.abap");
        let one = entities.iter().find(|e| e.name == "one").expect("one");
        assert_eq!(one.content, "  METHOD one.\n    lv = lines( lt ) * 2.\n  ENDMETHOD.");
        assert_eq!(one.end_byte, Some(code.find("ENDMETHOD.").unwrap() + "ENDMETHOD.".len()));
        assert!(one.metadata.is_none(), "trimmed, still the grammar's: {:?}", one.metadata);
        assert_eq!(
            abap_method_sources(code, "zcl_demo.clas.abap")[1],
            ("two".to_string(), Some("abap-fallback".to_string()))
        );

        // Trimmed or recovered, the method's content is what a clean parse gives,
        // so a diff across the two reads it as unchanged.
        let clean = code.replace("lv = lines( lt ) * 2.", "lv = a.");
        let stretched = code.replace("    WRITE 'y'.", "    lv = lines( lt ) * 2.");
        let content = |code: &str, name: &str| {
            CodeParserPlugin
                .extract_entities(code, "zcl_demo.clas.abap")
                .into_iter()
                .find(|e| e.name == name)
                .map(|e| (e.content, e.content_hash))
                .expect(name)
        };
        assert_eq!(content(code, "two"), content(&clean, "two"));
        assert_eq!(content(&stretched, "three"), content(&clean, "three"));
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_10_method_without_endmethod_dropped() {
        // A METHOD with no ENDMETHOD is dropped, as a FORM is; the methods before
        // it are intact. Here the grammar loses the implementation too.
        let code = "CLASS lcl_demo IMPLEMENTATION.\n  METHOD one.\n    WRITE 'x'.\n  ENDMETHOD.\n  METHOD two.\n    WRITE 'y'.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.locals_imp.abap"),
            vec![abap_row("class", "lcl_demo", 1, 7), abap_row("method", "one", 2, 4)]
        );

        // Before another METHOD, the grammar reads it on into that one (`two`
        // over L5-10); it is dropped there too, and `three` is its own.
        let code = "CLASS lcl_demo IMPLEMENTATION.\n  METHOD one.\n    WRITE 'x'.\n  ENDMETHOD.\n  METHOD two.\n    WRITE 'y'.\n\n  METHOD three.\n    WRITE 'z'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.locals_imp.abap"),
            vec![
                abap_row("class", "lcl_demo", 1, 11),
                abap_row("method", "one", 2, 4),
                abap_row("method", "three", 8, 10),
            ]
        );

        // A METHOD in a comment or a literal is not a block.
        let code = "CLASS lcl_demo IMPLEMENTATION.\n  METHOD one.\n* METHOD fake.\n    lv = 'METHOD fake. ENDMETHOD.'. \" ENDMETHOD.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.locals_imp.abap"),
            vec![abap_row("class", "lcl_demo", 1, 6), abap_row("method", "one", 2, 5)]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_1_10_interface_method_names() {
        // `zif_x~m` is one name: the grammar's `zif_x~m` is cut back to its
        // ENDMETHOD, and the recovered `ZIF_X~N` keeps its spelling.
        let code = "CLASS lcl_demo IMPLEMENTATION.\n  METHOD zif_x~m.\n    lv = lines( lt ) * 2.\n  ENDMETHOD.\n  METHOD ZIF_X~N.\n    WRITE 'y'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.locals_imp.abap"),
            vec![
                abap_row("class", "lcl_demo", 1, 8),
                abap_row("method", "zif_x~m", 2, 4),
                abap_row("method", "ZIF_X~N", 5, 7),
            ]
        );
        assert_eq!(
            abap_method_sources(code, "zcl_demo.clas.locals_imp.abap"),
            vec![
                ("zif_x~m".to_string(), None),
                ("ZIF_X~N".to_string(), Some("abap-fallback".to_string())),
            ]
        );
    }

    // Spec 2.0: a global name reaches across files, a local one stays in its object.

    /// The story 2.0 fixture objects, with the Tier 1 ones they call.
    #[cfg(feature = "lang-abap")]
    const ABAP_FIXTURE_2_0_FILES: &[&str] = &[
        "zif_fx_order.intf.abap",
        "zcl_fx_order.clas.abap",
        "zcl_fx_order.clas.locals_def.abap",
        "zcl_fx_order.clas.locals_imp.abap",
        "zcl_fx_order.clas.testclasses.abap",
        "zcl_fx_order_sub.clas.abap",
        "zcl_fx_user.clas.abap",
        "zcl_fx_other.clas.abap",
        "zcl_fx_other.clas.locals_imp.abap",
        "zcl_fx_other.clas.testclasses.abap",
    ];

    /// An ABAP entity as `object.name`, folded (`zcl_fx_order.create`), or the
    /// object name alone for the global class or interface itself.
    #[cfg(feature = "lang-abap")]
    fn abap_label(graph: &crate::parser::graph::EntityGraph, id: &str) -> String {
        let entity = &graph.entities[id];
        abap_object_label(&entity.file_path, &entity.name)
    }

    /// The `abap_label` of an entity named `name` in `file_path`.
    #[cfg(feature = "lang-abap")]
    fn abap_object_label(file_path: &str, name: &str) -> String {
        let object = abap_name::parse_abapgit_name(file_path)
            .map(|o| o.name.to_ascii_lowercase())
            .unwrap_or_default();
        let name = name.to_ascii_lowercase();
        if name == object {
            object
        } else {
            format!("{object}.{name}")
        }
    }

    /// Every edge of `graph` as sorted (from, to) labels, see `abap_label`.
    #[cfg(feature = "lang-abap")]
    fn abap_edges(graph: &crate::parser::graph::EntityGraph) -> Vec<(String, String)> {
        let mut edges: Vec<(String, String)> = graph
            .edges
            .iter()
            .map(|edge| {
                (
                    abap_label(graph, edge.from_entity.as_str()),
                    abap_label(graph, edge.to_entity.as_str()),
                )
            })
            .collect();
        edges.sort();
        edges.dedup();
        eprintln!("ABAP edges: {edges:?}");
        edges
    }

    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_edges() -> Vec<(String, String)> {
        let files: Vec<(&str, String)> = ABAP_FIXTURE_2_0_FILES
            .iter()
            .map(|file| (*file, abap_fixture_text(file)))
            .collect();
        abap_edges(&abap_graph(&files))
    }

    #[cfg(feature = "lang-abap")]
    fn abap_edge(from: &str, to: &str) -> (String, String) {
        (from.to_string(), to.to_string())
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_global_class_across_files() {
        // `ZCL_FX_ORDER=>CREATE( 1 )` in zcl_fx_user names a class and a method
        // defined in another file, in upper case.
        let edges = abap_fixture_2_0_edges();
        assert!(edges.contains(&abap_edge("zcl_fx_user.run", "zcl_fx_order")), "got: {edges:?}");
        assert!(
            edges.contains(&abap_edge("zcl_fx_user.run", "zcl_fx_order.create")),
            "got: {edges:?}"
        );

        // Whatever the case of either spelling.
        let shifted = |user: String, order: String| {
            let mut files: Vec<(&str, String)> = ABAP_FIXTURE_2_0_FILES
                .iter()
                .filter(|file| !matches!(**file, "zcl_fx_user.clas.abap" | "zcl_fx_order.clas.abap"))
                .map(|file| (*file, abap_fixture_text(file)))
                .collect();
            files.push(("zcl_fx_user.clas.abap", user));
            files.push(("zcl_fx_order.clas.abap", order));
            abap_edges(&abap_graph(&files))
        };
        let user = abap_fixture_text("zcl_fx_user.clas.abap");
        let order = abap_fixture_text("zcl_fx_order.clas.abap");
        assert_eq!(shifted(user.to_ascii_lowercase(), order.to_ascii_uppercase()), edges);
        assert_eq!(shifted(user.clone(), order.to_ascii_uppercase()), edges);
        assert_eq!(shifted(user.to_ascii_lowercase(), order), edges);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_local_parts_keep_definition_references() {
        // A global class's local classes are its children from other files. Their
        // line numbers must not be cut out of the class's own definition lines,
        // or `TYPE REF TO` / `INTERFACES` / parameter types there lose their edges.
        // A class with a one-line child (its `DATA`) is scanned through content
        // spans, which already stay in the file, so drop the attributes: the
        // class then has only methods as children and is scanned by line range,
        // as an abstract class of local parts is. The definition then has
        // references of its own: `INTERFACES zif_fx_order`, and one more nothing
        // else makes, a `METHODS` parameter typed `zcl_fx_user`.
        let class_edges = |names: &[&str]| {
            let files: Vec<(&str, String)> = names
                .iter()
                .map(|file| {
                    let text = abap_fixture_text(file);
                    if *file != "zcl_fx_order.clas.abap" {
                        return (*file, text);
                    }
                    let text: String = text
                        .lines()
                        .filter(|line| !line.trim_start().starts_with("DATA m"))
                        .map(|line| {
                            if line.trim_start().starts_with("METHODS describe") {
                                format!(
                                    "    METHODS adopt IMPORTING io_user TYPE REF TO zcl_fx_user.\n{line}\n"
                                )
                            } else {
                                format!("{line}\n")
                            }
                        })
                        .collect();
                    assert!(text.contains("io_user") && !text.contains("DATA m"));
                    (*file, text)
                })
                .collect();
            let graph = abap_graph(&files);
            let mut edges: Vec<(String, String, String)> = graph
                .edges
                .iter()
                .filter(|edge| abap_label(&graph, edge.from_entity.as_str()) == "zcl_fx_order")
                .map(|edge| {
                    (
                        abap_label(&graph, edge.from_entity.as_str()),
                        abap_label(&graph, edge.to_entity.as_str()),
                        format!("{:?}", edge.ref_type),
                    )
                })
                .collect();
            edges.sort();
            edges.dedup();
            eprintln!("zcl_fx_order edges: {edges:?}");
            edges
        };
        let alone = class_edges(&[
            "zcl_fx_order.clas.abap",
            "zif_fx_order.intf.abap",
            "zcl_fx_user.clas.abap",
        ]);
        let with_parts = class_edges(&[
            "zcl_fx_order.clas.abap",
            "zif_fx_order.intf.abap",
            "zcl_fx_user.clas.abap",
            "zcl_fx_order.clas.locals_def.abap",
            "zcl_fx_order.clas.locals_imp.abap",
            "zcl_fx_order.clas.testclasses.abap",
        ]);
        assert!(
            alone.iter().any(|(_, to, _)| to == "zif_fx_order"),
            "definition lost its interface reference: {alone:?}"
        );
        assert!(
            alone.iter().any(|(_, to, _)| to == "zcl_fx_user"),
            "definition lost its parameter type reference: {alone:?}"
        );
        assert_eq!(with_parts, alone);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_local_class_stays_in_object() {
        // zcl_fx_order and zcl_fx_other each have a local `lcl_helper` with a
        // `tag` method. Each object's use reaches its own, in its other files.
        let edges = abap_fixture_2_0_edges();
        for (from, object) in [("zcl_fx_order.describe", "zcl_fx_order"), ("zcl_fx_other.label", "zcl_fx_other")] {
            let other = if object == "zcl_fx_order" { "zcl_fx_other" } else { "zcl_fx_order" };
            assert!(edges.contains(&abap_edge(from, &format!("{object}.lcl_helper"))), "{from}: {edges:?}");
            assert!(edges.contains(&abap_edge(from, &format!("{object}.tag"))), "{from}: {edges:?}");
            assert!(
                !edges.iter().any(|(f, to)| f == from && to.starts_with(&format!("{other}."))),
                "{from} leaks into {other}: {edges:?}"
            );
        }
        // No other object reaches either local class.
        for (from, to) in &edges {
            if to.ends_with(".lcl_helper") || to.ends_with(".tag") {
                assert_eq!(from.split('.').next(), to.split('.').next(), "{from} -> {to}");
            }
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_ambiguous_method_no_edge() {
        // `describe` is defined in zcl_fx_order, zcl_fx_order_sub and zcl_fx_user.
        // `io_order->describe( )` in zcl_fx_other resolves to none of them.
        let edges = abap_fixture_2_0_edges();
        assert!(
            !edges.iter().any(|(from, to)| from == "zcl_fx_other.label" && to.ends_with(".describe")),
            "got: {edges:?}"
        );
        // Inside an object the name is its own: zcl_fx_order's test class
        // reaches zcl_fx_order's `describe` only.
        let describes: Vec<&String> = edges
            .iter()
            .filter(|(from, to)| from == "zcl_fx_order.describe_mentions_id" && to.ends_with(".describe"))
            .map(|(_, to)| to)
            .collect();
        assert_eq!(describes, vec!["zcl_fx_order.describe"]);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_unique_method_across_files() {
        // `label` is defined once, in zcl_fx_other: zcl_fx_user's call reaches it.
        let edges = abap_fixture_2_0_edges();
        assert!(edges.contains(&abap_edge("zcl_fx_user.run", "zcl_fx_other.label")), "got: {edges:?}");
        // An interface component is one name though written in two tokens.
        assert!(
            edges.contains(&abap_edge("zcl_fx_other.total", "zcl_fx_order.zif_fx_order~get_total")),
            "got: {edges:?}"
        );
        // A name defined in the object itself needs no uniqueness: zcl_fx_order's
        // test class reaches `create` and `describe` in its global class.
        assert!(
            edges.contains(&abap_edge("zcl_fx_order.setup", "zcl_fx_order.create")),
            "got: {edges:?}"
        );
        assert!(
            edges.contains(&abap_edge("zcl_fx_order.describe_mentions_id", "zcl_fx_order.describe")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_local_friends_does_not_shadow() {
        // `CLASS zcl_fx_other DEFINITION LOCAL FRIENDS ltc_other.` in the test
        // classes names the global class. Story 1.3 makes no entity of it (nor
        // of `DEFINITION DEFERRED`): the file holds `ltc_other` and its method.
        // Were it a class, its `testclasses` part would keep it in the object.
        let files: Vec<(&str, String)> = ABAP_FIXTURE_2_0_FILES
            .iter()
            .map(|file| (*file, abap_fixture_text(file)))
            .collect();
        let graph = abap_graph(&files);
        let mut in_file: Vec<(&str, &str)> = graph
            .entities
            .values()
            .filter(|e| e.file_path == "zcl_fx_other.clas.testclasses.abap")
            .map(|e| (e.entity_type.as_str(), e.name.as_str()))
            .collect();
        in_file.sort();
        assert_eq!(in_file, vec![("class", "ltc_other"), ("method", "label_has_tag")]);

        // Readers in the test file and in another object both reach the
        // global class in `zcl_fx_other.clas.abap`.
        for reader in ["label_has_tag", "run"] {
            let targets: Vec<&str> = graph
                .edges
                .iter()
                .filter(|edge| graph.entities[edge.from_entity.as_str()].name == reader)
                .map(|edge| edge.to_entity.as_str())
                .filter(|id| graph.entities[*id].name == "zcl_fx_other")
                .collect();
            assert_eq!(targets, vec!["zcl_fx_other.clas.abap::class::zcl_fx_other"], "{reader}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_0_incremental_follows_other_files() {
        // A reader in zcl_c stays clean while zcl_a and zcl_b gain and lose
        // `ping`. Each step, an incremental rebuild (both the cached-graph path
        // and the red-green session) must give the edges a fresh build gives.
        let class = |name: &str, method: &str, body: &str| {
            format!(
                "CLASS {name} DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS {method}.\nENDCLASS.\n\n\nCLASS {name} IMPLEMENTATION.\n\n  METHOD {method}.\n    {body}\n  ENDMETHOD.\n\nENDCLASS.\n"
            )
        };
        let dir = tempfile::TempDir::new().unwrap();
        let write = |file: &str, content: String| std::fs::write(dir.path().join(file), content).unwrap();
        let files: Vec<String> = ["zcl_a.clas.abap", "zcl_b.clas.abap", "zcl_c.clas.abap"]
            .iter()
            .map(|f| f.to_string())
            .collect();
        let registry = crate::parser::plugins::create_default_registry();
        let build = || crate::parser::graph::EntityGraph::build(dir.path(), &files, &registry);
        let go_targets = |graph: &crate::parser::graph::EntityGraph| -> Vec<String> {
            let mut targets: Vec<String> = abap_edges(graph)
                .into_iter()
                .filter(|(from, _)| from == "zcl_c.go")
                .map(|(_, to)| to)
                .collect();
            targets.sort();
            targets
        };

        write("zcl_a.clas.abap", class("zcl_a", "ping", "WRITE 'a'."));
        write("zcl_b.clas.abap", class("zcl_b", "ping", "WRITE 'b'."));
        write("zcl_c.clas.abap", class("zcl_c", "go", "lo_x->ping( )."));
        let (mut graph, mut entities) = build();
        let mut session = crate::parser::session::GraphSession::build(dir.path(), &files, &registry);
        assert!(go_targets(&graph).is_empty(), "`ping` is defined twice");

        // (changed file, its new content, the targets of `go` afterwards)
        let steps = [
            // zcl_b loses `ping`: the name is unique, `go` gains an edge.
            ("zcl_b.clas.abap", class("zcl_b", "pong", "WRITE 'b'."), vec!["zcl_a.ping"]),
            // zcl_b defines it again: ambiguous, the edge goes.
            ("zcl_b.clas.abap", class("zcl_b", "ping", "WRITE 'b'."), vec![]),
            // zcl_a loses it: unique again, now in zcl_b.
            ("zcl_a.clas.abap", class("zcl_a", "pong", "WRITE 'a'."), vec!["zcl_b.ping"]),
        ];
        for (file, content, expected) in steps {
            write(file, content);
            let (stale, clean): (Vec<_>, Vec<_>) =
                entities.into_iter().partition(|e| e.file_path == file);
            let (incremental, next) = crate::parser::graph::EntityGraph::build_incremental(
                dir.path(),
                &[file.to_string()],
                &files,
                clean,
                graph.edges,
                stale,
                &registry,
            );
            session.rebuild(&files, &[file.to_string()], &registry);
            let fresh = build().0;
            assert_eq!(go_targets(&fresh), expected, "after {file}");
            assert_eq!(abap_edges(&incremental), abap_edges(&fresh), "build_incremental, after {file}");
            assert_eq!(abap_edges(session.graph()), abap_edges(&fresh), "session, after {file}");
            graph = incremental;
            entities = next;
        }
    }

    // Story 2.4: a program and its includes, and a function group, are one
    // compiled unit; a form is visible in its unit and nowhere else.

    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_edges(files: &[&str]) -> Vec<(String, String)> {
        let files: Vec<(&str, String)> =
            files.iter().map(|file| (*file, abap_fixture_text(file))).collect();
        abap_edges(&abap_graph(&files))
    }

    #[cfg(feature = "lang-abap")]
    const ABAP_FIXTURE_2_4_PROGRAMS: [&str; 4] = [
        "zfx_report.prog.abap",
        "zfx_report_f01.prog.abap",
        "zfx_report2.prog.abap",
        "zfx_other.prog.abap",
    ];

    #[cfg(feature = "lang-abap")]
    const ABAP_FIXTURE_2_4_FUGR: [&str; 5] = [
        "zfx_fg.fugr.zfx_fm.abap",
        "zfx_fg.fugr.saplzfx_fg.abap",
        "zfx_fg.fugr.lzfx_fgtop.abap",
        "zfx_fg.fugr.lzfx_fgf01.abap",
        "zfx_fg.fugr.lzfx_fgo01.abap",
    ];

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_include_joins_forms() {
        // `PERFORM format_total` in zfx_report2 reaches the form of
        // zfx_report_f01, an object of its own that zfx_report2 includes.
        let edges = abap_fixture_2_4_edges(&["zfx_report2.prog.abap", "zfx_report_f01.prog.abap"]);
        assert!(
            edges.contains(&abap_edge("zfx_report2.run_report", "zfx_report_f01.format_total")),
            "got: {edges:?}"
        );

        // Without the `INCLUDE` line the same text reaches nothing.
        let without = abap_fixture_text("zfx_report2.prog.abap").replace("INCLUDE zfx_report_f01.", "");
        let files = [
            ("zfx_report2.prog.abap", without),
            ("zfx_report_f01.prog.abap", abap_fixture_text("zfx_report_f01.prog.abap")),
        ];
        let edges = abap_edges(&abap_graph(&files));
        assert!(
            !edges.contains(&abap_edge("zfx_report2.run_report", "zfx_report_f01.format_total")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_include_sees_main_and_siblings() {
        // The unit is symmetric, as compilation is: an include sees the main
        // program's forms, and a sibling include's.
        let main = "REPORT zmain.\nINCLUDE zmain_f01.\nINCLUDE zmain_f02.\n\nFORM main_form.\n  WRITE 'm'.\nENDFORM.\n";
        let f01 = "FORM first_form.\n  PERFORM main_form.\n  PERFORM second_form.\nENDFORM.\n";
        let f02 = "FORM second_form.\n  WRITE 's'.\nENDFORM.\n";
        let edges = abap_edges(&abap_graph(&[
            ("zmain.prog.abap", main.to_string()),
            ("zmain_f01.prog.abap", f01.to_string()),
            ("zmain_f02.prog.abap", f02.to_string()),
        ]));
        assert!(edges.contains(&abap_edge("zmain_f01.first_form", "zmain.main_form")), "got: {edges:?}");
        assert!(edges.contains(&abap_edge("zmain_f01.first_form", "zmain_f02.second_form")), "got: {edges:?}");
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_forms_do_not_leak() {
        // zfx_other includes nothing. Its `show_order` is its own, and the
        // form of zfx_report_f01 is out of its reach.
        let edges = abap_fixture_2_4_edges(&ABAP_FIXTURE_2_4_PROGRAMS);
        let from_other: Vec<&(String, String)> =
            edges.iter().filter(|(from, _)| from == "zfx_other.run_own").collect();
        assert_eq!(from_other, [&abap_edge("zfx_other.run_own", "zfx_other.show_order")]);
        assert!(
            !edges.iter().any(|(from, to)| from == "zfx_other.run_leak" && to == "zfx_report_f01.format_total"),
            "got: {edges:?}"
        );
        assert!(
            !edges.iter().any(|(from, to)| from.starts_with("zfx_other.") && to == "zfx_report.show_order"
                && from != "zfx_other.run_remote"),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_in_program_target() {
        // `PERFORM show_order IN PROGRAM zfx_report` in zfx_other goes to
        // zfx_report's form, never to zfx_other's own, which ABAP would not
        // call. `IN PROGRAM (iv_name)` is not resolved: it is a dynamic site
        // (story 2.5). Asked of the calls pipeline alone: the bag-of-words
        // pass, which still adds its own guesses to the graph (the bare name
        // `show_order` bound to zfx_other's own form, `zfx_report` to the
        // report) until `replaces_bow()` flips with story 2.2's typed
        // receivers, so the graph's edge list cannot say "only".
        let (sites, _) = abap_pipeline_calls(&ABAP_FIXTURE_2_4_PROGRAMS);
        let line = |needle: &str| {
            abap_fixture_text("zfx_other.prog.abap")
                .lines()
                .position(|l| l.contains(needle))
                .expect(needle)
                + 1
        };
        let answers = |needle: &str| abap_answers(&sites, "zfx_other.prog.abap", line(needle));
        assert_eq!(answers("IN PROGRAM zfx_report"), ["zfx_report.show_order"]);
        assert_eq!(answers("IN PROGRAM (iv_name)"), ["unknown: dynamic form name"]);
        assert_eq!(answers("PERFORM show_order USING iv_id."), ["zfx_other.show_order"]);
        let edges = abap_fixture_2_4_edges(&ABAP_FIXTURE_2_4_PROGRAMS);
        assert!(
            edges.contains(&abap_edge("zfx_other.run_remote", "zfx_report.show_order")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_5_dynamic_sites_are_unresolved_with_reasons() {
        // Every computed call of zfx_dynamic is one site answered `unknown`
        // with its form's reason, and none is an edge.
        let (sites, stats) = abap_pipeline_calls(&["zfx_dynamic.prog.abap"]);
        let reasons: Vec<&str> = sites
            .iter()
            .filter_map(|(_, _, a)| a.strip_prefix("unknown: dynamic "))
            .collect();
        for form in ["method name", "function name", "form name", "class name"] {
            assert!(reasons.contains(&form), "{form}: {sites:?}");
        }
        let counted: usize = stats.unresolved.iter().filter(|(why, _)| why.starts_with("dynamic ")).map(|(_, n)| *n).sum();
        assert_eq!(counted, reasons.len(), "{:?}", stats.unresolved);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_fugr_perform() {
        // `PERFORM calc_extra` in the module zfx_fm reaches the form in
        // `lzfx_fgf01`, one part of the same function group.
        let edges = abap_fixture_2_4_edges(&ABAP_FIXTURE_2_4_FUGR);
        assert!(edges.contains(&abap_edge("zfx_fg.zfx_fm", "zfx_fg.calc_extra")), "got: {edges:?}");
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_fugr_global_data() {
        // `calc_extra` reads `gv_extra`, declared by `DATA` in `lzfx_fgtop`.
        let files: Vec<(&str, String)> =
            ABAP_FIXTURE_2_4_FUGR.iter().map(|f| (*f, abap_fixture_text(f))).collect();
        let graph = abap_graph(&files);
        let found = graph.edges.iter().any(|edge| {
            abap_label(&graph, edge.from_entity.as_str()) == "zfx_fg.calc_extra"
                && abap_label(&graph, edge.to_entity.as_str()) == "zfx_fg.gv_extra"
                && edge.ref_type == crate::parser::graph::RefType::TypeRef
        });
        assert!(found, "got: {:?}", abap_edges(&graph));
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_missing_include_recorded() {
        // The generated `uxx` include is not in abapGit: recorded with a
        // reason, not dropped and not an error. The other three resolve.
        let dir = tempfile::TempDir::new().unwrap();
        for file in ABAP_FIXTURE_2_4_FUGR {
            std::fs::copy(abap_fixture_dir().join(file), dir.path().join(file)).unwrap();
        }
        let files: Vec<String> = ABAP_FIXTURE_2_4_FUGR.iter().map(|f| f.to_string()).collect();
        let graph = abap_include::IncludeGraph::build(dir.path(), &files);
        assert_eq!(
            graph.unresolved,
            [abap_include::UnresolvedInclude {
                file: "zfx_fg.fugr.saplzfx_fg.abap".into(),
                include: "lzfx_fguxx".into()
            }]
        );
        assert_eq!(graph.unresolved[0].reason(), "include not in repo");
        let mut included: Vec<&str> = graph.edges.iter().map(|(_, to)| to.as_str()).collect();
        included.sort();
        assert_eq!(
            included,
            ["zfx_fg.fugr.lzfx_fgf01.abap", "zfx_fg.fugr.lzfx_fgo01.abap", "zfx_fg.fugr.lzfx_fgtop.abap"]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_include_structure_is_not_include() {
        // `INCLUDE STRUCTURE` and `INCLUDE TYPE` inside `DATA` and `TYPES` are
        // statements that start with the same keyword, and link no file.
        let code = "REPORT zfx_structs.\n\nDATA: BEGIN OF gs_order.\n  INCLUDE STRUCTURE zfx_order.\nDATA: END OF gs_order.\n\nTYPES: BEGIN OF ty_order.\n  INCLUDE TYPE zfx_order.\nTYPES: END OF ty_order.\n\nINCLUDE zfx_structs_f01.\n";
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("zfx_structs.prog.abap"), code).unwrap();
        let files = vec!["zfx_structs.prog.abap".to_string(), "zfx_order.tabl.xml".to_string()];
        let graph = abap_include::IncludeGraph::build(dir.path(), &files);
        // Only the real include, and it names no file here.
        assert!(graph.edges.is_empty());
        assert_eq!(
            graph.unresolved,
            [abap_include::UnresolvedInclude {
                file: "zfx_structs.prog.abap".into(),
                include: "zfx_structs_f01".into()
            }]
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_top_include_data_only_in_top() {
        // The exception is the `TOP` include. Any other program's top-level
        // `DATA` stays out of the entity list (`test_abap_fixture_prog`).
        let rows = abap_fixture_entities("zfx_fg.fugr.lzfx_fgtop.abap");
        assert_eq!(rows, abap_expect(&[("variable", "gv_extra", None)]));
        let program = "REPORT zfx_data.\nDATA gv_global TYPE i.\nFORM f.\n  DATA lv_local TYPE i.\nENDFORM.\n";
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("zfx_data.prog.abap"), program).unwrap();
        let top = "FUNCTION-POOL zfx_fg.\nDATA gv_top TYPE i.\nFORM g.\n  DATA lv_local TYPE i.\nENDFORM.\n";
        std::fs::write(dir.path().join("zfx_fg.fugr.lzfx_fgtop.abap"), top).unwrap();
        let graph = abap_graph(&[
            ("zfx_data.prog.abap", program.to_string()),
            ("zfx_fg.fugr.lzfx_fgtop.abap", top.to_string()),
        ]);
        let mut variables: Vec<String> = graph
            .entities
            .values()
            .filter(|e| e.entity_type == "variable")
            .map(|e| e.name.clone())
            .collect();
        variables.sort();
        assert_eq!(variables, ["gv_top"]);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_4_incremental_follows_include() {
        // zinc's `other` calls `sib_form`, a form of zsib. Both stay clean while
        // zmain, which has no entity to change, gains `INCLUDE zinc.` and
        // `INCLUDE zsib.` and joins them into one unit. The edge must appear,
        // and go again with the lines, in a cached-graph rebuild and in the
        // red-green session, exactly as a fresh build has it.
        let main = |includes: &str| format!("REPORT zmain.\n{includes}\nFORM main_form.\n  WRITE 'm'.\nENDFORM.\n");
        let dir = tempfile::TempDir::new().unwrap();
        let write = |file: &str, content: &str| std::fs::write(dir.path().join(file), content).unwrap();
        let files: Vec<String> = ["zmain.prog.abap", "zinc.prog.abap", "zsib.prog.abap"]
            .iter()
            .map(|f| f.to_string())
            .collect();
        let registry = crate::parser::plugins::create_default_registry();
        let build = || crate::parser::graph::EntityGraph::build(dir.path(), &files, &registry);
        let reaches = |graph: &crate::parser::graph::EntityGraph| {
            abap_edges(graph).contains(&abap_edge("zinc.other", "zsib.sib_form"))
        };

        write("zmain.prog.abap", &main(""));
        write("zinc.prog.abap", "FORM other.\n  PERFORM sib_form.\nENDFORM.\n");
        write("zsib.prog.abap", "FORM sib_form.\n  WRITE 's'.\nENDFORM.\n");
        let (mut graph, mut entities) = build();
        let mut session = crate::parser::session::GraphSession::build(dir.path(), &files, &registry);
        assert!(!reaches(&graph));

        for (includes, expected) in [("INCLUDE zinc.\nINCLUDE zsib.", true), ("", false)] {
            let file = "zmain.prog.abap";
            write(file, &main(includes));
            let (stale, clean): (Vec<_>, Vec<_>) =
                entities.into_iter().partition(|e| e.file_path == file);
            let (incremental, next) = crate::parser::graph::EntityGraph::build_incremental(
                dir.path(),
                &[file.to_string()],
                &files,
                clean,
                graph.edges,
                stale,
                &registry,
            );
            session.rebuild(&files, &[file.to_string()], &registry);
            let fresh = build().0;
            assert_eq!(reaches(&fresh), expected, "fresh, `{includes}`");
            assert_eq!(abap_edges(&incremental), abap_edges(&fresh), "build_incremental, `{includes}`");
            assert_eq!(abap_edges(session.graph()), abap_edges(&fresh), "session, `{includes}`");
            graph = incremental;
            entities = next;
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_keyword_is_not_a_unique_name() {
        // `CREATE PUBLIC` in a class definition is a keyword, not a call of
        // `zcl_fx_order.create`, the repo's one `create`: no class reaches it.
        // `run`'s `ZCL_FX_ORDER=>CREATE( 1 )` writes it as a call and still does.
        let edges = abap_fixture_2_0_edges();
        for class in ["zcl_fx_user", "zcl_fx_other", "zcl_fx_order_sub"] {
            assert!(!edges.contains(&abap_edge(class, "zcl_fx_order.create")), "{class}: {edges:?}");
        }
        assert!(edges.contains(&abap_edge("zcl_fx_user.run", "zcl_fx_order.create")), "got: {edges:?}");

        // A keyword written as a call through an untyped receiver still binds
        // by the unique name, in any case; one written only as a keyword, in
        // the method's own lines, does not.
        let class = |name: &str, body: &str| {
            format!(
                "CLASS {name} DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS create.\nENDCLASS.\n\n\nCLASS {name} IMPLEMENTATION.\n\n  METHOD create.\n    {body}\n  ENDMETHOD.\n\nENDCLASS.\n"
            )
        };
        let user = |name: &str, body: &str| {
            format!(
                "CLASS {name} DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    METHODS go.\nENDCLASS.\n\n\nCLASS {name} IMPLEMENTATION.\n\n  METHOD go.\n    {body}\n  ENDMETHOD.\n\nENDCLASS.\n"
            )
        };
        let graph = abap_graph(&[
            ("zcl_a.clas.abap", class("zcl_a", "WRITE 'a'.")),
            ("zcl_b.clas.abap", user("zcl_b", "lo_a->CREATE( ).")),
            ("zcl_c.clas.abap", user("zcl_c", "CREATE OBJECT lo_a.")),
        ]);
        let edges = abap_edges(&graph);
        assert!(edges.contains(&abap_edge("zcl_b.go", "zcl_a.create")), "got: {edges:?}");
        assert!(
            !edges.iter().any(|(from, to)| from.starts_with("zcl_c") && to == "zcl_a.create"),
            "got: {edges:?}"
        );
    }

    // Spec 2.1: a call written in a static form resolves exactly, through the
    // calls pipeline, beside the bag-of-words resolver until receivers are typed.

    /// The story 2.1 fixture objects: 2.0's, `zcl_fx_calls`, and the report and
    /// function group that call by `PERFORM` and `CALL FUNCTION`.
    #[cfg(feature = "lang-abap")]
    const ABAP_FIXTURE_2_1_FILES: &[&str] = &[
        "zif_fx_order.intf.abap",
        "zcl_fx_order.clas.abap",
        "zcl_fx_order.clas.locals_def.abap",
        "zcl_fx_order.clas.locals_imp.abap",
        "zcl_fx_order.clas.testclasses.abap",
        "zcl_fx_order_sub.clas.abap",
        "zcl_fx_user.clas.abap",
        "zcl_fx_other.clas.abap",
        "zcl_fx_other.clas.locals_imp.abap",
        "zcl_fx_other.clas.testclasses.abap",
        "zcl_fx_calls.clas.abap",
        "zfx_report.prog.abap",
        "zfx_fg.fugr.zfx_fm.abap",
        "zfx_fg.fugr.lzfx_fgf01.abap",
    ];

    /// One call site of the calls pipeline: its file, its 1-based line, and its
    /// answer, the callees' `abap_label`s, `external` or `unknown: <reason>`.
    #[cfg(feature = "lang-abap")]
    type AbapSite = (String, usize, String);

    /// Every call site of the story 2.1 fixture as the calls pipeline answers
    /// it alone, and the pipeline's counts.
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_calls() -> (Vec<AbapSite>, crate::parser::calls::Stats) {
        use crate::parser::calls::{self, SiteAnswer};
        let registry = crate::parser::plugins::create_default_registry();
        let sources: Vec<(&str, String)> = ABAP_FIXTURE_2_1_FILES
            .iter()
            .map(|file| (*file, abap_fixture_text(file)))
            .collect();
        let entities: Vec<SemanticEntity> = sources
            .iter()
            .flat_map(|(file, src)| registry.extract_entities(file, src))
            .collect();
        let labels: HashMap<&str, String> = entities
            .iter()
            .map(|e| (e.id.as_str(), abap_object_label(&e.file_path, &e.name)))
            .collect();
        let facts: Vec<(&str, calls::ir::FileFacts)> = sources
            .iter()
            .map(|(file, src)| (*file, calls::lower_source(file, src).expect("an ABAP file")))
            .collect();
        let files: Vec<(&str, &calls::ir::FileFacts)> =
            facts.iter().map(|(file, f)| (*file, f)).collect();
        let lang = calls::language_for("zcl_fx_calls.clas.abap").expect("ABAP is in LANGUAGES");
        let root = std::path::Path::new("/nonexistent");
        let mut sites = Vec::new();
        for ((file, src), answers) in sources.iter().zip(calls::site_answers(root, lang, &files, &entities)) {
            for (at, call, answer) in answers {
                if !call {
                    continue;
                }
                let answer = match answer {
                    SiteAnswer::Defs(ids) => {
                        ids.iter().map(|id| labels[id.as_str()].as_str()).collect::<Vec<_>>().join(" ")
                    }
                    SiteAnswer::Value(..) => "value".to_string(),
                    SiteAnswer::External(_) => "external".to_string(),
                    SiteAnswer::Unknown(why) => format!("unknown: {why}"),
                };
                let line = src[..at as usize].matches('\n').count() + 1;
                sites.push((file.to_string(), line, answer));
            }
        }
        let (_, stats) = calls::resolve(root, lang, &files, &entities);
        (sites, stats)
    }

    /// Every call site of `files` (fixture names) as the calls pipeline
    /// answers it alone, and the pipeline's counts.
    #[cfg(feature = "lang-abap")]
    fn abap_pipeline_calls(files: &[&str]) -> (Vec<AbapSite>, crate::parser::calls::Stats) {
        use crate::parser::calls::{self, SiteAnswer};
        let registry = crate::parser::plugins::create_default_registry();
        let sources: Vec<(&str, String)> =
            files.iter().map(|file| (*file, abap_fixture_text(file))).collect();
        let entities: Vec<SemanticEntity> = sources
            .iter()
            .flat_map(|(file, src)| registry.extract_entities(file, src))
            .collect();
        let labels: HashMap<&str, String> = entities
            .iter()
            .map(|e| (e.id.as_str(), abap_object_label(&e.file_path, &e.name)))
            .collect();
        let facts: Vec<(&str, calls::ir::FileFacts)> = sources
            .iter()
            .map(|(file, src)| (*file, calls::lower_source(file, src).expect("an ABAP file")))
            .collect();
        let refs: Vec<(&str, &calls::ir::FileFacts)> =
            facts.iter().map(|(file, f)| (*file, f)).collect();
        let lang = calls::language_for("x.prog.abap").expect("ABAP is in LANGUAGES");
        let root = std::path::Path::new("/nonexistent");
        let mut sites = Vec::new();
        for ((file, src), answers) in sources.iter().zip(calls::site_answers(root, lang, &refs, &entities)) {
            for (at, call, answer) in answers {
                if !call {
                    continue;
                }
                let answer = match answer {
                    SiteAnswer::Defs(ids) => {
                        ids.iter().map(|id| labels[id.as_str()].as_str()).collect::<Vec<_>>().join(" ")
                    }
                    SiteAnswer::Value(..) => "value".to_string(),
                    SiteAnswer::External(_) => "external".to_string(),
                    SiteAnswer::Unknown(why) => format!("unknown: {why}"),
                };
                let line = src[..at as usize].matches('\n').count() + 1;
                sites.push((file.to_string(), line, answer));
            }
        }
        let (_, stats) = calls::resolve(root, lang, &refs, &entities);
        (sites, stats)
    }

    /// The answers of the call sites on `line` of `file`, in source order.
    #[cfg(feature = "lang-abap")]
    fn abap_answers<'s>(sites: &'s [AbapSite], file: &str, line: usize) -> Vec<&'s str> {
        sites
            .iter()
            .filter(|(f, l, _)| f == file && *l == line)
            .map(|(_, _, answer)| answer.as_str())
            .collect()
    }

    /// Every edge of the story 2.1 fixture's graph as (from, to, kind), by
    /// `abap_label`.
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_edges() -> Vec<(String, String, &'static str)> {
        let files: Vec<(&str, String)> = ABAP_FIXTURE_2_1_FILES
            .iter()
            .map(|file| (*file, abap_fixture_text(file)))
            .collect();
        let graph = abap_graph(&files);
        let mut edges: Vec<(String, String, &'static str)> = graph
            .edges
            .iter()
            .map(|edge| {
                (
                    abap_label(&graph, edge.from_entity.as_str()),
                    abap_label(&graph, edge.to_entity.as_str()),
                    edge.ref_type.as_str(),
                )
            })
            .collect();
        edges.sort();
        edges
    }

    #[cfg(feature = "lang-abap")]
    fn abap_typed_edge(from: &str, to: &str, kind: &'static str) -> (String, String, &'static str) {
        (from.to_string(), to.to_string(), kind)
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_me_call() {
        // `me->static_call( )`, `STATIC_CALL( )` with no receiver, and
        // `CALL METHOD me->static_call` reach the class's own method.
        let (sites, _) = abap_fixture_2_1_calls();
        for line in [26, 27, 28] {
            assert_eq!(
                abap_answers(&sites, "zcl_fx_calls.clas.abap", line),
                vec!["zcl_fx_calls.static_call"],
                "line {line}"
            );
        }
        let edges = abap_fixture_2_1_edges();
        assert!(
            edges.contains(&abap_typed_edge("zcl_fx_calls.me_call", "zcl_fx_calls.static_call", "calls")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_static_call() {
        // `zcl_fx_order=>create( )` in any case, here and in another object.
        let (sites, _) = abap_fixture_2_1_calls();
        for (file, line) in [
            ("zcl_fx_calls.clas.abap", 32),
            ("zcl_fx_calls.clas.abap", 33),
            ("zcl_fx_user.clas.abap", 13),
            ("zcl_fx_order.clas.testclasses.abap", 15),
            ("zfx_fg.fugr.zfx_fm.abap", 10),
        ] {
            assert_eq!(abap_answers(&sites, file, line), vec!["zcl_fx_order.create"], "{file}:{line}");
        }
        let edges = abap_fixture_2_1_edges();
        assert!(
            edges.contains(&abap_typed_edge("zcl_fx_user.run", "zcl_fx_order.create", "calls")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_super_call() {
        // `super->describe( )` in a redefinition reaches the base class's.
        let (sites, _) = abap_fixture_2_1_calls();
        for (file, line) in [
            ("zcl_fx_order_sub.clas.abap", 12),
            ("zcl_fx_calls.clas.abap", 21),
        ] {
            assert_eq!(abap_answers(&sites, file, line), vec!["zcl_fx_order.describe"], "{file}:{line}");
        }
        let edges = abap_fixture_2_1_edges();
        assert!(
            edges.contains(&abap_typed_edge("zcl_fx_order_sub.describe", "zcl_fx_order.describe", "calls")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_interface_prefixed_call() {
        // `zif_fx_order~get_total( )` with no receiver is the class's own
        // implementing method, or the one it inherits. Beside it on line 30,
        // `lo_helper->tag( )` has no receiver type yet.
        let (sites, _) = abap_fixture_2_1_calls();
        assert_eq!(
            abap_answers(&sites, "zcl_fx_order.clas.abap", 30),
            vec!["zcl_fx_order.zif_fx_order~get_total", "unknown: unknown receiver type"]
        );
        for line in [38, 39] {
            assert_eq!(
                abap_answers(&sites, "zcl_fx_calls.clas.abap", line),
                vec!["zcl_fx_order.zif_fx_order~get_total"],
                "line {line}"
            );
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_call_method() {
        // `CALL METHOD x->m` answers as `x->m( )` does, for `super`, `me`, a
        // class and a receiver with no type.
        let (sites, _) = abap_fixture_2_1_calls();
        let file = "zcl_fx_calls.clas.abap";
        for (classic, functional) in [(22, 21), (28, 26), (34, 32), (55, 54)] {
            assert_eq!(
                abap_answers(&sites, file, classic),
                abap_answers(&sites, file, functional),
                "line {classic} against {functional}"
            );
            assert_eq!(abap_answers(&sites, file, classic).len(), 1, "line {classic}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_call_function() {
        // `CALL FUNCTION 'ZFX_FM'` names the function module in a literal,
        // which the stripper blanks; in any case.
        let (sites, _) = abap_fixture_2_1_calls();
        for (file, line) in [
            ("zfx_report.prog.abap", 15),
            ("zcl_fx_calls.clas.abap", 43),
            ("zcl_fx_calls.clas.abap", 44),
        ] {
            assert_eq!(abap_answers(&sites, file, line), vec!["zfx_fg.zfx_fm"], "{file}:{line}");
        }
        // The report's own statements are its body; the method's call is its own.
        let edges = abap_fixture_2_1_edges();
        for from in ["zfx_report", "zcl_fx_calls.call_function"] {
            assert!(edges.contains(&abap_typed_edge(from, "zfx_fg.zfx_fm", "calls")), "{from}: {edges:?}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_perform() {
        // `PERFORM show_order` reaches the report's form, and `PERFORM
        // calc_extra` in a function module the form in its function group's
        // other file.
        let (sites, _) = abap_fixture_2_1_calls();
        assert_eq!(abap_answers(&sites, "zfx_report.prog.abap", 14), vec!["zfx_report.show_order"]);
        assert_eq!(abap_answers(&sites, "zfx_fg.fugr.zfx_fm.abap", 12), vec!["zfx_fg.calc_extra"]);
        let edges = abap_fixture_2_1_edges();
        for (from, to) in [("zfx_report", "zfx_report.show_order"), ("zfx_fg.zfx_fm", "zfx_fg.calc_extra")] {
            assert!(edges.contains(&abap_typed_edge(from, to, "calls")), "{from} -> {to}: {edges:?}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_new_gives_class_edge() {
        // `NEW zcl_x( )` calls the class: an edge to the class entity.
        let (sites, _) = abap_fixture_2_1_calls();
        for (file, line, class) in [
            ("zfx_report.prog.abap", 22, "zcl_fx_order"),
            ("zcl_fx_calls.clas.abap", 48, "zcl_fx_order"),
            ("zcl_fx_calls.clas.abap", 49, "zcl_fx_other"),
        ] {
            assert_eq!(abap_answers(&sites, file, line), vec![class], "{file}:{line}");
        }
        let edges = abap_fixture_2_1_edges();
        assert!(
            edges.contains(&abap_typed_edge("zfx_report.show_order", "zcl_fx_order", "calls")),
            "got: {edges:?}"
        );
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_attribute_refs_kept() {
        // The eight `typeref` edges from methods to their own class's
        // attributes, which the graph had before the pipeline ran for ABAP.
        let edges = abap_fixture_2_1_edges();
        for (from, to) in [
            ("zcl_fx_order.constructor", "zcl_fx_order.mv_id"),
            ("zcl_fx_order.describe", "zcl_fx_order.mv_id"),
            ("zcl_fx_order.zif_fx_order~add_item", "zcl_fx_order.mt_names"),
            ("zcl_fx_order.zif_fx_order~add_item", "zcl_fx_order.mv_total"),
            ("zcl_fx_order.zif_fx_order~get_total", "zcl_fx_order.mv_total"),
            ("zcl_fx_order.setup", "zcl_fx_order.mo_cut"),
            ("zcl_fx_order.total_starts_at_zero", "zcl_fx_order.mo_cut"),
            ("zcl_fx_order.describe_mentions_id", "zcl_fx_order.mo_cut"),
        ] {
            assert!(edges.contains(&abap_typed_edge(from, to, "typeref")), "{from} -> {to}: {edges:?}");
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_unknown_receiver() {
        // A receiver with no type yet, a local or an attribute, is an unknown
        // with its reason, in `Stats.unresolved`, never a guessed target.
        // (The bag-of-words resolver still binds `total` by its unique name
        // until receivers are typed and the pipeline replaces it.)
        let (sites, stats) = abap_fixture_2_1_calls();
        for line in [54, 55, 56] {
            assert_eq!(
                abap_answers(&sites, "zcl_fx_calls.clas.abap", line),
                vec!["unknown: unknown receiver type"],
                "line {line}"
            );
        }
        let unknown = sites.iter().filter(|(_, _, a)| a == "unknown: unknown receiver type").count();
        let mut reasons: Vec<(&str, usize)> = stats.unresolved.iter().map(|(why, n)| (*why, *n)).collect();
        reasons.sort();
        // plus the function group's generated `lzfx_fguxx` include, which abapGit does not hold
        assert_eq!(reasons, vec![("include not in repo", 1), ("unknown receiver type", unknown)]);
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_fixture_2_1_incremental_static_call() {
        // zcl_c calls `zcl_a=>ping( )` and stays clean while zcl_a loses and
        // regains `ping`: incremental rebuilds give the pipeline's edge as a
        // fresh build does.
        let class = |name: &str, method: &str, body: &str| {
            format!(
                "CLASS {name} DEFINITION PUBLIC CREATE PUBLIC.\n  PUBLIC SECTION.\n    CLASS-METHODS {method}.\nENDCLASS.\n\n\nCLASS {name} IMPLEMENTATION.\n\n  METHOD {method}.\n    {body}\n  ENDMETHOD.\n\nENDCLASS.\n"
            )
        };
        let dir = tempfile::TempDir::new().unwrap();
        let write = |file: &str, content: String| std::fs::write(dir.path().join(file), content).unwrap();
        let files: Vec<String> = ["zcl_a.clas.abap", "zcl_c.clas.abap"].iter().map(|f| f.to_string()).collect();
        let registry = crate::parser::plugins::create_default_registry();
        let build = || crate::parser::graph::EntityGraph::build(dir.path(), &files, &registry);
        let go_calls = |graph: &crate::parser::graph::EntityGraph| -> Vec<String> {
            graph
                .edges
                .iter()
                .filter(|e| abap_label(graph, e.from_entity.as_str()) == "zcl_c.go")
                .filter(|e| e.ref_type.as_str() == "calls")
                .map(|e| abap_label(graph, e.to_entity.as_str()))
                .collect()
        };

        write("zcl_a.clas.abap", class("zcl_a", "ping", "WRITE 'a'."));
        write("zcl_c.clas.abap", class("zcl_c", "go", "zcl_a=>ping( )."));
        let (mut graph, mut entities) = build();
        let mut session = crate::parser::session::GraphSession::build(dir.path(), &files, &registry);
        assert_eq!(go_calls(&graph), vec!["zcl_a.ping"]);

        for (method, expected) in [("pong", vec![]), ("ping", vec!["zcl_a.ping"])] {
            write("zcl_a.clas.abap", class("zcl_a", method, "WRITE 'a'."));
            let file = "zcl_a.clas.abap";
            let (stale, clean): (Vec<_>, Vec<_>) = entities.into_iter().partition(|e| e.file_path == file);
            let (incremental, next) = crate::parser::graph::EntityGraph::build_incremental(
                dir.path(),
                &[file.to_string()],
                &files,
                clean,
                graph.edges,
                stale,
                &registry,
            );
            session.rebuild(&files, &[file.to_string()], &registry);
            let fresh = build().0;
            assert_eq!(go_calls(&fresh), expected, "zcl_a defines {method}");
            assert_eq!(go_calls(&incremental), expected, "build_incremental, zcl_a defines {method}");
            assert_eq!(go_calls(session.graph()), expected, "session, zcl_a defines {method}");
            graph = incremental;
            entities = next;
        }
    }

    #[test]
    #[cfg(feature = "lang-abap")]
    fn abap_grammar_ends_literals_at_end_of_line() {
        // The fork's grammar ends a `'...'` or backtick literal at its line, with
        // a doubled quote as the escape, and reads a `|...|` template as one
        // token. So the census triggers above, `''` and `|{ a }*|`, no longer
        // cost the methods after them: the grammar has every one, the fallback
        // none.
        let code = "CLASS zcl_demo DEFINITION PUBLIC.\n  PUBLIC SECTION.\n    METHODS one.\nENDCLASS.\n\nCLASS zcl_demo IMPLEMENTATION.\n  METHOD one.\n    check( '' ).\n    lv = `it``s`.\n  ENDMETHOD.\n\n  METHOD two.\n    lv = |{ a }*|.\n    lv = 'it''s'.\n  ENDMETHOD.\n\n  METHOD three.\n    WRITE 'z'.\n  ENDMETHOD.\nENDCLASS.\n";
        assert_eq!(
            abap_rows(code, "zcl_demo.clas.abap"),
            vec![
                abap_row("class", "zcl_demo", 1, 20),
                abap_row("method", "one", 7, 10),
                abap_row("method", "two", 12, 15),
                abap_row("method", "three", 17, 19),
            ]
        );
        assert_eq!(
            abap_method_sources(code, "zcl_demo.clas.abap"),
            vec![("one".to_string(), None), ("two".to_string(), None), ("three".to_string(), None)]
        );
    }

    #[test]
    #[cfg(feature = "lang-fish")]
    fn test_fish_entity_extraction() {
        let code = r#"function greet
    echo "hello $argv[1]"
end

# the config.fish pattern: definitions wrapped in a top-level guard
if status is-interactive
    function fish_prompt
        set_color green
        echo -n (prompt_pwd) '> '
    end
end

function notify --on-event fish_command_finished --description "ping on done"
    greet $argv
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "config.fish");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"greet"), "plain function, got: {:?}", names);
        assert!(
            names.contains(&"fish_prompt"),
            "function inside a top-level if block, got: {:?}",
            names
        );
        assert!(
            names.contains(&"notify"),
            "function with option flags, got: {:?}",
            names
        );
        assert_eq!(entities.len(), 3, "only functions, got: {:?}", names);
    }

    #[test]
    fn test_typescript_entity_extraction() {
        // Existing language should still work
        let code = r#"
export function hello(): string {
    return "hello";
}

export class Greeter {
    greet(name: string): string {
        return `Hello, ${name}!`;
    }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "test.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"hello"), "Should find hello function");
        assert!(names.contains(&"Greeter"), "Should find Greeter class");
    }

    #[test]
    fn test_same_line_typescript_overload_ids_are_unique() {
        let code = "function f(a: number): void {}; function f(a: string): void {}\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "over.ts");
        let overloads: Vec<&SemanticEntity> = entities
            .iter()
            .filter(|entity| entity.name == "f" && entity.entity_type == "function")
            .collect();
        let ids: Vec<&str> = overloads.iter().map(|entity| entity.id.as_str()).collect();

        assert_eq!(
            overloads.len(),
            2,
            "expected both overloads, got: {entities:?}"
        );
        assert_eq!(
            ids,
            vec!["over.ts::function::f@L1#1", "over.ts::function::f@L1#2"]
        );
    }

    #[test]
    fn test_same_line_duplicate_parent_ids_are_propagated_to_children() {
        let code = "class C { m(){ return 1 } } class C { m(){ return 2 } }\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "c.ts");
        let classes: Vec<&SemanticEntity> = entities
            .iter()
            .filter(|entity| entity.name == "C" && entity.entity_type == "class")
            .collect();
        let methods: Vec<&SemanticEntity> = entities
            .iter()
            .filter(|entity| entity.name == "m" && entity.entity_type == "method")
            .collect();

        assert_eq!(classes.len(), 2, "expected both classes, got: {entities:?}");
        assert_eq!(methods.len(), 2, "expected both methods, got: {entities:?}");
        assert_eq!(classes[0].id, "c.ts::class::C@L1#1");
        assert_eq!(classes[1].id, "c.ts::class::C@L1#2");
        assert_eq!(methods[0].parent_id.as_deref(), Some("c.ts::class::C@L1#1"));
        assert_eq!(methods[1].parent_id.as_deref(), Some("c.ts::class::C@L1#2"));
        assert_eq!(methods[0].id, "c.ts::class::C@L1#1::m");
        assert_eq!(methods[1].id, "c.ts::class::C@L1#2::m");
    }

    #[test]
    fn test_module_typescript_entity_extraction() {
        let code = r#"
export function hello(): string {
    return "hello";
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "test.mts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"hello"), "Should find hello function");
    }

    #[test]
    fn test_commonjs_typescript_entity_extraction() {
        let code = r#"
export class Greeter {
    greet(name: string): string {
        return `Hello, ${name}!`;
    }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "test.cts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"Greeter"), "Should find Greeter class");
        assert!(names.contains(&"greet"), "Should find greet method");
    }

    #[test]
    fn test_typescript_generator_function_entity_extraction() {
        let code = r#"
export async function* streamUsers(): AsyncGenerator<string> {
    yield "alice";
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "stream.ts");
        let stream = entities.iter().find(|e| e.name == "streamUsers");

        assert!(
            stream.is_some(),
            "Should find generator function, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(stream.unwrap().entity_type, "function");
    }

    #[test]
    fn test_javascript_generator_function_entity_extraction() {
        let code = r#"
export function* ids() {
    yield 1;
    yield 2;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "ids.js");
        let ids = entities.iter().find(|e| e.name == "ids");

        assert!(
            ids.is_some(),
            "Should find generator function, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(ids.unwrap().entity_type, "function");
    }

    #[test]
    fn test_nested_functions_typescript() {
        let code = r#"
function outer() {
    function inner() {
        return 42;
    }
    return inner();
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "nested.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Nested TS: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"outer"),
            "Should find outer, got: {:?}",
            names
        );
        assert!(
            names.contains(&"inner"),
            "Should find inner, got: {:?}",
            names
        );

        let inner = entities.iter().find(|e| e.name == "inner").unwrap();
        assert!(inner.parent_id.is_some(), "inner should have parent_id");
    }

    #[test]
    fn test_typescript_nested_anonymous_class_fields() {
        let code = r#"
class L1 {
  L2 = class {
    L3 = class {
      L4 = class {
        method() { return 1; }
      };
    };
  };
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "a.ts");
        let find = |name: &str| {
            entities.iter().find(|e| e.name == name).unwrap_or_else(|| {
                panic!(
                    "missing {name}; got: {:?}",
                    entities
                        .iter()
                        .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                        .collect::<Vec<_>>()
                )
            })
        };

        let l1 = find("L1");
        assert_eq!(l1.entity_type, "class");
        let l1_id = l1.id.clone();

        let l2 = find("L2");
        assert_eq!(l2.entity_type, "field");
        assert_eq!(l2.parent_id.as_deref(), Some(l1_id.as_str()));
        let l2_id = l2.id.clone();

        let l3 = find("L3");
        assert_eq!(l3.entity_type, "field");
        assert_eq!(l3.parent_id.as_deref(), Some(l2_id.as_str()));
        let l3_id = l3.id.clone();

        let l4 = find("L4");
        assert_eq!(l4.entity_type, "field");
        assert_eq!(l4.parent_id.as_deref(), Some(l3_id.as_str()));
        let l4_id = l4.id.clone();

        let method = find("method");
        assert_eq!(method.entity_type, "method");
        assert_eq!(method.parent_id.as_deref(), Some(l4_id.as_str()));
        assert_eq!(method.id, "a.ts::class::L1::L2::L3::L4::method");
    }

    #[test]
    fn test_nested_functions_python() {
        let code = "def outer():\n    def inner():\n        return 42\n    return inner()\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "nested.py");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"outer"), "got: {:?}", names);
        assert!(names.contains(&"inner"), "got: {:?}", names);

        let inner = entities.iter().find(|e| e.name == "inner").unwrap();
        assert!(inner.parent_id.is_some(), "inner should have parent_id");
    }

    #[test]
    fn test_nested_functions_rust() {
        let code = "fn outer() {\n    fn inner() -> i32 {\n        42\n    }\n    inner();\n}\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "nested.rs");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"outer"), "got: {:?}", names);
        assert!(names.contains(&"inner"), "got: {:?}", names);

        let inner = entities.iter().find(|e| e.name == "inner").unwrap();
        assert!(inner.parent_id.is_some(), "inner should have parent_id");
    }

    #[test]
    fn test_rust_impl_blocks_unique_names() {
        let code = r#"
trait Greeting {
    fn greet(&self) -> String;
}

struct Person;
struct Robot;
struct Cat;

impl Greeting for Person {
    fn greet(&self) -> String { "Hello".to_string() }
}

impl Greeting for Robot {
    fn greet(&self) -> String { "Beep".to_string() }
}

impl Greeting for Cat {
    fn greet(&self) -> String { "Meow".to_string() }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "impls.rs");
        let impl_entities: Vec<&_> = entities
            .iter()
            .filter(|e| e.entity_type == "impl")
            .collect();
        let names: Vec<&str> = impl_entities.iter().map(|e| e.name.as_str()).collect();

        assert_eq!(
            impl_entities.len(),
            3,
            "Should find 3 impl blocks, got: {:?}",
            names
        );
        assert!(names.contains(&"Greeting for Person"), "got: {:?}", names);
        assert!(names.contains(&"Greeting for Robot"), "got: {:?}", names);
        assert!(names.contains(&"Greeting for Cat"), "got: {:?}", names);
    }

    #[test]
    fn test_nested_functions_go() {
        // Go doesn't have named nested functions, but has nested type/var declarations
        let code = "package main\n\nfunc outer() {\n    var x int = 42\n    _ = x\n}\n";
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "nested.go");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"outer"), "got: {:?}", names);
    }

    #[test]
    fn test_renamed_function_same_structural_hash() {
        let code_a = "def get_card():\n    return db.query('cards')\n";
        let code_b = "def get_card_1():\n    return db.query('cards')\n";

        let plugin = CodeParserPlugin;
        let entities_a = plugin.extract_entities(code_a, "a.py");
        let entities_b = plugin.extract_entities(code_b, "b.py");

        assert_eq!(entities_a.len(), 1, "Should find one entity in a");
        assert_eq!(entities_b.len(), 1, "Should find one entity in b");
        assert_eq!(entities_a[0].name, "get_card");
        assert_eq!(entities_b[0].name, "get_card_1");

        // Structural hash should match since only the name differs
        assert_eq!(
            entities_a[0].structural_hash, entities_b[0].structural_hash,
            "Renamed function with identical body should have same structural_hash"
        );

        // Content hash should differ (it includes the name)
        assert_ne!(
            entities_a[0].content_hash, entities_b[0].content_hash,
            "Content hash should differ since raw content includes the name"
        );
    }

    #[test]
    fn test_swift_renamed_operator_same_structural_hash() {
        let plugin = CodeParserPlugin;
        let entities_a = plugin.extract_entities("prefix operator ~~~\n", "a.swift");
        let entities_b = plugin.extract_entities("prefix operator !!!\n", "b.swift");

        assert_eq!(entities_a.len(), 1, "Should find one entity in a");
        assert_eq!(entities_b.len(), 1, "Should find one entity in b");
        assert_eq!(entities_a[0].name, "~~~");
        assert_eq!(entities_b[0].name, "!!!");
        assert_eq!(entities_a[0].entity_type, "operator");
        assert_eq!(entities_b[0].entity_type, "operator");
        assert_eq!(
            entities_a[0].structural_hash, entities_b[0].structural_hash,
            "Renamed operator with otherwise identical declaration should have same structural_hash"
        );
        assert_ne!(
            entities_a[0].content_hash, entities_b[0].content_hash,
            "Content hash should differ since raw content includes the operator token"
        );
    }

    #[test]
    fn test_swift_synthesized_names_disambiguate_overloads() {
        let plugin = CodeParserPlugin;
        let code = r#"
struct Matrix {
    subscript(row: Int) -> Double {
        return Double(row)
    }

    subscript(row: Int, column: Int) -> Double {
        return Double(row + column)
    }
}

class Builder {
    init(value: Int) {}
    init(text: String) {}
}
"#;

        let entities = plugin.extract_entities(code, "Overloads.swift");

        let subscript_ids: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "subscript")
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(subscript_ids.len(), 2);
        assert_ne!(subscript_ids[0], subscript_ids[1]);
        assert!(subscript_ids.iter().all(|id| id.contains("@L")));

        let init_ids: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "init")
            .map(|e| e.id.as_str())
            .collect();
        assert_eq!(init_ids.len(), 2);
        assert_ne!(init_ids[0], init_ids[1]);
        assert!(init_ids.iter().all(|id| id.contains("@L")));
    }

    #[test]
    fn test_hcl_entity_extraction() {
        let code = r#"
region = "eu-west-1"

variable "image_id" {
  type = string
}

resource "aws_instance" "web" {
  ami = var.image_id

  lifecycle {
    create_before_destroy = true
  }
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.tf");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "HCL entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"region"),
            "Should find top-level attribute, got: {:?}",
            names
        );
        assert!(
            names.contains(&"variable.image_id"),
            "Should find variable block, got: {:?}",
            names
        );
        assert!(
            names.contains(&"resource.aws_instance.web"),
            "Should find resource block, got: {:?}",
            names
        );
        assert!(
            names.contains(&"resource.aws_instance.web.lifecycle"),
            "Should find nested lifecycle block with qualified name, got: {:?}",
            names
        );
        assert!(
            !names.contains(&"ami"),
            "Should skip nested attributes inside blocks, got: {:?}",
            names
        );
        assert!(
            !names.contains(&"create_before_destroy"),
            "Should skip nested attributes inside nested blocks, got: {:?}",
            names
        );

        let lifecycle = entities
            .iter()
            .find(|e| e.name == "resource.aws_instance.web.lifecycle")
            .unwrap();
        assert!(
            lifecycle.parent_id.is_some(),
            "lifecycle should be nested under resource"
        );
        assert!(
            types.contains(&"attribute"),
            "Should preserve attribute entity type for top-level attributes"
        );
    }

    #[test]
    fn test_kotlin_entity_extraction() {
        let code = r#"
class UserService {
    val name: String = ""

    fun greet(): String {
        return "Hello, $name"
    }

    companion object {
        fun create(): UserService = UserService()
    }
}

interface Repository {
    fun findById(id: Int): Any?
}

object AppConfig {
    val version = "1.0"
}

fun topLevel(x: Int): Int = x * 2
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "App.kt");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Kotlin entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert!(names.contains(&"UserService"), "got: {:?}", names);
        assert!(names.contains(&"greet"), "got: {:?}", names);
        assert!(names.contains(&"Repository"), "got: {:?}", names);
        assert!(names.contains(&"findById"), "got: {:?}", names);
        assert!(names.contains(&"AppConfig"), "got: {:?}", names);
        assert!(names.contains(&"topLevel"), "got: {:?}", names);
    }

    #[test]
    fn test_xml_entity_extraction() {
        let code = r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
    <dependencies>
        <dependency>
            <groupId>junit</groupId>
            <artifactId>junit</artifactId>
        </dependency>
    </dependencies>
    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven</groupId>
            </plugin>
        </plugins>
    </build>
</project>
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "pom.xml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "XML entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert!(names.contains(&"project"), "got: {:?}", names);
        assert!(names.contains(&"dependencies"), "got: {:?}", names);
        assert!(names.contains(&"build"), "got: {:?}", names);
    }

    #[test]
    fn test_arrow_callback_scope_boundary_typescript() {
        // Arrow function callbacks: locals are suppressed, but inner
        // class/function declarations are still extracted. Nested callbacks
        // also suppress their locals.
        let code = r#"
const activeQueues = [
  { queue: queues.fooQueue, processor: foo.process },
];

activeQueues.forEach((handler: any) => {
  const queue = handler.queue;
  let retries = 0;

  class QueueHandler {
    handle() { return queue; }
  }

  function createHandler() {
    return new QueueHandler();
  }

  queue.process((job) => {
    const orderId = job.data.orderId;
    return orderId;
  });
});

function handleFailure(job: any, err: any) {
  console.error('failed', err);
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "process.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let top_level: Vec<&str> = entities
            .iter()
            .filter(|e| e.parent_id.is_none())
            .map(|e| e.name.as_str())
            .collect();

        // Top-level entities preserved
        assert!(top_level.contains(&"activeQueues"), "got: {:?}", top_level);
        assert!(top_level.contains(&"handleFailure"), "got: {:?}", top_level);

        // Declarations inside callback extracted
        assert!(names.contains(&"QueueHandler"), "got: {:?}", names);
        assert!(names.contains(&"handle"), "got: {:?}", names);
        assert!(names.contains(&"createHandler"), "got: {:?}", names);

        // Locals inside callbacks suppressed
        assert!(!names.contains(&"queue"), "got: {:?}", names);
        assert!(!names.contains(&"retries"), "got: {:?}", names);
        assert!(!names.contains(&"orderId"), "got: {:?}", names);
    }

    #[test]
    fn test_top_level_iife_wrapper_still_extracts_typescript_entities() {
        let code = r#"
function factory() {
  class Foo {
    method(): number {
      return 1;
    }
  }

  function bar(): Foo {
    return new Foo();
  }
}

factory();
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "wrapped.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"factory"),
            "Should find top-level wrapper function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Foo"),
            "Should find class inside top-level wrapper, got: {:?}",
            names
        );
        assert!(
            names.contains(&"bar"),
            "Should find function inside top-level wrapper, got: {:?}",
            names
        );
    }

    #[test]
    fn test_top_level_iife_still_extracts_typescript_entities() {
        let code = r#"
(() => {
  class Foo {
    method(): number {
      return 1;
    }
  }

  function bar(): Foo {
    return new Foo();
  }
})();
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "iife.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"Foo"),
            "Should find class inside top-level IIFE, got: {:?}",
            names
        );
        assert!(
            names.contains(&"bar"),
            "Should find function inside top-level IIFE, got: {:?}",
            names
        );
    }

    #[test]
    fn test_function_locals_not_extracted_as_nested_entities_typescript() {
        let code = r#"
export default function foo() {
  const x = 1;
  return x;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "default-export.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"foo"),
            "Should find exported function, got: {:?}",
            names
        );
        assert!(
            !names.contains(&"x"),
            "Local inside function should not be extracted as an entity, got: {:?}",
            names
        );
    }

    #[test]
    fn test_function_expression_scope_boundary_typescript() {
        // Function expressions: assigned to variables, or used as callback
        // arguments. Locals are suppressed in all cases.
        let code = r#"
const foo = function namedExpr(x: number) {
  const inner = x + 1;
  return inner;
};

const bar = function(y: number) {
  const local = y * 2;
  return local;
};

const items = [1, 2, 3];

items.forEach(function process(item) {
  const doubled = item * 2;
  console.log(doubled);
});
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "funexpr.ts");
        let top_level: Vec<&str> = entities
            .iter()
            .filter(|e| e.parent_id.is_none())
            .map(|e| e.name.as_str())
            .collect();
        let find = |name: &str| entities.iter().find(|e| e.name == name).unwrap();
        let all_names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        // Top-level declarations preserved, and const-assigned function
        // expressions are promoted from variable to function.
        assert!(top_level.contains(&"foo"), "got: {:?}", top_level);
        assert!(top_level.contains(&"bar"), "got: {:?}", top_level);
        assert!(top_level.contains(&"items"), "got: {:?}", top_level);
        assert_eq!(find("foo").entity_type, "function");
        assert_eq!(find("bar").entity_type, "function");
        assert_eq!(find("items").entity_type, "variable");

        // Locals inside function expressions suppressed
        assert!(!all_names.contains(&"inner"), "got: {:?}", all_names);
        assert!(!all_names.contains(&"local"), "got: {:?}", all_names);
        assert!(!all_names.contains(&"doubled"), "got: {:?}", all_names);

        // Named function expression used as callback argument not extracted
        assert!(!top_level.contains(&"process"), "got: {:?}", top_level);
    }

    #[test]
    fn test_variable_assigned_arrow_extracts_inner_entities() {
        // Arrow function assigned to a variable: inner class/function
        // declarations should be extracted, locals should be suppressed.
        let code = r#"
const handler = () => {
  class Inner {
    run() { return 1; }
  }

  function make() {
    return new Inner();
  }

  const local = 42;
};
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "assigned.ts");
        let handler = entities.iter().find(|e| e.name == "handler").unwrap();
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert_eq!(handler.entity_type, "function");
        assert!(names.contains(&"handler"), "got: {:?}", names);
        assert!(names.contains(&"Inner"), "got: {:?}", names);
        assert!(names.contains(&"run"), "got: {:?}", names);
        assert!(names.contains(&"make"), "got: {:?}", names);
        assert!(!names.contains(&"local"), "got: {:?}", names);
    }

    #[test]
    fn test_variable_assigned_function_expression_extracts_inner_entities() {
        // Function expression assigned to a variable: same behavior.
        let code = r#"
const handler = function() {
  class Inner {}
  function make() { return new Inner(); }
  const local = 42;
};
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "funexpr-inner.ts");
        let handler = entities.iter().find(|e| e.name == "handler").unwrap();
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert_eq!(handler.entity_type, "function");
        assert!(names.contains(&"handler"), "got: {:?}", names);
        assert!(names.contains(&"Inner"), "got: {:?}", names);
        assert!(names.contains(&"make"), "got: {:?}", names);
        assert!(!names.contains(&"local"), "got: {:?}", names);
    }

    #[test]
    fn test_let_assigned_arrow_stays_variable_typescript() {
        let code = r#"
let handler = () => {
  return 42;
};
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "let-assigned.ts");
        let handler = entities.iter().find(|e| e.name == "handler").unwrap();

        assert_eq!(handler.entity_type, "variable");
    }

    #[test]
    fn test_const_assigned_arrow_promoted_to_function_javascript() {
        let code = r#"
const handler = () => {
  return 42;
};
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "handler.js");
        let handler = entities.iter().find(|e| e.name == "handler").unwrap();

        assert_eq!(handler.entity_type, "function");
    }

    #[test]
    fn test_js_ts_multi_declarator_promotes_each_const_initializer() {
        let code = r#"
const value = 1, handler = () => value;
const first = () => 1, second = 2;
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "sample.ts");
        let find = |name: &str| {
            entities.iter().find(|e| e.name == name).unwrap_or_else(|| {
                panic!(
                    "missing {name}; got: {:?}",
                    entities
                        .iter()
                        .map(|e| (&e.name, &e.entity_type))
                        .collect::<Vec<_>>()
                )
            })
        };

        assert_eq!(find("value").entity_type, "variable");
        assert_eq!(find("handler").entity_type, "function");
        assert_eq!(find("first").entity_type, "function");
        assert_eq!(find("second").entity_type, "variable");
    }

    #[test]
    fn test_suppressed_multi_declarator_traverses_skipped_initializers() {
        let code = r#"
function wrapper() {
  const holder = class {
    run() { return 1; }
  }, handler = () => {
    class Inner {
      go() { return 2; }
    }
  }, value = 1;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "sample.ts");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let find = |name: &str| {
            entities.iter().find(|e| e.name == name).unwrap_or_else(|| {
                panic!(
                    "missing {name}; got: {:?}",
                    entities
                        .iter()
                        .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                        .collect::<Vec<_>>()
                )
            })
        };

        assert_eq!(find("wrapper").entity_type, "function");
        assert_eq!(find("handler").entity_type, "function");
        assert!(names.contains(&"run"), "got: {:?}", names);
        assert!(names.contains(&"Inner"), "got: {:?}", names);
        assert!(names.contains(&"go"), "got: {:?}", names);
        assert!(!names.contains(&"holder"), "got: {:?}", names);
        assert!(!names.contains(&"value"), "got: {:?}", names);
    }

    #[test]
    fn test_go_var_declaration() {
        let code = r#"package featuremgmt

type FeatureFlag struct {
	Name        string
	Description string
	Stage       string
}

var standardFeatureFlags = []FeatureFlag{
	{
		Name:        "panelTitleSearch",
		Description: "Search for dashboards using panel title",
		Stage:       "PublicPreview",
	},
}

func GetFlags() []FeatureFlag {
	return standardFeatureFlags
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "flags.go");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "Go entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"FeatureFlag"),
            "Should find type FeatureFlag, got: {:?}",
            names
        );
        assert!(
            names.contains(&"standardFeatureFlags"),
            "Should find var standardFeatureFlags, got: {:?}",
            names
        );
        assert!(
            names.contains(&"GetFlags"),
            "Should find func GetFlags, got: {:?}",
            names
        );
    }

    #[test]
    fn test_go_grouped_var_declaration() {
        let code = r#"package test

var (
	simple = 42
	flags = []string{"a", "b"}
)

const (
	x = 1
	y = 2
)

func main() {}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "test.go");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: Vec<&str> = entities.iter().map(|e| e.entity_type.as_str()).collect();
        eprintln!(
            "Go grouped entities: {:?}",
            names.iter().zip(types.iter()).collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"flags") || names.contains(&"simple"),
            "Should find grouped var, got: {:?}",
            names
        );
        assert!(
            names.contains(&"x"),
            "Should find grouped const x, got: {:?}",
            names
        );
        assert!(
            names.contains(&"main"),
            "Should find func main, got: {:?}",
            names
        );
    }

    #[test]
    fn test_dart_entity_extraction() {
        let code = r#"
import 'dart:math';

class Calculator {
  final String name;

  Calculator(this.name);

  Calculator.withDefault() : name = 'default';

  factory Calculator.create(String name) {
    return Calculator(name);
  }

  int add(int a, int b) {
    return a + b;
  }

  int get doubleAdd => add(1, 1) * 2;

  set label(String value) {
    // no-op
  }

  int operator +(Calculator other) {
    return 0;
  }
}

mixin Loggable {
  void log(String message) {
    print(message);
  }
}

extension StringExt on String {
  bool get isBlank => trim().isEmpty;
}

enum Status {
  active,
  inactive;

  String display() => name.toUpperCase();
}

typedef Callback = void Function(int);

int add(int a, int b) {
  return a + b;
}

extension type Wrapper(int value) implements int {}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "calculator.dart");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Dart entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        // Top-level declarations
        assert!(
            names.contains(&"Calculator"),
            "Should find class, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Loggable"),
            "Should find mixin, got: {:?}",
            names
        );
        assert!(
            names.contains(&"StringExt"),
            "Should find extension, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Status"),
            "Should find enum, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Callback"),
            "Should find typedef, got: {:?}",
            names
        );
        assert!(
            names.contains(&"add"),
            "Should find top-level function, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Wrapper"),
            "Should find extension type, got: {:?}",
            names
        );

        // Class members with correct types
        let add_method = entities
            .iter()
            .find(|e| e.name == "add" && e.parent_id.is_some());
        assert!(
            add_method.is_some(),
            "Should find add method inside Calculator"
        );
        assert_eq!(add_method.unwrap().entity_type, "method");

        // Named constructor gets distinct name from unnamed constructor
        let unnamed_ctor = entities
            .iter()
            .find(|e| e.name == "Calculator" && e.entity_type == "constructor");
        assert!(unnamed_ctor.is_some(), "Should find unnamed constructor");
        let named_ctor = entities.iter().find(|e| e.name == "Calculator.withDefault");
        assert!(
            named_ctor.is_some(),
            "Should find named constructor Calculator.withDefault, got: {:?}",
            names
        );
        assert_eq!(named_ctor.unwrap().entity_type, "constructor");
        assert_ne!(
            unnamed_ctor.unwrap().id,
            named_ctor.unwrap().id,
            "Named and unnamed constructors must have different entity IDs"
        );

        // Factory constructor
        let factory_ctor = entities.iter().find(|e| e.name == "Calculator.create");
        assert!(
            factory_ctor.is_some(),
            "Should find factory constructor Calculator.create, got: {:?}",
            names
        );
        assert_eq!(factory_ctor.unwrap().entity_type, "constructor");

        // Getter, setter, operator
        let getter = entities.iter().find(|e| e.name == "doubleAdd");
        assert!(getter.is_some(), "Should find getter doubleAdd");
        assert_eq!(getter.unwrap().entity_type, "getter");

        let setter = entities.iter().find(|e| e.name == "label");
        assert!(setter.is_some(), "Should find setter label");
        assert_eq!(setter.unwrap().entity_type, "setter");

        let operator = entities.iter().find(|e| e.name == "operator +");
        assert!(operator.is_some(), "Should find operator +");
        assert_eq!(operator.unwrap().entity_type, "method");

        // Mixin members have parent
        let log_method = entities.iter().find(|e| e.name == "log");
        assert!(log_method.is_some(), "Should find log in Loggable");
        assert!(
            log_method.unwrap().parent_id.is_some(),
            "log should have parent_id"
        );

        // Entity type mapping
        let callback = entities.iter().find(|e| e.name == "Callback").unwrap();
        assert_eq!(callback.entity_type, "type", "typedef should map to 'type'");

        let loggable = entities.iter().find(|e| e.name == "Loggable").unwrap();
        assert_eq!(loggable.entity_type, "mixin");

        let ext = entities.iter().find(|e| e.name == "StringExt").unwrap();
        assert_eq!(ext.entity_type, "extension");

        let wrapper = entities.iter().find(|e| e.name == "Wrapper").unwrap();
        assert_eq!(wrapper.entity_type, "extension");
    }

    #[test]
    #[cfg(feature = "lang-sql")]
    fn test_sql_entity_extraction() {
        let code = r#"
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
CREATE VIEW active_users AS SELECT * FROM users WHERE active;
CREATE FUNCTION add(a INT, b INT) RETURNS INT AS $$ BEGIN RETURN a + b; END; $$ LANGUAGE plpgsql;
CREATE INDEX idx_name ON users(name);
CREATE TYPE mood AS ENUM ('sad', 'happy');
CREATE SCHEMA myapp;
CREATE MATERIALIZED VIEW mv AS SELECT 1;
CREATE TABLE billing.invoices (id INT);
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "schema.sql");
        let by_name = |n: &str| entities.iter().find(|e| e.name == n);

        // object_reference names (incl. schema-qualified)
        assert_eq!(
            by_name("users").map(|e| e.entity_type.as_str()),
            Some("table")
        );
        assert_eq!(
            by_name("active_users").map(|e| e.entity_type.as_str()),
            Some("view")
        );
        assert_eq!(
            by_name("add").map(|e| e.entity_type.as_str()),
            Some("function")
        );
        assert_eq!(
            by_name("mood").map(|e| e.entity_type.as_str()),
            Some("type")
        );
        assert_eq!(by_name("mv").map(|e| e.entity_type.as_str()), Some("view"));
        assert_eq!(
            by_name("billing.invoices").map(|e| e.entity_type.as_str()),
            Some("table"),
            "schema-qualified table name should be preserved"
        );

        // CREATE INDEX / SCHEMA name a bare identifier, not the ON-table
        assert_eq!(
            by_name("idx_name").map(|e| e.entity_type.as_str()),
            Some("index"),
            "index should be named idx_name, not the table it indexes"
        );
        assert_eq!(
            by_name("myapp").map(|e| e.entity_type.as_str()),
            Some("schema")
        );
    }

    #[test]
    fn test_dart_top_level_function_includes_body() {
        let code = r#"
int add(int a, int b) {
  return a + b;
}

String greet(String name) => 'Hello, $name!';
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "funcs.dart");
        eprintln!(
            "Dart top-level: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.content))
                .collect::<Vec<_>>()
        );

        let add_fn = entities.iter().find(|e| e.name == "add").unwrap();
        assert!(
            add_fn.content.contains("return a + b"),
            "Top-level function content should include the body, got: {:?}",
            add_fn.content
        );

        let greet_fn = entities.iter().find(|e| e.name == "greet").unwrap();
        assert!(
            greet_fn.content.contains("Hello"),
            "Expression body should be included, got: {:?}",
            greet_fn.content
        );

        // Body changes should produce different content_hash
        let code_v2 = r#"
int add(int a, int b) {
  return a * b;
}

String greet(String name) => 'Hello, $name!';
"#;
        let entities_v2 = plugin.extract_entities(code_v2, "funcs.dart");
        let add_v2 = entities_v2.iter().find(|e| e.name == "add").unwrap();
        assert_ne!(
            add_fn.content_hash, add_v2.content_hash,
            "Body change should produce different content_hash"
        );

        // Unchanged function should keep the same hash
        let greet_v2 = entities_v2.iter().find(|e| e.name == "greet").unwrap();
        assert_eq!(
            greet_fn.content_hash, greet_v2.content_hash,
            "Unchanged function should keep the same content_hash"
        );
    }

    #[test]
    fn test_dart_renamed_named_constructor_same_structural_hash() {
        let code_a = r#"
class Foo {
  Foo.fromJson(Map<String, dynamic> json) {
    print(json);
  }
}
"#;
        let code_b = r#"
class Foo {
  Foo.fromMap(Map<String, dynamic> json) {
    print(json);
  }
}
"#;
        let plugin = CodeParserPlugin;
        let entities_a = plugin.extract_entities(code_a, "a.dart");
        let entities_b = plugin.extract_entities(code_b, "b.dart");

        let ctor_a = entities_a
            .iter()
            .find(|e| e.name == "Foo.fromJson")
            .unwrap();
        let ctor_b = entities_b.iter().find(|e| e.name == "Foo.fromMap").unwrap();

        assert_eq!(
            ctor_a.structural_hash, ctor_b.structural_hash,
            "Renamed named constructor with identical body should have same structural_hash"
        );
        assert_ne!(
            ctor_a.content_hash, ctor_b.content_hash,
            "Content hash should differ since raw content includes the name"
        );
    }

    #[test]
    fn test_dart_top_level_getter_setter() {
        let code = r#"
int _value = 0;

int get currentValue {
  return _value;
}

set currentValue(int v) {
  _value = v;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "accessors.dart");
        eprintln!(
            "Dart top-level accessors: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.content))
                .collect::<Vec<_>>()
        );

        let getter = entities
            .iter()
            .find(|e| e.name == "currentValue" && e.entity_type == "getter");
        assert!(
            getter.is_some(),
            "Should find top-level getter, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert!(
            getter.unwrap().content.contains("return _value"),
            "Top-level getter content should include the body"
        );
        assert!(
            getter.unwrap().parent_id.is_none(),
            "Top-level getter should have no parent"
        );

        // tree-sitter-dart 0.2.0 parses top-level setters as function_signature
        // (treating `set` as a type_identifier). setter_signature is only
        // produced inside class_member → method_signature.
        let setter = entities
            .iter()
            .find(|e| e.name == "currentValue" && e.entity_type == "function");
        assert!(
            setter.is_some(),
            "Should find top-level setter as function, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert!(
            setter.unwrap().content.contains("_value = v"),
            "Top-level setter content should include the body"
        );
    }

    #[test]
    fn test_dart_field_entity_type() {
        let code = r#"
class Config {
  final String name;
  static const int maxRetries = 3;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "config.dart");
        eprintln!(
            "Dart fields: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        let name_field = entities
            .iter()
            .find(|e| e.name == "name" && e.parent_id.is_some());
        assert!(
            name_field.is_some(),
            "Should find field 'name', got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(name_field.unwrap().entity_type, "field");

        let max_retries = entities.iter().find(|e| e.name == "maxRetries");
        assert!(
            max_retries.is_some(),
            "Should find field 'maxRetries', got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(max_retries.unwrap().entity_type, "field");
    }

    #[test]
    fn test_dart_identifier_list_fields() {
        // identifier_list produces bare identifier children (no "name" field),
        // unlike initialized_identifier_list which wraps each in an
        // initialized_identifier node with a "name" field.
        let code = r#"
abstract class Shape {
  abstract double x, y;
  abstract String label;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "shape.dart");
        eprintln!(
            "Dart identifier_list fields: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        let x_field = entities.iter().find(|e| e.name == "x");
        assert!(
            x_field.is_some(),
            "Should find field 'x' from identifier_list, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(x_field.unwrap().entity_type, "field");
        assert!(
            x_field.unwrap().parent_id.is_some(),
            "field 'x' should be nested under Shape"
        );

        let label_field = entities.iter().find(|e| e.name == "label");
        assert!(
            label_field.is_some(),
            "Should find field 'label' from single-element identifier_list, got: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );
        assert_eq!(label_field.unwrap().entity_type, "field");
    }

    #[test]
    fn test_ocaml_entity_extraction() {
        let code = r#"
type color = Red | Green | Blue

type point = {
  x : float;
  y : float;
}

exception Not_found of string

let greet name =
  Printf.printf "Hello, %s!\n" name

let add a b = a + b

let version = "1.0"

let color_to_string = function
  | Red -> "red"
  | Blue -> "blue"

let inc = fun x -> x + 1

module MyModule = struct
  let helper x = x * 2
end

module type Printable = sig
  val to_string : 'a -> string
end

external caml_input : in_channel -> bytes -> int -> int -> int = "caml_input"

class point_class x_init = object
  val mutable x = x_init
  method get_x = x
end

class type measurable = object
  method measure : float
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "example.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("color").entity_type, "type");
        assert_eq!(find("point").entity_type, "type");
        assert_eq!(find("Not_found").entity_type, "exception");
        assert_eq!(find("greet").entity_type, "function");
        assert_eq!(find("add").entity_type, "function");
        assert_eq!(find("version").entity_type, "value");
        assert_eq!(find("color_to_string").entity_type, "function");
        assert_eq!(find("inc").entity_type, "function");
        assert_eq!(find("MyModule").entity_type, "module");
        assert_eq!(find("Printable").entity_type, "module_type");
        assert_eq!(find("caml_input").entity_type, "external");
        assert_eq!(find("point_class").entity_type, "class");
        assert_eq!(find("measurable").entity_type, "class_type");
    }

    #[test]
    fn test_ocaml_nested_module_entities() {
        let code = r#"
module Outer = struct
  let x = 42

  module Inner = struct
    let y = 0
  end
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "nested.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml nested: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        let outer = find("Outer");
        let x = find("x");
        let inner = find("Inner");
        let y = find("y");

        assert_eq!(outer.entity_type, "module");
        assert_eq!(x.entity_type, "value");
        assert_eq!(inner.entity_type, "module");
        assert_eq!(y.entity_type, "value");

        assert!(
            x.parent_id.as_ref().is_some_and(|p| p == &outer.id),
            "x should be nested under Outer"
        );
        assert!(
            inner.parent_id.as_ref().is_some_and(|p| p == &outer.id),
            "Inner should be nested under Outer"
        );
        assert!(
            y.parent_id.as_ref().is_some_and(|p| p == &inner.id),
            "y should be nested under Inner"
        );
    }

    #[test]
    fn test_ocaml_interface_entity_extraction() {
        let code = r#"
type t

val create : string -> t
val to_string : t -> string

exception Invalid_input of string

module type Serializable = sig
  val serialize : t -> string
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "example.mli");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml interface entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("t").entity_type, "type");
        assert_eq!(find("create").entity_type, "val");
        assert_eq!(find("to_string").entity_type, "val");
        assert_eq!(find("Invalid_input").entity_type, "exception");
        assert_eq!(find("Serializable").entity_type, "module_type");
    }

    #[test]
    fn test_ocaml_mutual_recursion_let() {
        let code = r#"
let rec even n = (n = 0) || odd (n - 1)
and odd n = (n <> 0) && even (n - 1)

let rec ping x = pong (x - 1)
and pong x = if x <= 0 then 0 else ping (x - 1)
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "mutual.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml mutual let: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("even").entity_type, "function");
        assert_eq!(find("odd").entity_type, "function");
        assert_eq!(find("ping").entity_type, "function");
        assert_eq!(find("pong").entity_type, "function");
    }

    #[test]
    fn test_ocaml_mutual_recursion_module() {
        let code = r#"
module rec A : sig val x : int end = struct
  let x = B.y + 1
end
and B : sig val y : int end = struct
  let y = 0
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "mutual_mod.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml mutual module: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type, &e.parent_id))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        let a = find("A");
        let b = find("B");
        assert_eq!(a.entity_type, "module");
        assert_eq!(b.entity_type, "module");

        let x = find("x");
        let y = find("y");
        assert!(
            x.parent_id.as_ref().is_some_and(|p| p == &a.id),
            "x should be nested under A"
        );
        assert!(
            y.parent_id.as_ref().is_some_and(|p| p == &b.id),
            "y should be nested under B"
        );
    }

    #[test]
    fn test_ocaml_destructured_let() {
        let code = r#"
let (a, b) = (1, 2)

let { x; y } = point

let simple = 42
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "destruct.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml destructured: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("a").entity_type, "value");
        assert_eq!(find("b").entity_type, "value");
        assert_eq!(find("x").entity_type, "value");
        assert_eq!(find("y").entity_type, "value");
        assert_eq!(find("simple").entity_type, "value");
    }

    #[test]
    fn test_ocaml_mutual_recursion_class() {
        let code = r#"
class foo = object
  method x = 1
end
and bar = object
  method y = 2
end
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "classes.ml");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "OCaml mutual class: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("foo").entity_type, "class");
        assert_eq!(find("bar").entity_type, "class");
    }

    #[test]
    fn test_perl_entity_extraction() {
        let code = r#"package Foo::Bar;

use strict;
use warnings;

sub hello {
    my ($self, $name) = @_;
    print "Hello, $name!\n";
}

sub _private_helper {
    return 42;
}

1;
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Foo/Bar.pm");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"Foo::Bar"), "got: {:?}", names);
        assert!(names.contains(&"hello"), "got: {:?}", names);
        assert!(names.contains(&"_private_helper"), "got: {:?}", names);

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("Foo::Bar").entity_type, "package");
        assert_eq!(find("hello").entity_type, "function");
        assert_eq!(find("_private_helper").entity_type, "function");
    }

    #[test]
    fn test_fortran_entity_extraction() {
        let code = r#"module math_utils
  implicit none
contains
  function add(a, b) result(c)
    integer, intent(in) :: a, b
    integer :: c
    c = a + b
  end function add

  subroutine greet()
    print *, "hello"
  end subroutine greet
end module math_utils

program main
  implicit none
  print *, "hello"
end program main
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "test.f90");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"math_utils"), "got: {:?}", names);
        assert!(names.contains(&"add"), "got: {:?}", names);
        assert!(names.contains(&"greet"), "got: {:?}", names);
        assert!(names.contains(&"main"), "got: {:?}", names);

        let find = |name: &str| {
            entities
                .iter()
                .find(|e| e.name == name)
                .unwrap_or_else(|| panic!("Should find {}, got: {:?}", name, names))
        };

        assert_eq!(find("math_utils").entity_type, "module");
        assert_eq!(find("add").entity_type, "function");
        assert_eq!(find("greet").entity_type, "subroutine");
        assert_eq!(find("main").entity_type, "program");

        // Nested entities have parent
        assert!(find("add").parent_id.is_some());
        assert!(find("greet").parent_id.is_some());
    }

    #[test]
    fn test_scala_entity_extraction() {
        let code = r#"
package com.example

import scala.collection.mutable

class UserService(val name: String) {
  def getUsers(): List[User] = db.findAll()

  def createUser(user: User): Unit = db.save(user)

  private def validate(user: User): Boolean = true
}

object UserService {
  def apply(name: String): UserService = new UserService(name)

  val DefaultName: String = "default"
}

trait Repository[T] {
  def findById(id: String): Option[T]
  def findAll(): List[T]
}

case class User(id: String, name: String)

type UserId = String
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "UserService.scala");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Scala entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"UserService"),
            "Should find class UserService, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Repository"),
            "Should find trait Repository, got: {:?}",
            names
        );
        assert!(
            names.contains(&"getUsers"),
            "Should find method getUsers, got: {:?}",
            names
        );
        assert!(
            names.contains(&"createUser"),
            "Should find method createUser, got: {:?}",
            names
        );

        // Methods should be nested under class
        let get_users = entities.iter().find(|e| e.name == "getUsers").unwrap();
        assert!(
            get_users.parent_id.is_some(),
            "getUsers should have parent_id"
        );
    }

    #[test]
    fn test_scala3_entity_extraction() {
        let code = r#"
package com.example

enum Color:
  case Red, Green, Blue

enum Planet(mass: Double, radius: Double):
  case Mercury extends Planet(3.303e+23, 2.4397e6)
  case Venus   extends Planet(4.869e+24, 6.0518e6)

object Main:
  def main(args: Array[String]): Unit =
    println("Hello, World!")

trait Greeter:
  def greet(name: String): String

given Greeter with
  def greet(name: String): String = s"Hello, $name!"

extension (s: String)
  def shout: String = s.toUpperCase + "!"

type Predicate[A] = A => Boolean
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "Main.scala");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        eprintln!(
            "Scala 3 entities: {:?}",
            entities
                .iter()
                .map(|e| (&e.name, &e.entity_type))
                .collect::<Vec<_>>()
        );

        assert!(
            names.contains(&"Color"),
            "Should find enum Color, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Planet"),
            "Should find enum Planet, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Main"),
            "Should find object Main, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Greeter"),
            "Should find trait Greeter, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Predicate"),
            "Should find type alias Predicate, got: {:?}",
            names
        );
    }

    #[test]
    fn test_zig_entity_extraction() {
        let code = r#"
const std = @import("std");

pub const Point = struct {
    x: i32,
    y: i32,
};

pub const Color = enum {
    red,
    green,
    blue,
};

const Person = struct {
    name: []const u8,
    age: u32,
};

pub fn greet(name: []const u8) void {
    std.debug.print("Hello, {s}!\n", .{name});
}

fn add(a: i32, b: i32) i32 {
    return a + b;
}

pub fn main() !void {
    greet("world");
}

test "basic addition" {
    const result = add(2, 3);
    _ = result;
}
"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "main.zig");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: std::collections::HashMap<&str, &str> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.entity_type.as_str()))
            .collect();

        assert!(
            names.contains(&"greet"),
            "Should find greet, got: {:?}",
            names
        );
        assert!(names.contains(&"add"), "Should find add, got: {:?}", names);
        assert!(
            names.contains(&"main"),
            "Should find main, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Point"),
            "Should find Point, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Color"),
            "Should find Color, got: {:?}",
            names
        );
        assert!(
            names.contains(&"Person"),
            "Should find Person, got: {:?}",
            names
        );

        assert_eq!(types["greet"], "function");
        assert_eq!(types["add"], "function");
        assert_eq!(types["Point"], "struct");
        assert_eq!(types["Color"], "enum");
        assert_eq!(types["Person"], "struct");
    }

    #[test]
    #[cfg(feature = "lang-edn")]
    fn test_edn_deps_edn_map_entries() {
        let code = r#"{:deps {org.clojure/clojure {:mvn/version "1.11.0"}}
 :paths ["src" "resources"]
 :aliases {:dev {:extra-deps {cider/cider-nrepl {:mvn/version "0.28.5"}}}}}"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "deps.edn");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();
        let types: std::collections::HashMap<&str, &str> = entities
            .iter()
            .map(|e| (e.name.as_str(), e.entity_type.as_str()))
            .collect();

        assert!(
            names.contains(&":deps"),
            "Should find :deps, got: {:?}",
            names
        );
        assert!(
            names.contains(&":paths"),
            "Should find :paths, got: {:?}",
            names
        );
        assert!(
            names.contains(&":aliases"),
            "Should find :aliases, got: {:?}",
            names
        );
        assert_eq!(
            names.len(),
            3,
            "Should have exactly 3 entries, got: {:?}",
            names
        );
        assert_eq!(types[":deps"], "entry");
        assert_eq!(types[":paths"], "entry");
        assert_eq!(types[":aliases"], "entry");
    }

    #[test]
    #[cfg(feature = "lang-edn")]
    fn test_edn_nested_map_values_not_extracted() {
        // Inner map entries (inside :aliases) must not leak as top-level entities.
        let code = r#"{:a {:b 1 :c 2} :d 3}"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "config.edn");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&":a"), "Should find :a, got: {:?}", names);
        assert!(names.contains(&":d"), "Should find :d, got: {:?}", names);
        assert!(!names.contains(&":b"), "Inner :b should not be extracted");
        assert!(!names.contains(&":c"), "Inner :c should not be extracted");
        assert_eq!(names.len(), 2);
    }

    #[test]
    #[cfg(feature = "lang-edn")]
    fn test_edn_non_map_top_level_forms_not_extracted() {
        // A bare vector at the top level has no meaningful name and yields no entities.
        let code = r#"["alpha" "beta"]"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "data.edn");
        assert_eq!(entities.len(), 0);
    }

    #[test]
    #[cfg(feature = "lang-edn")]
    fn test_edn_symbol_keys_extracted() {
        let code = r#"{foo 1 bar 2}"#;
        let plugin = CodeParserPlugin;
        let entities = plugin.extract_entities(code, "sym.edn");
        let names: Vec<&str> = entities.iter().map(|e| e.name.as_str()).collect();

        assert!(names.contains(&"foo"), "Should find foo, got: {:?}", names);
        assert!(names.contains(&"bar"), "Should find bar, got: {:?}", names);
    }

    #[test]
    #[cfg(feature = "lang-typescript")]
    fn test_top_level_imports_excludes_strings_comments_and_nested() {
        assert!(top_level_imports("a.ts", "const x = 1;").unwrap().is_empty());
        assert!(top_level_imports("a.ts", "import { broken").is_none());
        assert!(top_level_imports("a.unknown", "something").is_none());
        let code = concat!(
            "import { parse } from \"./parser.js\";\n",
            "\n",
            "export const FIXTURE = `\n",
            "import { parse } from \"./legacy.js\";\n",
            "`;\n",
            "\n",
            "declare module \"m\" {\n",
            "  import { x } from \"./nested.js\";\n",
            "}\n",
            "\n",
            "const y = 1;\n",
            "import {\n",
            "  a,\n",
            "  b\n",
            "} from \"./ab.js\";\n",
        );
        let imports = top_level_imports("a.ts", code).expect("valid TypeScript");
        let texts: Vec<&str> = imports.iter().map(|i| i.text.as_str()).collect();
        // Two real top-level imports: the leading one, and the one after `const y`.
        assert_eq!(imports.len(), 2, "got: {texts:?}");
        assert_eq!((imports[0].start_line, imports[0].end_line), (1, 1));
        // The multi-line import comes back as one node spanning its full range.
        assert_eq!((imports[1].start_line, imports[1].end_line), (12, 15));
        // Never the import inside the template literal or the ambient module block.
        assert!(imports.iter().all(|i| !i.text.contains("legacy.js")), "got: {texts:?}");
        assert!(imports.iter().all(|i| !i.text.contains("nested.js")), "got: {texts:?}");
    }
}
