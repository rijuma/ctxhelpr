use tree_sitter::{Node, Tree};

use super::LanguageExtractor;
use crate::indexer::{ExtractedSymbol, SymbolKind};

pub struct MarkdownExtractor;

impl LanguageExtractor for MarkdownExtractor {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_md::LANGUAGE.into()
    }

    fn extensions(&self) -> &[&str] {
        &["md", "markdown"]
    }

    fn extract(&self, source: &[u8], tree: &Tree) -> Vec<ExtractedSymbol> {
        let mut flat_headings = Vec::new();
        collect_headings(tree.root_node(), source, &mut flat_headings);
        build_hierarchy(flat_headings)
    }
}

struct FlatHeading {
    name: String,
    level: usize,
    start_line: usize,
    end_line: usize,
}

fn collect_headings(node: Node, source: &[u8], headings: &mut Vec<FlatHeading>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "atx_heading" {
            if let Some(heading) = parse_heading(child, source) {
                headings.push(heading);
            }
        } else if child.kind() == "section" {
            collect_headings(child, source, headings);
        }
    }
}

fn parse_heading(node: Node, source: &[u8]) -> Option<FlatHeading> {
    let mut level = 0;
    let mut name = String::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "atx_h1_marker" => level = 1,
            "atx_h2_marker" => level = 2,
            "atx_h3_marker" => level = 3,
            "atx_h4_marker" => level = 4,
            "atx_h5_marker" => level = 5,
            "atx_h6_marker" => level = 6,
            "heading_content" | "inline" => {
                name = child.utf8_text(source).unwrap_or("").trim().to_string();
            }
            _ => {}
        }
    }

    if level == 0 {
        // Fallback: try to extract from raw text
        let raw = node.utf8_text(source).unwrap_or("").trim().to_string();
        let trimmed = raw.trim_start_matches('#').trim();
        if !trimmed.is_empty() {
            level = raw.len() - raw.trim_start_matches('#').len();
            name = trimmed.to_string();
        }
    }

    if level == 0 || name.is_empty() {
        return None;
    }

    Some(FlatHeading {
        name,
        level,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
    })
}

fn build_hierarchy(headings: Vec<FlatHeading>) -> Vec<ExtractedSymbol> {
    if headings.is_empty() {
        return Vec::new();
    }

    // Stack-based approach: each entry is (level, symbol)
    let mut stack: Vec<(usize, ExtractedSymbol)> = Vec::new();
    let mut result: Vec<ExtractedSymbol> = Vec::new();

    for heading in headings {
        let sym = ExtractedSymbol {
            name: heading.name,
            kind: SymbolKind::Section,
            signature: None,
            doc_comment: None,
            start_line: heading.start_line,
            end_line: heading.end_line,
            children: Vec::new(),
            references: Vec::new(),
        };

        // Pop items from stack with level >= current (they can't be parents)
        while let Some((top_level, _)) = stack.last() {
            if *top_level >= heading.level {
                let (_, popped) = stack.pop().unwrap();
                if let Some((_, parent)) = stack.last_mut() {
                    parent.children.push(popped);
                } else {
                    result.push(popped);
                }
            } else {
                break;
            }
        }

        stack.push((heading.level, sym));
    }

    // Flush remaining stack
    while let Some((_, popped)) = stack.pop() {
        if let Some((_, parent)) = stack.last_mut() {
            parent.children.push(popped);
        } else {
            result.push(popped);
        }
    }

    result
}
