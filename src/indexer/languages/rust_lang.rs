use tree_sitter::{Node, Tree, TreeCursor};

use super::LanguageExtractor;
use crate::indexer::{ExtractedRef, ExtractedSymbol, RefKind, SymbolKind};

pub struct RustExtractor;

impl LanguageExtractor for RustExtractor {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn extensions(&self) -> &[&str] {
        &["rs"]
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
            "function_item" => {
                if let Some(sym) = extract_function(child, source, false) {
                    symbols.push(sym);
                }
            }
            "struct_item" => {
                if let Some(sym) = extract_struct(child, source) {
                    symbols.push(sym);
                }
            }
            "enum_item" => {
                if let Some(sym) = extract_enum(child, source) {
                    symbols.push(sym);
                }
            }
            "trait_item" => {
                if let Some(sym) = extract_trait(child, source) {
                    symbols.push(sym);
                }
            }
            "impl_item" => {
                if let Some(sym) = extract_impl(child, source) {
                    symbols.push(sym);
                }
            }
            "mod_item" => {
                if let Some(sym) = extract_mod(child, source) {
                    symbols.push(sym);
                }
            }
            "type_item" => {
                if let Some(sym) = extract_type_alias(child, source) {
                    symbols.push(sym);
                }
            }
            "const_item" | "static_item" => {
                if let Some(sym) = extract_const(child, source) {
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
        if s.kind() == "line_comment" {
            let t = text(s, source);
            if let Some(stripped) = t.strip_prefix("///") {
                lines.push(stripped.trim().to_string());
                sibling = s.prev_sibling();
                continue;
            }
        }
        break;
    }
    if lines.is_empty() {
        return None;
    }
    lines.reverse();
    Some(lines.join("\n"))
}

fn build_fn_signature(node: Node, source: &[u8]) -> String {
    let type_params = node
        .child_by_field_name("type_parameters")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let params = node
        .child_by_field_name("parameters")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let mut sig = String::new();
    if !type_params.is_empty() {
        sig.push_str(&type_params);
    }
    sig.push_str(&params);
    if !return_type.is_empty() {
        sig.push_str(&format!(" {return_type}"));
    }
    sig
}

fn extract_function(node: Node, source: &[u8], is_method: bool) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let sig = build_fn_signature(node, source);
    let doc = get_doc_comment(node, source);

    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls(body, source, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: if is_method {
            SymbolKind::Method
        } else {
            SymbolKind::Fn
        },
        signature: Some(sig),
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: refs,
    })
}

fn extract_struct(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                if let Some(field_name) = child.child_by_field_name("name") {
                    let type_sig = child.child_by_field_name("type").map(|n| text(n, source));
                    children.push(ExtractedSymbol {
                        name: text(field_name, source),
                        kind: SymbolKind::Var,
                        signature: type_sig,
                        doc_comment: None,
                        start_line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        children: Vec::new(),
                        references: Vec::new(),
                    });
                }
            }
        }
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Struct,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: Vec::new(),
    })
}

fn extract_enum(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                if let Some(variant_name) = child.child_by_field_name("name") {
                    children.push(ExtractedSymbol {
                        name: text(variant_name, source),
                        kind: SymbolKind::Const,
                        signature: None,
                        doc_comment: None,
                        start_line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        children: Vec::new(),
                        references: Vec::new(),
                    });
                }
            }
        }
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Enum,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: Vec::new(),
    })
}

fn extract_trait(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" || child.kind() == "function_signature_item" {
                if let Some(sym) = extract_function(child, source, true) {
                    children.push(sym);
                }
            }
        }
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Trait,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: Vec::new(),
    })
}

fn extract_impl(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let type_node = node.child_by_field_name("type")?;
    let type_name = text(type_node, source);

    let trait_node = node.child_by_field_name("trait");
    let name = match trait_node {
        Some(t) => format!("{} for {}", text(t, source), type_name),
        None => type_name,
    };

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "function_item" => {
                    if let Some(sym) = extract_function(child, source, true) {
                        children.push(sym);
                    }
                }
                "const_item" => {
                    if let Some(sym) = extract_const(child, source) {
                        children.push(sym);
                    }
                }
                "type_item" => {
                    if let Some(sym) = extract_type_alias(child, source) {
                        children.push(sym);
                    }
                }
                _ => {}
            }
        }
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Impl,
        signature: None,
        doc_comment: None,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: Vec::new(),
    })
}

fn extract_mod(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_top_level(body, source, &mut children);
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Mod,
        signature: None,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children,
        references: Vec::new(),
    })
}

fn extract_type_alias(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);
    let value = node.child_by_field_name("type").map(|n| text(n, source));

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Type,
        signature: value,
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: Vec::new(),
    })
}

fn extract_const(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);
    let type_sig = node.child_by_field_name("type").map(|n| text(n, source));

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Const,
        signature: type_sig,
        doc_comment: doc,
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
        if node.kind() == "call_expression" {
            if let Some(func) = node.child_by_field_name("function") {
                let name = match func.kind() {
                    "identifier" => text(func, source),
                    "field_expression" => {
                        let field = func.child_by_field_name("field").map(|n| text(n, source));
                        let obj = func.child_by_field_name("value").map(|n| text(n, source));
                        match (obj, field) {
                            (Some(o), Some(f)) => format!("{o}.{f}"),
                            (None, Some(f)) => f,
                            _ => text(func, source),
                        }
                    }
                    "scoped_identifier" => text(func, source),
                    _ => text(func, source),
                };
                if !name.is_empty() {
                    refs.push(ExtractedRef {
                        name,
                        kind: RefKind::Call,
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
