//! ABAP entities that mkoval1/tree-sitter-abap has no node for.
//!
//! The grammar parses `FORM`, `MODULE`, `DEFINE` and `PROGRAM` as a generic
//! `macro_include` whose name is the keyword, and leaves the matching
//! `ENDFORM`, `ENDMODULE` or `END-OF-DEFINITION` wherever its error recovery
//! lands: often inside an `ERROR` that has also swallowed the body, and
//! sometimes the next `FORM` with it. A class's `TYPES` become `ERROR` nodes,
//! or part of the `METHODS` or `DATA` statement before them. So none of these
//! can be read off a node or a sibling sequence. They are read off the source
//! instead, with comments and literals blanked by the same stripper the
//! reference scan uses (`strip_abap_content`), cut into statements at each `.`.
//! A period inside a literal or a comment does not end a statement.
//!
//! Not off the tree's leaves: the grammar does not end a `'...'` literal at
//! the end of its line, as ABAP does, so after a quote it misreads (`''`) one
//! leaf can run over several lines and hold the statements in them, `METHOD`
//! and `ENDMETHOD` among them.
//!
//! `METHOD` has a node, `method_implementation`, but the grammar's error
//! recovery loses it in two ways. A statement it cannot parse can drop the
//! rest of a `CLASS x IMPLEMENTATION`, so no method after it is an entity; or
//! a `method_implementation` runs on past its own `ENDMETHOD` to the end of the
//! class, so one entity covers the methods after it. So `METHOD x.` ...
//! `ENDMETHOD.` blocks are read off the same statements, and reconciled with
//! the grammar's methods (`reconcile_methods`): the grammar's entity wins
//! where it has one, ended at its own `ENDMETHOD`, and a block it has none for
//! becomes a method of its own.
//!
//! Every entity made here carries `source: abap-fallback` in its metadata, so
//! they can be counted apart from the ones the grammar gives. A grammar method
//! cut back to its `ENDMETHOD` is still the grammar's, and carries nothing.
//!
//! An interface's `METHODS` are read off the same statements, one `method`
//! per name of a chain (`chain_parts`), as its declarations are the methods.
//!
//! This module exists only for the grammar's gaps. Delete it once the grammar
//! has nodes for forms, dynpro modules, macro definitions and `TYPES`, and
//! recovers a method without losing the class around it (asked of
//! mkoval1/tree-sitter-abap), and list those nodes in `ABAP_CONFIG` like the
//! rest.

use std::collections::BTreeMap;

use super::abap_name::is_abap_top_include;
use super::entity_extractor::line_number_for_byte;
use crate::model::entity::{build_entity_id, SemanticEntity};
use crate::parser::graph::strip_abap_content;
use crate::utils::hash::content_hash;

const METADATA_SOURCE: (&str, &str) = ("source", "abap-fallback");

/// Add the `report` (for `PROGRAM`), `form`, `module`, `macro` and class-level
/// `type`, `constant` and `variable` entities of an ABAP file to the ones the
/// tree walk found, and the `method` entities its error recovery lost, then
/// put all of them back in source order.
///
/// Forms, modules and `PROGRAM` sit at the top of the file; a macro defined
/// inside a form or module nests under it; a `TYPES`, `CONSTANTS`, `DATA` or
/// `CLASS-DATA` nests under the `CLASS ... DEFINITION` or `INTERFACE` it is
/// declared in, one entity per name of a chain. Outside one (a program's, or
/// a local in a body) it is not an entity, save a `TOP` include's globals. A method nests under the
/// `CLASS ... IMPLEMENTATION` it is written in; a `METHOD` outside one is not
/// an entity. An interface's `METHODS` and `CLASS-METHODS` declare its
/// methods, one `method` entity per name of a chain, under the interface:
/// they have no body, so the declaration is the method.
pub(super) fn extract_abap_fallback_entities(
    file_path: &str,
    source: &[u8],
    entities: &mut Vec<SemanticEntity>,
) {
    let Ok(text) = std::str::from_utf8(source) else {
        return;
    };
    let code = strip_abap_content(text);

    // The blocks a `TYPES` or `CONSTANTS` statement can be declared in.
    let classes: Vec<(usize, usize, String)> = entities
        .iter()
        .filter(|e| e.entity_type == "class" || e.entity_type == "interface")
        .filter_map(|e| Some((e.start_byte?, e.end_byte?, e.id.clone())))
        .collect();

    let mut found = Vec::new();
    let mut open_blocks: Vec<OpenBlock> = Vec::new();
    let mut open_type: Option<OpenType> = None;
    let mut open_data: Option<OpenType> = None;
    let mut open_constant: Option<OpenType> = None;
    let top_include = is_abap_top_include(file_path);
    let mut class_blocks: Vec<ClassBlock> = Vec::new();
    let mut open_class: Option<ClassBlock> = None;
    let mut open_method: Option<OpenMethod> = None;
    let mut in_interface = false;

    for statement in statements(&code) {
        let Some(head) = statement.head(&code) else {
            continue;
        };
        let keyword = head.keyword.to_ascii_uppercase();

        if keyword != "TYPES" {
            open_type = None;
        }
        if keyword != "DATA" && keyword != "CLASS-DATA" {
            open_data = None;
        }
        if keyword != "CONSTANTS" {
            open_constant = None;
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
            "CLASS" => {
                // CLASS does not nest: a block still open was never closed (or
                // is a `DEFINITION DEFERRED`, which has no ENDCLASS), and a
                // METHOD still open in it never will be.
                class_blocks.extend(open_class.take());
                open_method = None;
                let kind = head.kind.as_ref().map(|k| k.text.to_ascii_uppercase());
                let implementation = match kind.as_deref() {
                    Some("IMPLEMENTATION") => true,
                    Some("DEFINITION") => false,
                    _ => continue,
                };
                let Some(name) = head.name else {
                    continue;
                };
                open_class = Some(ClassBlock {
                    name,
                    implementation,
                    start_byte: indented_start(source, head.start_byte),
                    end_byte: None,
                    methods: Vec::new(),
                });
            }
            "ENDCLASS" => {
                if let Some(mut class) = open_class.take() {
                    class.end_byte = Some(statement.end_byte());
                    class_blocks.push(class);
                }
                open_method = None;
            }
            "METHOD" => {
                // METHOD does not nest either: an earlier one still open was
                // never closed, so it is dropped rather than given a guessed end.
                let in_implementation = open_class.as_ref().is_some_and(|c| c.implementation);
                open_method = match head.name {
                    Some(name) if in_implementation => Some(OpenMethod {
                        name,
                        start_byte: indented_start(source, head.start_byte),
                        keyword_byte: head.start_byte,
                    }),
                    _ => None,
                };
            }
            "ENDMETHOD" => {
                if let (Some(method), Some(class)) = (open_method.take(), open_class.as_mut()) {
                    class.methods.push(MethodBlock {
                        name: method.name,
                        start_byte: method.start_byte,
                        keyword_byte: method.keyword_byte,
                        end_byte: statement.end_byte(),
                    });
                }
            }
            "INTERFACE" => {
                let kind = head.kind.as_ref().map(|k| k.text.to_ascii_uppercase());
                in_interface = !matches!(kind.as_deref(), Some("DEFERRED" | "LOAD"));
            }
            "ENDINTERFACE" => in_interface = false,
            "METHODS" | "CLASS-METHODS" if in_interface => {
                let Some(interface_id) = innermost_class(&classes, head.start_byte) else {
                    continue;
                };
                for part in chain_parts(&statement, &code) {
                    let name = part.tokens[0];
                    let name = Word {
                        text: name.text(&code).to_string(),
                        start_byte: name.start_byte,
                        end_byte: name.end_byte,
                    };
                    found.push(text_entity(
                        file_path,
                        source,
                        "method",
                        name,
                        Some(interface_id),
                        part.start_byte,
                        part.end_byte,
                    ));
                }
            }
            "TYPES" => {
                let Some(class_id) =
                    declaring_block(file_path, &classes, open_class.as_ref(), head.start_byte)
                else {
                    continue;
                };
                if open_method.is_some() {
                    continue;
                }
                found.extend(declarator_entities(
                    file_path,
                    source,
                    &code,
                    &statement,
                    head.start_byte,
                    "type",
                    Some(&class_id),
                    &mut open_type,
                ));
            }
            // A class's or interface's constants, and the global constants of
            // a `TOP` include, by the same rule as its `DATA` below.
            "CONSTANTS" => {
                let class_id =
                    declaring_block(file_path, &classes, open_class.as_ref(), head.start_byte);
                if open_method.is_some()
                    || class_id.is_none() && !(top_include && open_blocks.is_empty())
                {
                    continue;
                }
                found.extend(declarator_entities(
                    file_path,
                    source,
                    &code,
                    &statement,
                    head.start_byte,
                    "constant",
                    class_id.as_deref(),
                    &mut open_constant,
                ));
            }
            // A class's or interface's attributes, `DATA` and `CLASS-DATA`, one
            // `variable` per name of a chain and one per `BEGIN OF ... END OF`
            // block, as for `TYPES`; the grammar gives the same where it reads
            // the definition, and `reconcile_declarations` keeps its entities.
            // And the global data of a function group or program: the `DATA` of
            // its `TOP` include, outside any FORM or MODULE. Anywhere else a
            // `DATA` is a local, or a program global the pool leaves out (see
            // `is_abap_local_data`).
            "DATA" | "CLASS-DATA" => {
                let class_id =
                    declaring_block(file_path, &classes, open_class.as_ref(), head.start_byte);
                let global = keyword == "DATA" && top_include && open_blocks.is_empty();
                if open_method.is_some() || class_id.is_none() && !global {
                    continue;
                }
                found.extend(declarator_entities(
                    file_path,
                    source,
                    &code,
                    &statement,
                    head.start_byte,
                    "variable",
                    class_id.as_deref(),
                    &mut open_data,
                ));
            }
            _ => {}
        }
    }

    class_blocks.extend(open_class);
    reconcile_declarations(entities, &mut found);
    reconcile_methods(file_path, source, &class_blocks, entities, &mut found);
    drop_unparented_declarations(entities, &mut found);

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

/// A `CLASS x DEFINITION` or `CLASS x IMPLEMENTATION`, and the `METHOD`
/// blocks closed inside an implementation. Its end is the `ENDCLASS`, none if
/// another `CLASS` or the end of the file comes first.
struct ClassBlock {
    name: Word,
    implementation: bool,
    start_byte: usize,
    end_byte: Option<usize>,
    methods: Vec<MethodBlock>,
}

/// A `METHOD` whose `ENDMETHOD` has not been seen yet.
struct OpenMethod {
    name: Word,
    start_byte: usize,
    keyword_byte: usize,
}

/// A `METHOD x.` ... `ENDMETHOD.` block: from the `METHOD` statement's
/// indentation, where the grammar starts a `method_implementation`, to the
/// `ENDMETHOD` statement's period.
struct MethodBlock {
    name: Word,
    start_byte: usize,
    keyword_byte: usize,
    end_byte: usize,
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
#[derive(Clone)]
struct Word {
    text: String,
    start_byte: usize,
    end_byte: usize,
}

/// One ABAP statement: its tokens, and the period that closes it (none for a
/// statement cut short by the end of the file).
pub(crate) struct Statement {
    pub(crate) tokens: Vec<Token>,
    pub(crate) period: Option<Token>,
}

/// Where a token is. Its text is read off the stripped code, never the source,
/// so a comment or a literal is never a word.
#[derive(Clone, Copy)]
pub(crate) struct Token {
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

impl Token {
    pub(crate) fn text<'a>(&self, code: &'a str) -> &'a str {
        &code[self.start_byte..self.end_byte]
    }
}

/// A statement's leading keyword (`END-OF-DEFINITION` is one keyword), the
/// word after it, which names a FORM, MODULE, DEFINE, PROGRAM, METHOD or
/// CLASS, and the word after that, which says whether a CLASS statement is its
/// `DEFINITION` or its `IMPLEMENTATION`.
struct Head {
    keyword: String,
    name: Option<Word>,
    kind: Option<Word>,
    start_byte: usize,
}

impl Statement {
    fn head(&self, code: &str) -> Option<Head> {
        let first = self.tokens.first()?;
        let last = self.tokens.last()?;
        let start = first.start_byte;
        let mut words = words(&code[start..last.end_byte]).map(|(offset, word)| Word {
            text: word.to_string(),
            start_byte: start + offset,
            end_byte: start + offset + word.len(),
        });
        let keyword = words.next()?.text;
        Some(Head {
            keyword,
            name: words.next(),
            kind: words.next(),
            start_byte: start,
        })
    }

    pub(crate) fn end_byte(&self) -> usize {
        self.period
            .or_else(|| self.tokens.last().copied())
            .map_or(0, |t| t.end_byte)
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

/// The names an ABAP source includes, in source order, as written: one per
/// `INCLUDE zfoo.` statement (and per name of `INCLUDE: a, b.`).
///
/// `INCLUDE STRUCTURE x.` and `INCLUDE TYPE x.` inside a `DATA` or `TYPES`
/// block are statements of their own that start with the same keyword, and are
/// not program includes. System includes (`INCLUDE <icon>.`) are not either.
/// Read off the statement stream like `FORM`, so a comment or a literal that
/// says "INCLUDE" is not one.
pub(crate) fn include_names(text: &str) -> Vec<String> {
    let code = strip_abap_content(text);
    let mut names = Vec::new();
    for statement in statements(&code) {
        let mut words = statement.tokens.iter().map(|t| t.text(&code)).peekable();
        if !words
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("INCLUDE"))
        {
            continue;
        }
        // Segments of a chained statement start after the keyword and each `,`.
        let mut segment_start = true;
        let mut first = true;
        for word in words {
            match word {
                ":" => continue,
                "," => {
                    segment_start = true;
                    continue;
                }
                _ => {}
            }
            if !std::mem::take(&mut segment_start) {
                continue;
            }
            if first
                && (word.eq_ignore_ascii_case("STRUCTURE") || word.eq_ignore_ascii_case("TYPE"))
            {
                break;
            }
            first = false;
            if !word.starts_with('<') {
                names.push(word.to_string());
            }
        }
    }
    names
}

/// Cut the stripped code into statements at each period. Whitespace separates
/// tokens, and `.`, `,` and `:` are tokens of their own. The calls pipeline's
/// ABAP lowering (`calls::abap`) reads its statements here too.
pub(crate) fn statements(code: &str) -> Vec<Statement> {
    let mut statements = Vec::new();
    let mut tokens = Vec::new();
    let mut word_start: Option<usize> = None;
    for (i, b) in code.bytes().enumerate() {
        let punctuation = matches!(b, b'.' | b',' | b':');
        if !b.is_ascii_whitespace() && !punctuation {
            word_start.get_or_insert(i);
            continue;
        }
        if let Some(start) = word_start.take() {
            tokens.push(Token {
                start_byte: start,
                end_byte: i,
            });
        }
        let token = Token {
            start_byte: i,
            end_byte: i + 1,
        };
        if b == b'.' {
            if !tokens.is_empty() {
                statements.push(Statement {
                    tokens: std::mem::take(&mut tokens),
                    period: Some(token),
                });
            }
        } else if punctuation {
            tokens.push(token);
        }
    }
    if let Some(start) = word_start {
        tokens.push(Token {
            start_byte: start,
            end_byte: code.len(),
        });
    }
    if !tokens.is_empty() {
        statements.push(Statement {
            tokens,
            period: None,
        });
    }
    statements
}

/// One declaration of a statement, chained (`METHODS: a ..., b ....`) or
/// not: its tokens, from the name it declares, and its text, from the
/// keyword for the first and from the name for the others, to the `,` or the
/// `.` after it.
pub(crate) struct ChainPart<'s> {
    pub(crate) tokens: &'s [Token],
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

/// Split a declaration statement at its chain's `:` and `,` into one part per
/// name it declares. A statement with no chain is one part.
pub(crate) fn chain_parts<'s>(statement: &'s Statement, code: &str) -> Vec<ChainPart<'s>> {
    let tokens = &statement.tokens;
    let Some(keyword) = tokens.first() else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    let mut from = 1;
    let mut start_byte = keyword.start_byte;
    for i in 1..=tokens.len() {
        let end_byte = match tokens.get(i) {
            Some(t) if matches!(t.text(code), ":" | ",") => t.end_byte,
            Some(_) => continue,
            None => statement.end_byte(),
        };
        if from < i {
            parts.push(ChainPart {
                tokens: &tokens[from..i],
                start_byte,
                end_byte,
            });
        }
        from = i + 1;
        // the first part starts at the keyword, past a `:` after it
        if let (Some(next), false) = (tokens.get(from), parts.is_empty()) {
            start_byte = next.start_byte;
        }
    }
    parts
}

/// The entities of one `TYPES` (`type`) or `DATA` (`variable`) statement: one
/// per declared name, so `TYPES: a TYPE i, b TYPE string.` gives `a` and `b`,
/// and a `BEGIN OF s ... END OF s` structure gives `s`, not its components. The
/// first declarator starts at the keyword; each ends at the `,` or `.` after it.
#[allow(clippy::too_many_arguments)]
fn declarator_entities(
    file_path: &str,
    source: &[u8],
    code: &str,
    statement: &Statement,
    statement_start: usize,
    entity_type: &'static str,
    parent_id: Option<&str>,
    open_type: &mut Option<OpenType>,
) -> Vec<SemanticEntity> {
    let texts: Vec<String> = statement
        .tokens
        .iter()
        .map(|t| t.text(code).to_ascii_uppercase())
        .collect();
    let is = |i: usize, word: &str| texts.get(i).is_some_and(|t| t == word);

    let mut out = Vec::new();
    // Skip `TYPES` and the `:` of a chain.
    let mut i = if is(1, ":") { 2 } else { 1 };
    let mut declarator_start = statement_start;
    while i < statement.tokens.len() {
        if is(i, ",") {
            if let Some(open) = open_type.take_if(|o| o.depth == 0) {
                out.push(declarator_entity(
                    file_path,
                    source,
                    open,
                    entity_type,
                    parent_id,
                    statement.tokens[i],
                ));
            }
            i += 1;
            declarator_start = statement.tokens.get(i).map_or(0, |t| t.start_byte);
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
                    text: name.text(code).to_string(),
                    start_byte: name.start_byte,
                    end_byte: name.end_byte,
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
            out.push(declarator_entity(
                file_path,
                source,
                open,
                entity_type,
                parent_id,
                end,
            ));
        }
    }
    out
}

fn declarator_entity(
    file_path: &str,
    source: &[u8],
    open: OpenType,
    entity_type: &'static str,
    parent_id: Option<&str>,
    end: Token,
) -> SemanticEntity {
    fallback_entity(
        file_path,
        source,
        entity_type,
        open.name,
        parent_id,
        open.start_byte,
        end.end_byte,
    )
}

/// Drop the `type`, `constant` and `variable` entities read off the
/// statements that the grammar already gave: one that has the same type and
/// parent and overlaps the fallback's range is the same declaration. The
/// grammar's entity wins, as a method's does in `reconcile_methods`; the
/// fallback keeps what the grammar lost, in the files and classes it fails on.
fn reconcile_declarations(entities: &[SemanticEntity], found: &mut Vec<SemanticEntity>) {
    found.retain(|f| {
        if !matches!(f.entity_type.as_str(), "type" | "constant" | "variable") {
            return true;
        }
        !entities.iter().any(|e| {
            e.entity_type == f.entity_type
                && e.parent_id == f.parent_id
                && e.start_byte < f.end_byte
                && f.start_byte < e.end_byte
        })
    });
}

/// Reconcile the `METHOD` blocks read off the statements with the `method`
/// entities the tree walk found, so that every method entity is one block.
///
/// A grammar method that starts at a block's `METHOD` and has its name is that
/// block's: it is kept, with its id and parent, and if it runs on past the
/// block's `ENDMETHOD` it is cut back to it, so it no longer spans the methods
/// after it. One the recovery left outside any class, with no parent, is
/// nested under its implementation.
///
/// A block with no grammar method becomes a `method` of its own, nested under
/// its `CLASS ... IMPLEMENTATION` the way the grammar nests one, so
/// `collapse_abap_classes` moves it to the class; or under the class itself
/// when the grammar's definition runs on over the implementation. Where the
/// grammar has neither, the implementation is added as an `impl` too, from
/// `CLASS` to `ENDCLASS`, and so is its `CLASS ... DEFINITION` if the grammar
/// lost that as well, so the class is named and identified by its definition
/// as it is when the grammar reads it. Without an `ENDCLASS` neither is added,
/// and nor are the implementation's recovered methods.
///
/// A grammar method that is no block's is dropped: a `METHOD` with no
/// `ENDMETHOD`, which the grammar reads on into the next method, as for
/// `FORM`; or a `METHOD` whose name the grammar misread (`get_steps` for
/// `zif_x~get_steps`, or a later token), whose block has an entity of its
/// own.
fn reconcile_methods(
    file_path: &str,
    source: &[u8],
    class_blocks: &[ClassBlock],
    entities: &mut Vec<SemanticEntity>,
    found: &mut Vec<SemanticEntity>,
) {
    let mut claimed = vec![false; entities.len()];
    for implementation in class_blocks.iter().filter(|c| c.implementation) {
        // The grammar's own implementation, or its definition when that runs on
        // over the implementation and so already holds it.
        let holds_implementation = |e: &SemanticEntity| {
            let at = implementation.start_byte;
            e.start_byte.is_some_and(|start| start < at) && e.end_byte.is_some_and(|end| at < end)
        };
        let grammar_impl = entities.iter().find(|e| {
            e.parent_id.is_none()
                && e.name.eq_ignore_ascii_case(&implementation.name.text)
                && (e.entity_type == "impl" || e.entity_type == "class" && holds_implementation(e))
        });
        let impl_id = match (grammar_impl, implementation.end_byte) {
            (Some(e), _) => Some(e.id.clone()),
            (None, Some(end_byte)) if !implementation.methods.is_empty() => {
                let has_class = entities.iter().any(|e| {
                    e.entity_type == "class"
                        && e.parent_id.is_none()
                        && e.name.eq_ignore_ascii_case(&implementation.name.text)
                });
                let lost_definition = class_blocks.iter().find_map(|c| {
                    let is_definition = !c.implementation
                        && c.name.text.eq_ignore_ascii_case(&implementation.name.text);
                    c.end_byte.filter(|_| is_definition).map(|end| (c, end))
                });
                if let (false, Some((definition, end_byte))) = (has_class, lost_definition) {
                    found.push(fallback_entity(
                        file_path,
                        source,
                        "class",
                        definition.name.clone(),
                        None,
                        definition.start_byte,
                        end_byte,
                    ));
                }
                let entity = fallback_entity(
                    file_path,
                    source,
                    "impl",
                    implementation.name.clone(),
                    None,
                    implementation.start_byte,
                    end_byte,
                );
                let id = entity.id.clone();
                found.push(entity);
                Some(id)
            }
            _ => None,
        };

        for block in &implementation.methods {
            let grammar_method = entities.iter().position(|e| {
                e.entity_type == "method"
                    && e.name.eq_ignore_ascii_case(&block.name.text)
                    && e.start_byte
                        .is_some_and(|start| starts_at(source, start, block.keyword_byte))
            });
            match (grammar_method, &impl_id) {
                (Some(i), _) => {
                    claimed[i] = true;
                    let method = &mut entities[i];
                    if method.end_byte.is_some_and(|end| end > block.end_byte) {
                        end_method_at(method, source, block);
                    }
                    if let (None, Some(impl_id)) = (&method.parent_id, &impl_id) {
                        method.id =
                            build_entity_id(file_path, "method", &method.name, Some(impl_id));
                        method.parent_id = Some(impl_id.clone());
                    }
                }
                (None, Some(impl_id)) => found.push(fallback_entity(
                    file_path,
                    source,
                    "method",
                    block.name.clone(),
                    Some(impl_id),
                    block.start_byte,
                    block.end_byte,
                )),
                (None, None) => {}
            }
        }
    }

    let mut index = 0;
    entities.retain(|e| {
        index += 1;
        e.entity_type != "method" || claimed[index - 1]
    });
}

/// Whether an entity starting at `start` is the one whose `METHOD` keyword is
/// at `keyword_byte`: nothing but whitespace between the two.
fn starts_at(source: &[u8], start: usize, keyword_byte: usize) -> bool {
    start <= keyword_byte
        && source[start..keyword_byte]
            .iter()
            .all(u8::is_ascii_whitespace)
}

/// Cut a grammar method that runs on past its `ENDMETHOD` back to it. Its
/// node still spans the methods after it, so the hashes are over its own text
/// the way a fallback entity's are.
fn end_method_at(entity: &mut SemanticEntity, source: &[u8], block: &MethodBlock) {
    let Some(start_byte) = entity.start_byte else {
        return;
    };
    let content = String::from_utf8_lossy(&source[start_byte..block.end_byte]).into_owned();
    entity.content_hash = content_hash(&content);
    entity.structural_hash = Some(structural_hash(
        source,
        &block.name,
        start_byte,
        block.end_byte,
    ));
    entity.kappa = None;
    entity.content = content;
    entity.end_line = line_number_for_byte(source, block.end_byte.saturating_sub(1));
    entity.end_byte = Some(block.end_byte);
}

/// The grammar starts a `method_implementation` at its line's indentation (its
/// first leaf carries it), so a recovered block starts there too, and a method
/// has the same content whichever of the two gives it.
fn indented_start(source: &[u8], keyword_byte: usize) -> usize {
    let line_start = source[..keyword_byte]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    if source[line_start..keyword_byte]
        .iter()
        .all(|&b| b == b' ' || b == b'\t')
    {
        line_start
    } else {
        keyword_byte
    }
}

/// The class or interface a declaration at `byte` belongs to: the innermost
/// one the grammar gave, or else the `CLASS ... DEFINITION` the statements
/// have open, which `reconcile_methods` recovers as a class when the grammar
/// lost it whole (`drop_unparented_declarations` drops the declarations of a
/// definition it does not recover).
fn declaring_block(
    file_path: &str,
    classes: &[(usize, usize, String)],
    open_class: Option<&ClassBlock>,
    byte: usize,
) -> Option<String> {
    innermost_class(classes, byte).map(str::to_string).or_else(|| {
        open_class
            .filter(|c| !c.implementation)
            .map(|c| build_entity_id(file_path, "class", &c.name.text, None))
    })
}

/// Drop a `type`, `constant` or `variable` read off the statements whose
/// parent is no entity: a definition the grammar lost and `reconcile_methods`
/// did not recover (no implementation with a method).
fn drop_unparented_declarations(entities: &[SemanticEntity], found: &mut Vec<SemanticEntity>) {
    let ids: std::collections::HashSet<String> = entities
        .iter()
        .chain(found.iter())
        .map(|e| e.id.clone())
        .collect();
    found.retain(|f| {
        !matches!(f.entity_type.as_str(), "type" | "constant" | "variable")
            || f.parent_id.as_ref().is_none_or(|p| ids.contains(p))
    });
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
    let mut entity = text_entity(file_path, source, entity_type, name, parent_id, start_byte, end_byte);
    entity.metadata = Some(BTreeMap::from([(
        METADATA_SOURCE.0.to_string(),
        METADATA_SOURCE.1.to_string(),
    )]));
    entity
}

/// An entity with no node of its own to hash, so its hashes are over its text
/// (see `structural_hash`). Untagged: the grammar's `TYPES` and `CONSTANTS`
/// (`abap_declarations`) are built here too, from the nodes that span them.
pub(super) fn text_declaration(
    file_path: &str,
    source: &[u8],
    entity_type: &str,
    name: (usize, usize),
    parent_id: &str,
    start_byte: usize,
    end_byte: usize,
) -> SemanticEntity {
    let word = Word {
        text: String::from_utf8_lossy(&source[name.0..name.1]).trim().to_string(),
        start_byte: name.0,
        end_byte: name.1,
    };
    text_entity(file_path, source, entity_type, word, Some(parent_id), start_byte, end_byte)
}

fn text_entity(
    file_path: &str,
    source: &[u8],
    entity_type: &str,
    name: Word,
    parent_id: Option<&str>,
    start_byte: usize,
    end_byte: usize,
) -> SemanticEntity {
    let content = String::from_utf8_lossy(&source[start_byte..end_byte]).into_owned();
    let structural = structural_hash(source, &name, start_byte, end_byte);
    SemanticEntity {
        id: build_entity_id(file_path, entity_type, &name.text, parent_id),
        file_path: file_path.to_string(),
        entity_type: entity_type.to_string(),
        name: name.text,
        parent_id: parent_id.map(String::from),
        content_hash: content_hash(&content),
        structural_hash: Some(structural),
        kappa: None,
        content,
        start_line: line_number_for_byte(source, start_byte),
        end_line: line_number_for_byte(source, end_byte.saturating_sub(1)),
        start_byte: Some(start_byte),
        end_byte: Some(end_byte),
        metadata: None,
    }
}

/// No single node spans the entity, so the structural hash is over its text
/// with whitespace collapsed and the name left out, and kappa is skipped, as
/// for Swift's recovered containers.
fn structural_hash(source: &[u8], name: &Word, start_byte: usize, end_byte: usize) -> String {
    let before = String::from_utf8_lossy(&source[start_byte..name.start_byte.max(start_byte)]);
    let after = String::from_utf8_lossy(&source[name.end_byte.min(end_byte)..end_byte]);
    let normalized = before
        .split_whitespace()
        .chain(after.split_whitespace())
        .collect::<Vec<_>>()
        .join(" ");
    content_hash(&normalized)
}

#[cfg(test)]
mod include_tests {
    use super::include_names;

    #[test]
    fn include_names_reads_statements() {
        let src = "*  INCLUDE zcomment.\nINCLUDE zfoo_f01.\nINCLUDE: zb, zc.\n\
                   DATA: BEGIN OF s.\n  INCLUDE STRUCTURE zfx_order.\nDATA END OF s.\n\
                   TYPES: BEGIN OF t.\n  INCLUDE TYPE zfx_x.\nTYPES END OF t.\n\
                   INCLUDE <icon>.\nINCLUDE zopt IF FOUND.\nWRITE 'INCLUDE zstring.'.\n";
        assert_eq!(include_names(src), ["zfoo_f01", "zb", "zc", "zopt"]);
    }
}
