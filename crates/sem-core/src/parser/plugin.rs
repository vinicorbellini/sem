use crate::model::entity::SemanticEntity;

/// What one file's parse yielded, for reporting how much of it the parser could
/// not read. A file with `error_node_count > 0` still has its entities; this
/// says how much of the file they were found around.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct ParseStats {
    /// Every entity extracted, `grammar_entity_count + fallback_entity_count`.
    pub entity_count: usize,
    /// Entities the grammar's own nodes gave.
    pub grammar_entity_count: usize,
    /// Entities a plugin's fallback pass read off the tokens instead
    /// (`source: abap-fallback` in their metadata).
    pub fallback_entity_count: usize,
    /// Tree-sitter `ERROR` and `MISSING` nodes. Always 0 for a plugin that
    /// does not parse with tree-sitter.
    pub error_node_count: usize,
}

pub trait SemanticParserPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn extract_entities(&self, content: &str, file_path: &str) -> Vec<SemanticEntity>;
    fn extract_entities_brief(&self, content: &str, file_path: &str) -> Vec<SemanticEntity> {
        let mut entities = self.extract_entities(content, file_path);
        strip_entity_payloads(&mut entities);
        entities
    }
    /// Extract entities and optionally return the tree-sitter Tree for reuse.
    /// Default returns None for the tree (non-code plugins).
    fn extract_entities_with_tree(
        &self,
        content: &str,
        file_path: &str,
    ) -> (Vec<SemanticEntity>, Option<tree_sitter::Tree>) {
        (self.extract_entities(content, file_path), None)
    }
    /// Entity and parse-error counts for one file. The default has no parse
    /// tree to count errors in, so it reports the entities and zero errors.
    fn parse_stats(&self, content: &str, file_path: &str) -> ParseStats {
        let entity_count = self.extract_entities(content, file_path).len();
        ParseStats {
            entity_count,
            grammar_entity_count: entity_count,
            fallback_entity_count: 0,
            error_node_count: 0,
        }
    }
    fn structural_hash_content(&self, _content: &str, _file_path: &str) -> Option<String> {
        None
    }
    fn compute_similarity(&self, a: &SemanticEntity, b: &SemanticEntity) -> f64 {
        crate::model::identity::default_similarity(a, b)
    }
}

pub fn strip_entity_payloads(entities: &mut [SemanticEntity]) {
    for entity in entities {
        entity.content.clear();
        entity.content_hash.clear();
        entity.structural_hash = None;
    }
}
