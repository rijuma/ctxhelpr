pub mod typescript;

use crate::indexer::ExtractedSymbol;

pub trait LanguageExtractor: Send + Sync {
    fn language(&self) -> tree_sitter::Language;
    fn extensions(&self) -> &[&str];
    fn extract(&self, source: &[u8], tree: &tree_sitter::Tree) -> Vec<ExtractedSymbol>;
}

pub fn detect_language(ext: &str) -> Option<&'static str> {
    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some("typescript"),
        "py" | "pyi" => Some("python"),
        "rs" => Some("rust"),
        _ => None,
    }
}
