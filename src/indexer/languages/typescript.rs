use tree_sitter::{Node, Tree, TreeCursor};

use super::LanguageExtractor;
use crate::indexer::{ExtractedRef, ExtractedSymbol, RefKind, SymbolKind};

pub struct TypeScriptExtractor;

impl LanguageExtractor for TypeScriptExtractor {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn extensions(&self) -> &[&str] {
        &["ts", "tsx", "js", "jsx", "mjs", "cjs"]
    }

    fn extract(&self, source: &[u8], tree: &Tree) -> Vec<ExtractedSymbol> {
        let mut symbols = Vec::new();
        let root = tree.root_node();
        extract_from_node(root, source, &mut symbols);
        symbols
    }
}

fn extract_from_node(node: Node, source: &[u8], symbols: &mut Vec<ExtractedSymbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if let Some(sym) = extract_function(child, source) {
                    symbols.push(sym);
                }
            }
            "class_declaration" => {
                if let Some(sym) = extract_class(child, source) {
                    symbols.push(sym);
                }
            }
            "interface_declaration" => {
                if let Some(sym) = extract_interface(child, source) {
                    symbols.push(sym);
                }
            }
            "type_alias_declaration" => {
                if let Some(sym) = extract_type_alias(child, source) {
                    symbols.push(sym);
                }
            }
            "enum_declaration" => {
                if let Some(sym) = extract_enum(child, source) {
                    symbols.push(sym);
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_variable_declarations(child, source, symbols);
            }
            "export_statement" => {
                // Recurse into export to find the actual declaration
                extract_from_node(child, source, symbols);
            }
            "import_statement" => {
                // We track imports as references, not symbols
            }
            _ => {}
        }
    }
}

fn text(node: Node, source: &[u8]) -> String {
    node.utf8_text(source).unwrap_or("").to_string()
}

fn build_signature(node: Node, source: &[u8]) -> String {
    let params = node
        .child_by_field_name("parameters")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text(n, source))
        .unwrap_or_default();
    format!("{params}{return_type}")
}

fn get_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let mut prev = node.prev_sibling();
    // Skip export_statement wrapper
    if node.parent().map(|p| p.kind()) == Some("export_statement") {
        prev = node.parent().and_then(|p| p.prev_sibling());
    }
    if let Some(comment_node) = prev {
        if comment_node.kind() == "comment" {
            let t = text(comment_node, source);
            if t.starts_with("/**") {
                let cleaned = t
                    .trim_start_matches("/**")
                    .trim_end_matches("*/")
                    .lines()
                    .map(|l| l.trim().trim_start_matches('*').trim())
                    .filter(|l| !l.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                if !cleaned.is_empty() {
                    return Some(cleaned);
                }
            }
        }
    }
    None
}

fn extract_function(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let sig = build_signature(node, source);
    let doc = get_doc_comment(node, source);

    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls(body, source, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Fn,
        signature: Some(sig),
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

    // Check for extends/implements
    // type_parameters are separate from heritage; heritage is handled below
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "class_heritage" {
            let heritage_text = text(child, source);
            if heritage_text.contains("extends") || heritage_text.contains("implements") {
                // Extract referenced type names
                let mut hc = child.walk();
                for hchild in child.children(&mut hc) {
                    if hchild.kind() == "identifier" || hchild.kind() == "type_identifier" {
                        refs.push(ExtractedRef {
                            name: text(hchild, source),
                            kind: RefKind::Extends,
                            line: hchild.start_position().row + 1,
                        });
                    }
                }
            }
        }
    }

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            match member.kind() {
                "method_definition" => {
                    if let Some(method) = extract_method(member, source) {
                        children.push(method);
                    }
                }
                "public_field_definition" | "property_definition" => {
                    if let Some(field) = extract_field(member, source) {
                        children.push(field);
                    }
                }
                _ => {}
            }
        }
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

fn extract_method(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let sig = build_signature(node, source);
    let doc = get_doc_comment(node, source);

    let mut refs = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls(body, source, &mut refs);
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Method,
        signature: Some(sig),
        doc_comment: doc,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: refs,
    })
}

fn extract_field(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let type_ann = node.child_by_field_name("type").map(|n| text(n, source));
    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Var,
        signature: type_ann,
        doc_comment: None,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: Vec::new(),
    })
}

fn extract_interface(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            match member.kind() {
                "property_signature" => {
                    if let Some(prop_name) = member.child_by_field_name("name") {
                        let type_ann = member.child_by_field_name("type").map(|n| text(n, source));
                        children.push(ExtractedSymbol {
                            name: text(prop_name, source),
                            kind: SymbolKind::Var,
                            signature: type_ann,
                            doc_comment: None,
                            start_line: member.start_position().row + 1,
                            end_line: member.end_position().row + 1,
                            children: Vec::new(),
                            references: Vec::new(),
                        });
                    }
                }
                "method_signature" => {
                    if let Some(method_name) = member.child_by_field_name("name") {
                        children.push(ExtractedSymbol {
                            name: text(method_name, source),
                            kind: SymbolKind::Method,
                            signature: Some(build_signature(member, source)),
                            doc_comment: None,
                            start_line: member.start_position().row + 1,
                            end_line: member.end_position().row + 1,
                            children: Vec::new(),
                            references: Vec::new(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Interface,
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
    let value = node.child_by_field_name("value").map(|n| text(n, source));
    let doc = get_doc_comment(node, source);

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

fn extract_enum(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_doc_comment(node, source);

    let mut children = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            if member.kind() == "enum_assignment" || member.kind() == "property_identifier" {
                let member_name = if member.kind() == "property_identifier" {
                    text(member, source)
                } else {
                    member
                        .child_by_field_name("name")
                        .map(|n| text(n, source))
                        .unwrap_or_default()
                };
                if !member_name.is_empty() {
                    children.push(ExtractedSymbol {
                        name: member_name,
                        kind: SymbolKind::Const,
                        signature: None,
                        doc_comment: None,
                        start_line: member.start_position().row + 1,
                        end_line: member.end_position().row + 1,
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

fn extract_variable_declarations(node: Node, source: &[u8], symbols: &mut Vec<ExtractedSymbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name_node = child.child_by_field_name("name");
            let value_node = child.child_by_field_name("value");

            if let (Some(name_n), Some(value_n)) = (name_node, value_node) {
                let name = text(name_n, source);
                match value_n.kind() {
                    "arrow_function" | "function" | "function_expression" => {
                        let sig = build_signature(value_n, source);
                        let doc = get_doc_comment(node, source);

                        let mut refs = Vec::new();
                        if let Some(body) = value_n.child_by_field_name("body") {
                            extract_calls(body, source, &mut refs);
                        }

                        symbols.push(ExtractedSymbol {
                            name,
                            kind: SymbolKind::Fn,
                            signature: Some(sig),
                            doc_comment: doc,
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            children: Vec::new(),
                            references: refs,
                        });
                    }
                    _ => {
                        // Regular variable/const
                        let type_ann = child.child_by_field_name("type").map(|n| text(n, source));
                        symbols.push(ExtractedSymbol {
                            name,
                            kind: SymbolKind::Const,
                            signature: type_ann,
                            doc_comment: get_doc_comment(node, source),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            children: Vec::new(),
                            references: Vec::new(),
                        });
                    }
                }
            }
        }
    }
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
                    "member_expression" => {
                        let prop = func
                            .child_by_field_name("property")
                            .map(|n| text(n, source));
                        let obj = func.child_by_field_name("object").map(|n| text(n, source));
                        match (obj, prop) {
                            (Some(o), Some(p)) => format!("{}.{}", o, p),
                            (None, Some(p)) => p,
                            _ => text(func, source),
                        }
                    }
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
