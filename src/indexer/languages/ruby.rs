use tree_sitter::{Node, Tree, TreeCursor};

use super::LanguageExtractor;
use crate::indexer::{ExtractedRef, ExtractedSymbol, RefKind, SymbolKind};

pub struct RubyExtractor;

impl LanguageExtractor for RubyExtractor {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_ruby::LANGUAGE.into()
    }

    fn extensions(&self) -> &[&str] {
        &["rb"]
    }

    fn extract(&self, source: &[u8], tree: &Tree) -> Vec<ExtractedSymbol> {
        let mut symbols = Vec::new();
        extract_top_level(tree.root_node(), source, &mut symbols);
        symbols
    }
}

fn extract_top_level(node: Node, source: &[u8], symbols: &mut Vec<ExtractedSymbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "method" => {
                if let Some(sym) = extract_method(child, source, false) {
                    symbols.push(sym);
                }
            }
            "singleton_method" => {
                if let Some(sym) = extract_singleton_method(child, source) {
                    symbols.push(sym);
                }
            }
            "class" => {
                if let Some(sym) = extract_class(child, source) {
                    symbols.push(sym);
                }
            }
            "module" => {
                if let Some(sym) = extract_module(child, source) {
                    symbols.push(sym);
                }
            }
            "assignment" => {
                if let Some(sym) = extract_constant(child, source) {
                    symbols.push(sym);
                }
            }
            _ => {}
        }
    }
}

fn text(node: Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

fn get_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let mut lines = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == "comment" {
            let t = text(s, source);
            let stripped = t.trim_start_matches('#').trim();
            lines.push(stripped.to_string());
            sibling = s.prev_sibling();
            continue;
        }
        break;
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

fn extract_method(node: Node, source: &[u8], is_in_class: bool) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let params = node
        .child_by_field_name("parameters")
        .map(|n| text(n, source));
    let doc = get_doc_comment(node, source);

    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls(body, source, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: if is_in_class {
            SymbolKind::Method
        } else {
            SymbolKind::Fn
        },
        signature: params,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: refs,
    })
}

fn extract_singleton_method(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let object = node
        .child_by_field_name("object")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let full_name = if object.is_empty() {
        name
    } else {
        format!("{object}.{name}")
    };
    let params = node
        .child_by_field_name("parameters")
        .map(|n| text(n, source));
    let doc = get_doc_comment(node, source);

    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls(body, source, &mut refs);
    }

    Some(ExtractedSymbol {
        name: full_name,
        kind: SymbolKind::Method,
        signature: params,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: refs,
    })
}

fn extract_class(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    let mut refs = Vec::new();

    // Superclass — the node includes the "<" token, so find the actual class name
    if let Some(superclass) = node.child_by_field_name("superclass") {
        let mut sc_cursor = superclass.walk();
        for sc_child in superclass.children(&mut sc_cursor) {
            if sc_child.kind() == "constant" || sc_child.kind() == "scope_resolution" {
                refs.push(ExtractedRef {
                    name: text(sc_child, source),
                    kind: RefKind::Extends,
                    line: sc_child.start_position().row + 1,
                });
                break;
            }
        }
    }

    if let Some(body) = node.child_by_field_name("body") {
        extract_class_body(body, source, &mut children, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Class,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: refs,
    })
}

fn extract_class_body(
    node: Node,
    source: &[u8],
    children: &mut Vec<ExtractedSymbol>,
    refs: &mut Vec<ExtractedRef>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "method" => {
                if let Some(sym) = extract_method(child, source, true) {
                    children.push(sym);
                }
            }
            "singleton_method" => {
                if let Some(sym) = extract_singleton_method(child, source) {
                    children.push(sym);
                }
            }
            "assignment" => {
                if let Some(sym) = extract_constant(child, source) {
                    children.push(sym);
                }
            }
            "call" => {
                // Handle include/extend/require at class level
                if let Some(method_name) = child.child_by_field_name("method") {
                    let mname = text(method_name, source);
                    match mname.as_str() {
                        "include" | "extend" => {
                            if let Some(args) = child.child_by_field_name("arguments") {
                                let mut ac = args.walk();
                                for arg in args.children(&mut ac) {
                                    if arg.kind() == "constant" || arg.kind() == "scope_resolution"
                                    {
                                        refs.push(ExtractedRef {
                                            name: text(arg, source),
                                            kind: RefKind::Extends,
                                            line: arg.start_position().row + 1,
                                        });
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_module(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_class_body(body, source, &mut children, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Mod,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: refs,
    })
}

fn is_ruby_constant(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_uppercase())
}

fn extract_constant(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let left = node.child_by_field_name("left")?;
    let name = match left.kind() {
        "constant" | "identifier" => text(left, source),
        "scope_resolution" => text(left, source),
        _ => return None,
    };
    if !is_ruby_constant(&name) {
        return None;
    }
    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Const,
        signature: None,
        doc_comment: None,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: Vec::new(),
    })
}

fn extract_calls(node: Node, source: &[u8], refs: &mut Vec<ExtractedRef>) {
    let mut cursor = node.walk();
    extract_calls_recursive(&mut cursor, source, refs);
}

fn extract_calls_recursive(cursor: &mut TreeCursor, source: &[u8], refs: &mut Vec<ExtractedRef>) {
    loop {
        let node = cursor.node();
        if node.kind() == "call" {
            if let Some(method) = node.child_by_field_name("method") {
                let name = text(method, source);
                let receiver = node
                    .child_by_field_name("receiver")
                    .map(|n| text(n, source));
                let full_name = match receiver {
                    Some(r) => format!("{r}.{name}"),
                    None => name,
                };
                if !full_name.is_empty() {
                    let kind = match full_name.as_str() {
                        "require" | "require_relative" => RefKind::Import,
                        _ => RefKind::Call,
                    };
                    refs.push(ExtractedRef {
                        name: full_name,
                        kind,
                        line: node.start_position().row + 1,
                    });
                }
            }
        }

        if cursor.goto_first_child() {
            extract_calls_recursive(cursor, source, refs);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
