//! ABAP `TYPES` and `CONSTANTS`, read off the grammar's nodes.
//!
//! The grammar reads both, single (`types_declaration`, `constants_declaration`)
//! and chained (`chained_types_declaration` of `type_definition`s,
//! `chained_constants_declaration` of `constant`s), in a class's sections and
//! in an interface. A chain is flat: `BEGIN OF s`, the components and `END OF s`
//! are parts of their own (`structure_begin`, `structure_end`), and so are the
//! single statements of the classic form (`TYPES BEGIN OF s.` ... `TYPES END OF
//! s.`), which are siblings. So the entities come from a pass over a section's
//! or an interface's children with a little state, not from one node each: a
//! declaration is an entity per name, and a structure is one entity named
//! after it, from its `BEGIN OF` to its `END OF`, its components not entities.
//!
//! `TYPES` and `CONSTANTS` in a method body or a program are not entities, the
//! same as `DATA` there, so nothing walks to them; only the children of a
//! section or an interface are read. What the grammar loses to error recovery
//! the fallback pass reads off the statements (`abap_fallback.rs`), and skips
//! whatever this pass already gave.

use tree_sitter::Node;

use super::abap_fallback::text_declaration;
use crate::model::entity::SemanticEntity;

/// Nodes that declare types or constants: the generic walk leaves them to
/// `push_abap_declarations`.
pub(super) fn is_declaration_node(kind: &str) -> bool {
    family(kind).is_some()
}

#[derive(Clone, Copy, PartialEq)]
enum Family {
    Types,
    Constants,
}

fn family(kind: &str) -> Option<Family> {
    match kind {
        "types_declaration" | "chained_types_declaration" => Some(Family::Types),
        "constants_declaration" | "chained_constants_declaration" => Some(Family::Constants),
        _ => None,
    }
}

impl Family {
    fn entity_type(self) -> &'static str {
        match self {
            Family::Types => "type",
            Family::Constants => "constant",
        }
    }
}

/// A `BEGIN OF` whose `END OF` has not been seen yet.
struct OpenStructure {
    name: (usize, usize),
    start_byte: usize,
    depth: usize,
}

enum Part {
    Begin(Option<(usize, usize)>),
    End,
    Include,
    Item(Option<(usize, usize)>),
}

/// Push the `type` and `constant` entities among a section's or an
/// interface's children, nested under `parent_id`.
pub(super) fn push_abap_declarations(
    container: Node,
    file_path: &str,
    source: &[u8],
    parent_id: &str,
    entities: &mut Vec<SemanticEntity>,
) {
    let mut open: [Option<OpenStructure>; 2] = [None, None];
    let slot = |f: Family| usize::from(f == Family::Constants);

    let mut cursor = container.walk();
    for child in container.named_children(&mut cursor) {
        if child.kind().ends_with("comment") {
            continue;
        }
        // A statement of another kind ends the structure a family left open,
        // as it does in the fallback's token reading.
        for f in [Family::Types, Family::Constants] {
            if family(child.kind()) != Some(f) || child.has_error() {
                open[slot(f)] = None;
            }
        }
        let Some(fam) = family(child.kind()) else {
            continue;
        };
        if child.has_error() {
            continue;
        }
        let state = &mut open[slot(fam)];

        let chained = child.kind().starts_with("chained_");
        let mut parts: Vec<Node> = Vec::new();
        if chained {
            let mut c = child.walk();
            parts.extend(child.named_children(&mut c).filter(|p| !p.kind().ends_with("comment")));
        } else {
            parts.push(child);
        }
        let last = parts.len().saturating_sub(1);
        for (i, part) in parts.iter().enumerate() {
            // The first part starts at the keyword, the others at their first
            // character; the last ends at the period, the others at the comma.
            let start_byte = if i == 0 {
                child.start_byte()
            } else {
                skip_space(source, part.start_byte())
            };
            let end_byte = if i == last {
                child.end_byte()
            } else {
                include_comma(source, part.end_byte())
            };
            match classify(*part, source) {
                Part::Item(Some(name)) if state.is_none() => entities.push(text_declaration(
                    file_path,
                    source,
                    fam.entity_type(),
                    name,
                    parent_id,
                    start_byte,
                    end_byte,
                )),
                Part::Begin(name) => match (state.as_mut(), name) {
                    (Some(o), _) => o.depth += 1,
                    (None, Some(name)) => {
                        *state = Some(OpenStructure {
                            name,
                            start_byte,
                            depth: 1,
                        });
                    }
                    // A structure with no name node (`BEGIN OF ENUM e`) is the
                    // fallback's to read.
                    (None, None) => {}
                },
                Part::End => {
                    if let Some(o) = state.as_mut() {
                        o.depth -= 1;
                        if o.depth == 0 {
                            if let Some(o) = state.take() {
                                entities.push(text_declaration(
                                    file_path,
                                    source,
                                    fam.entity_type(),
                                    o.name,
                                    parent_id,
                                    o.start_byte,
                                    end_byte,
                                ));
                            }
                        }
                    }
                }
                Part::Item(_) | Part::Include => {}
            }
        }
    }
}

/// What a part of a declaration is. A single statement is its own part: the
/// node holds the `structure_begin`, `structure_end` or `structure_include`
/// as a child, or the name.
fn classify(part: Node, source: &[u8]) -> Part {
    let name_of = |node: Node| {
        node.child_by_field_name("name")
            .map(|n| trim_range(source, n.start_byte(), n.end_byte()))
    };
    match part.kind() {
        "structure_begin" => Part::Begin(name_of(part)),
        "structure_end" => Part::End,
        "structure_include" => Part::Include,
        _ => {
            let mut c = part.walk();
            let structural = part
                .named_children(&mut c)
                .find(|n| n.kind().starts_with("structure_"));
            match structural {
                Some(n) => classify(n, source),
                None => Part::Item(name_of(part)),
            }
        }
    }
}

fn skip_space(source: &[u8], mut at: usize) -> usize {
    while source.get(at).is_some_and(u8::is_ascii_whitespace) {
        at += 1;
    }
    at
}

/// A part's node stops before its `,`; the entity of a part ends after it.
fn include_comma(source: &[u8], end: usize) -> usize {
    let at = skip_space(source, end);
    if source.get(at) == Some(&b',') {
        at + 1
    } else {
        end
    }
}

fn trim_range(source: &[u8], start: usize, end: usize) -> (usize, usize) {
    let start = skip_space(source, start).min(end);
    let mut end = end;
    while end > start && source[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    (start, end)
}
