use tree_sitter::{Node, Tree, TreeCursor};

use super::LanguageExtractor;
use crate::indexer::{ExtractedRef, ExtractedSymbol, RefKind, SymbolKind};

pub struct PythonExtractor;

impl LanguageExtractor for PythonExtractor {
    fn language(&self) -> tree_sitter::Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn extensions(&self) -> &[&str] {
        &["py", "pyi"]
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
            "function_definition" => {
                if let Some(sym) = extract_function(child, source, false) {
                    symbols.push(sym);
                }
            }
            "class_definition" => {
                if let Some(sym) = extract_class(child, source) {
                    symbols.push(sym);
                }
            }
            "decorated_definition" => {
                if let Some(inner) = child.child_by_field_name("definition") {
                    match inner.kind() {
                        "function_definition" => {
                            if let Some(sym) = extract_function(inner, source, false) {
                                symbols.push(sym);
                            }
                        }
                        "class_definition" => {
                            if let Some(sym) = extract_class(inner, source) {
                                symbols.push(sym);
                            }
                        }
                        _ => {}
                    }
                }
            }
            "expression_statement" => {
                if let Some(sym) = extract_module_constant(child, source) {
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

fn extract_function(node: Node, source: &[u8], is_method: bool) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let sig = build_signature(node, source);
    let doc = get_docstring(node, source);

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

fn extract_class(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let name = node.child_by_field_name("name").map(|n| text(n, source))?;
    let doc = get_docstring(node, source);

    let mut children = Vec::new();
    let mut refs = Vec::new();

    // Extract base classes
    if let Some(superclasses) = node.child_by_field_name("superclasses") {
        let mut cursor = superclasses.walk();
        for child in superclasses.children(&mut cursor) {
            if child.kind() == "identifier" || child.kind() == "attribute" {
                refs.push(ExtractedRef {
                    name: text(child, source),
                    kind: RefKind::Extends,
                    line: child.start_position().row + 1,
                });
            }
        }
    }

    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(sym) = extract_function(child, source, true) {
                        children.push(sym);
                    }
                }
                "decorated_definition" => {
                    if let Some(inner) = child.child_by_field_name("definition") {
                        if inner.kind() == "function_definition" {
                            if let Some(sym) = extract_function(inner, source, true) {
                                children.push(sym);
                            }
                        }
                    }
                }
                "expression_statement" => {
                    if let Some(sym) = extract_class_constant(child, source) {
                        children.push(sym);
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

fn build_signature(node: Node, source: &[u8]) -> String {
    let params = node
        .child_by_field_name("parameters")
        .map(|n| text(n, source))
        .unwrap_or_default();
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text(n, source))
        .unwrap_or_default();
    if return_type.is_empty() {
        params
    } else {
        format!("{params} -> {return_type}")
    }
}

fn get_docstring(node: Node, source: &[u8]) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let first_child = body.children(&mut cursor).next()?;

    if first_child.kind() != "expression_statement" {
        return None;
    }

    let mut inner_cursor = first_child.walk();
    let string_node = first_child.children(&mut inner_cursor).next()?;
    if string_node.kind() != "string" {
        return None;
    }

    let raw = text(string_node, source);
    let cleaned = strip_triple_quotes(&raw);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn strip_triple_quotes(s: &str) -> String {
    let s = s.trim();
    let is_triple_quoted = (s.starts_with("\"\"\"") && s.ends_with("\"\"\""))
        || (s.starts_with("'''") && s.ends_with("'''"));
    if !is_triple_quoted {
        return String::new();
    }
    let stripped = &s[3..s.len() - 3];
    stripped
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_upper_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

fn extract_module_constant(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let mut cursor = node.walk();
    let child = node.children(&mut cursor).next()?;
    if child.kind() != "assignment" {
        return None;
    }
    let left = child.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    let name = text(left, source);
    if !is_upper_snake_case(&name) {
        return None;
    }
    let type_node = child.child_by_field_name("type");
    let sig = type_node.map(|n| text(n, source));
    Some(ExtractedSymbol {
        name,
        kind: SymbolKind::Const,
        signature: sig,
        doc_comment: None,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: Vec::new(),
    })
}

fn extract_class_constant(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let mut cursor = node.walk();
    let child = node.children(&mut cursor).next()?;
    if child.kind() != "assignment" {
        return None;
    }
    let left = child.child_by_field_name("left")?;
    if left.kind() != "identifier" {
        return None;
    }
    let name = text(left, source);
    if !is_upper_snake_case(&name) {
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
            if let Some(func) = node.child_by_field_name("function") {
                let name = match func.kind() {
                    "identifier" => text(func, source),
                    "attribute" => {
                        let attr = func
                            .child_by_field_name("attribute")
                            .map(|n| text(n, source));
                        let obj = func.child_by_field_name("object").map(|n| text(n, source));
                        match (obj, attr) {
                            (Some(o), Some(a)) => format!("{o}.{a}"),
                            (None, Some(a)) => a,
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
