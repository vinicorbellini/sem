//! ABAP entities that mkoval1/tree-sitter-abap has no node for.
//!
//! The grammar parses `FORM`, `MODULE`, `DEFINE` and `PROGRAM` as a generic
//! `macro_include` whose name is the keyword, and leaves the matching
//! `ENDFORM`, `ENDMODULE` or `END-OF-DEFINITION` wherever its error recovery
//! lands: often inside an `ERROR` that has also swallowed the body, and
//! sometimes the next `FORM` with it. A class's `TYPES` become `ERROR` nodes,
//! or part of the `METHODS` or `DATA` statement before them. So none of these
//! can be read off a node or a sibling sequence. They are read off the tokens
//! instead: every leaf of the tree in document order, comments left out, cut
//! into statements at each `.`. A literal is a single leaf, so a period inside
//! one does not end a statement.
//!
//! Every entity made here carries `source: abap-fallback` in its metadata, so
//! they can be counted apart from the ones the grammar gives.
//!
//! This module exists only for the grammar's gaps. Delete it once the grammar
//! has nodes for forms, dynpro modules, macro definitions and `TYPES` (asked
//! of mkoval1/tree-sitter-abap), and list those nodes in `ABAP_CONFIG` like
//! the rest.

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::entity_extractor::line_number_for_byte;
use crate::model::entity::{build_entity_id, SemanticEntity};
use crate::utils::hash::content_hash;

const METADATA_SOURCE: (&str, &str) = ("source", "abap-fallback");

/// Add the `report` (for `PROGRAM`), `form`, `module`, `macro` and class-level
/// `type` entities of an ABAP file to the ones the tree walk found, then put
/// all of them back in source order.
///
/// Forms, modules and `PROGRAM` sit at the top of the file; a macro defined
/// inside a form or module nests under it; a `TYPES` nests under the
/// `CLASS ... DEFINITION` it is declared in. `TYPES` outside a class
/// definition (a program's or an interface's, or a local type in a body) is
/// not an entity, the same as `DATA` there.
pub(super) fn extract_abap_fallback_entities(
    root: Node,
    file_path: &str,
    source: &[u8],
    entities: &mut Vec<SemanticEntity>,
) {
    let classes: Vec<(usize, usize, String)> = entities
        .iter()
        .filter(|e| e.entity_type == "class")
        .filter_map(|e| Some((e.start_byte?, e.end_byte?, e.id.clone())))
        .collect();

    let mut found = Vec::new();
    let mut open_blocks: Vec<OpenBlock> = Vec::new();
    let mut open_type: Option<OpenType> = None;

    for statement in statements(root, source) {
        let Some(head) = statement.head(source) else {
            continue;
        };
        let keyword = head.keyword.to_ascii_uppercase();

        if keyword != "TYPES" {
            open_type = None;
        }

        match keyword.as_str() {
            "PROGRAM" => {
                if let Some(name) = head.name {
                    found.push(fallback_entity(
                        file_path,
                        source,
                        "report",
                        name,
                        None,
                        head.start_byte,
                        statement.end_byte(),
                    ));
                }
            }
            "FORM" | "MODULE" | "DEFINE" => {
                let Some(name) = head.name else {
                    continue;
                };
                let entity_type = match keyword.as_str() {
                    "FORM" => "form",
                    "MODULE" => "module",
                    _ => "macro",
                };
                // FORM and MODULE do not nest: an earlier one still open was
                // never closed, so it is dropped rather than given a guessed end.
                if entity_type != "macro" {
                    if let Some(i) = open_blocks.iter().position(|b| b.entity_type != "macro") {
                        open_blocks.truncate(i);
                    }
                }
                let parent_id = open_blocks.last().map(|b| b.id.clone());
                open_blocks.push(OpenBlock {
                    id: build_entity_id(file_path, entity_type, &name.text, parent_id.as_deref()),
                    entity_type,
                    name,
                    parent_id,
                    start_byte: head.start_byte,
                });
            }
            "ENDFORM" | "ENDMODULE" | "END-OF-DEFINITION" => {
                let entity_type = match keyword.as_str() {
                    "ENDFORM" => "form",
                    "ENDMODULE" => "module",
                    _ => "macro",
                };
                let Some(i) = open_blocks
                    .iter()
                    .rposition(|b| b.entity_type == entity_type)
                else {
                    continue;
                };
                // Anything opened after it and still open was never closed.
                let Some(block) = open_blocks.drain(i..).next() else {
                    continue;
                };
                found.push(fallback_entity(
                    file_path,
                    source,
                    block.entity_type,
                    block.name,
                    block.parent_id.as_deref(),
                    block.start_byte,
                    statement.end_byte(),
                ));
            }
            "TYPES" => {
                let Some(class_id) = innermost_class(&classes, head.start_byte) else {
                    continue;
                };
                found.extend(types_entities(
                    file_path,
                    source,
                    &statement,
                    head.start_byte,
                    class_id,
                    &mut open_type,
                ));
            }
            _ => {}
        }
    }

    if found.is_empty() {
        return;
    }
    entities.extend(found);
    entities.sort_by_key(|e| {
        (
            e.start_byte.unwrap_or(0),
            std::cmp::Reverse(e.end_byte.unwrap_or(0)),
        )
    });
}

/// A FORM, MODULE or DEFINE whose closing statement has not been seen yet.
struct OpenBlock {
    id: String,
    entity_type: &'static str,
    name: Word,
    parent_id: Option<String>,
    start_byte: usize,
}

/// A `TYPES BEGIN OF x` whose `END OF x` has not been seen yet: the classic
/// form spreads one structure over several `TYPES` statements.
struct OpenType {
    name: Word,
    start_byte: usize,
    depth: usize,
}

/// A word of the source and where it is, so the structural hash can leave the
/// name out the way the grammar path does.
struct Word {
    text: String,
    start_byte: usize,
    end_byte: usize,
}

/// One ABAP statement: its tokens, and the period that closes it (none for a
/// statement cut short by the end of the file).
struct Statement<'tree> {
    tokens: Vec<Node<'tree>>,
    period: Option<Node<'tree>>,
}

/// A statement's leading keyword (`END-OF-DEFINITION` is one keyword, though
/// it is five tokens) and the word after it, which names a FORM, MODULE,
/// DEFINE or PROGRAM.
struct Head {
    keyword: String,
    name: Option<Word>,
    start_byte: usize,
}

impl Statement<'_> {
    fn head(&self, source: &[u8]) -> Option<Head> {
        let first = self.tokens.first()?;
        let last = self.tokens.last()?;
        let start = trimmed_start(*first, source);
        let text = std::str::from_utf8(&source[start..last.end_byte()]).ok()?;
        let mut words = words(text).map(|(offset, word)| Word {
            text: word.to_string(),
            start_byte: start + offset,
            end_byte: start + offset + word.len(),
        });
        let keyword = words.next()?.text;
        Some(Head {
            keyword,
            name: words.next(),
            start_byte: start,
        })
    }

    fn end_byte(&self) -> usize {
        self.period
            .or_else(|| self.tokens.last().copied())
            .map_or(0, |n| n.end_byte())
    }
}

/// Words of a statement's text, with their byte offsets: split at whitespace,
/// and at `:` and `,`, which end a word in a chained statement.
fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split(|c: char| c.is_whitespace() || c == ':' || c == ',')
        .scan(0usize, |offset, word| {
            let at = *offset;
            *offset += word.len() + 1;
            Some((at, word))
        })
        .filter(|(_, word)| !word.is_empty())
}

/// Cut the tree's leaves into statements at each period.
fn statements<'tree>(root: Node<'tree>, source: &[u8]) -> Vec<Statement<'tree>> {
    let mut statements = Vec::new();
    let mut tokens = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "bol_comment" | "eol_comment") {
            continue;
        }
        if node.child_count() > 0 {
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
            continue;
        }
        // A MISSING period is the grammar's guess, not the source's.
        if node.is_missing() || node.start_byte() == node.end_byte() {
            continue;
        }
        if node.kind() == "." {
            if !tokens.is_empty() {
                statements.push(Statement {
                    tokens: std::mem::take(&mut tokens),
                    period: Some(node),
                });
            }
        } else if !node_text(node, source).trim().is_empty() {
            tokens.push(node);
        }
    }
    if !tokens.is_empty() {
        statements.push(Statement {
            tokens,
            period: None,
        });
    }
    statements
}

/// The `type` entities of one `TYPES` statement: one per declared name, so
/// `TYPES: a TYPE i, b TYPE string.` gives `a` and `b`, and a `BEGIN OF s ...
/// END OF s` structure gives `s`, not its components. The first declarator
/// starts at the `TYPES` keyword; each ends at the `,` or `.` after it.
fn types_entities(
    file_path: &str,
    source: &[u8],
    statement: &Statement,
    statement_start: usize,
    class_id: &str,
    open_type: &mut Option<OpenType>,
) -> Vec<SemanticEntity> {
    let texts: Vec<String> = statement
        .tokens
        .iter()
        .map(|t| node_text(*t, source).trim().to_ascii_uppercase())
        .collect();
    let is = |i: usize, word: &str| texts.get(i).is_some_and(|t| t == word);

    let mut out = Vec::new();
    // Skip `TYPES` and the `:` of a chain.
    let mut i = if is(1, ":") { 2 } else { 1 };
    let mut declarator_start = statement_start;
    while i < statement.tokens.len() {
        if is(i, ",") {
            if let Some(open) = open_type.take_if(|o| o.depth == 0) {
                out.push(type_entity(
                    file_path,
                    source,
                    open,
                    class_id,
                    statement.tokens[i],
                ));
            }
            i += 1;
            declarator_start = statement
                .tokens
                .get(i)
                .map_or(0, |t| trimmed_start(*t, source));
            continue;
        }
        if open_type.is_none() {
            let name_at = if is(i, "BEGIN") && is(i + 1, "OF") {
                // `BEGIN OF ENUM e` and `BEGIN OF MESH m` name the type after the kind.
                if is(i + 2, "ENUM") || is(i + 2, "MESH") {
                    i + 3
                } else {
                    i + 2
                }
            } else {
                i
            };
            let Some(name) = statement.tokens.get(name_at) else {
                break;
            };
            *open_type = Some(OpenType {
                name: Word {
                    text: node_text(*name, source).trim().to_string(),
                    start_byte: trimmed_start(*name, source),
                    end_byte: name.end_byte(),
                },
                start_byte: declarator_start,
                depth: 0,
            });
        }
        if let Some(open) = open_type.as_mut() {
            if is(i, "BEGIN") && is(i + 1, "OF") {
                open.depth += 1;
                i += 1;
            } else if is(i, "END") && is(i + 1, "OF") {
                open.depth = open.depth.saturating_sub(1);
                i += 1;
            }
        }
        i += 1;
    }

    // The period closes the last declarator, unless a `BEGIN OF` is still open
    // and its `END OF` comes in a later `TYPES` statement.
    if let Some(end) = statement
        .period
        .or_else(|| statement.tokens.last().copied())
    {
        if let Some(open) = open_type.take_if(|o| o.depth == 0) {
            out.push(type_entity(file_path, source, open, class_id, end));
        }
    }
    out
}

fn type_entity(
    file_path: &str,
    source: &[u8],
    open: OpenType,
    class_id: &str,
    end: Node,
) -> SemanticEntity {
    fallback_entity(
        file_path,
        source,
        "type",
        open.name,
        Some(class_id),
        open.start_byte,
        end.end_byte(),
    )
}

fn innermost_class(classes: &[(usize, usize, String)], byte: usize) -> Option<&str> {
    classes
        .iter()
        .filter(|(start, end, _)| *start <= byte && byte < *end)
        .min_by_key(|(start, end, _)| end - start)
        .map(|(_, _, id)| id.as_str())
}

fn fallback_entity(
    file_path: &str,
    source: &[u8],
    entity_type: &str,
    name: Word,
    parent_id: Option<&str>,
    start_byte: usize,
    end_byte: usize,
) -> SemanticEntity {
    let content = String::from_utf8_lossy(&source[start_byte..end_byte]).into_owned();
    // No single node spans the entity, so the structural hash is over its text
    // with whitespace collapsed and the name left out, and kappa is skipped,
    // as for Swift's recovered containers.
    let before = String::from_utf8_lossy(&source[start_byte..name.start_byte.max(start_byte)]);
    let after = String::from_utf8_lossy(&source[name.end_byte.min(end_byte)..end_byte]);
    let normalized = before
        .split_whitespace()
        .chain(after.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ");
    SemanticEntity {
        id: build_entity_id(file_path, entity_type, &name.text, parent_id),
        file_path: file_path.to_string(),
        entity_type: entity_type.to_string(),
        name: name.text,
        parent_id: parent_id.map(String::from),
        content_hash: content_hash(&content),
        structural_hash: Some(content_hash(&normalized)),
        kappa: None,
        content,
        start_line: line_number_for_byte(source, start_byte),
        end_line: line_number_for_byte(source, end_byte.saturating_sub(1)),
        start_byte: Some(start_byte),
        end_byte: Some(end_byte),
        metadata: Some(BTreeMap::from([(
            METADATA_SOURCE.0.to_string(),
            METADATA_SOURCE.1.to_string(),
        )])),
    }
}

/// The grammar's leaves carry the whitespace before them (`" show_order"`).
fn trimmed_start(node: Node, source: &[u8]) -> usize {
    let text = node_text(node, source);
    node.start_byte() + (text.len() - text.trim_start().len())
}

fn node_text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}
