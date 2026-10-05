//! SAP ABAP grammar for [tree-sitter], sem's fork of
//! [mkoval1/tree-sitter-abap](https://github.com/mkoval1/tree-sitter-abap)
//! at commit `c7604df9e25d56ae879fa25694fd9f2ddbab05d8` (2024-06-29, MIT).
//! The changes this fork makes are listed in the crate's `README.md`.
//!
//! ```
//! let mut parser = tree_sitter::Parser::new();
//! parser
//!     .set_language(&tree_sitter_abap::LANGUAGE.into())
//!     .expect("Error loading ABAP parser");
//! let tree = parser.parse("REPORT zdemo.", None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_abap() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for this grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_abap) };

/// The content of the [`node-types.json`] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading ABAP parser");
    }
}
