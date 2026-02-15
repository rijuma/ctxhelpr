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
    let mut import_refs = Vec::new();
    let mut test_refs = Vec::new();
    let mut file_start = usize::MAX;
    let mut file_end = 0usize;

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
                extract_export_statement(child, source, symbols);
            }
            "import_statement" => {
                let refs = extract_import_refs(child, source);
                if !refs.is_empty() {
                    let line = child.start_position().row + 1;
                    if line < file_start {
                        file_start = line;
                    }
                    let end = child.end_position().row + 1;
                    if end > file_end {
                        file_end = end;
                    }
                    import_refs.extend(refs);
                }
            }
            "ambient_declaration" => {
                // `declare module "name" { ... }` — recurse into body
                extract_ambient_declaration(child, source, symbols);
            }
            "expression_statement" => {
                // Top-level expressions like `describe(...)`, `test(...)`
                collect_callback_refs(child, source, &mut test_refs);
            }
            _ => {}
        }
    }

    // Create single _imports symbol if any import refs were collected
    if !import_refs.is_empty() {
        symbols.push(ExtractedSymbol {
            name: "_imports".to_string(),
            kind: SymbolKind::Mod,
            signature: None,
            doc_comment: None,
            start_line: file_start,
            end_line: file_end,
            children: Vec::new(),
            references: import_refs,
        });
    }

    // Create single _tests symbol if any test refs were collected
    if !test_refs.is_empty() {
        // Deduplicate refs by (name, kind)
        let mut seen = std::collections::HashSet::new();
        test_refs.retain(|r| seen.insert((r.name.clone(), r.kind.as_str().to_string())));

        symbols.push(ExtractedSymbol {
            name: "_tests".to_string(),
            kind: SymbolKind::Fn,
            signature: None,
            doc_comment: None,
            start_line: 1,
            end_line: node.end_position().row + 1,
            children: Vec::new(),
            references: test_refs,
        });
    }
}

/// Handle export statements, including:
/// - `export function/class/...` (recurse to find declaration)
/// - `export default <expression>` (synthetic symbol)
/// - `export { X, Y } from './mod'` (barrel re-exports)
/// - `export * from './mod'`
fn extract_export_statement(node: Node, source: &[u8], symbols: &mut Vec<ExtractedSymbol>) {
    let mut has_known_child = false;
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Standard declarations — recurse as before
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "lexical_declaration"
            | "variable_declaration" => {
                extract_from_node(node, source, symbols);
                return;
            }
            // Named re-exports: `export { X, Y } from './mod'`
            "export_clause" => {
                has_known_child = true;
                extract_export_clause(child, node, source, symbols);
            }
            // Namespace re-export: `export * from './mod'`
            "*" => {
                has_known_child = true;
                // Find the source module
                let mut c2 = node.walk();
                for sibling in node.children(&mut c2) {
                    if sibling.kind() == "string" || sibling.kind() == "string_fragment" {
                        let module = text(sibling, source)
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string();
                        symbols.push(ExtractedSymbol {
                            name: format!("* from {}", module),
                            kind: SymbolKind::Mod,
                            signature: None,
                            doc_comment: None,
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            children: Vec::new(),
                            references: Vec::new(),
                        });
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    // If no standard declaration was found, handle `export default <expression>`
    if !has_known_child {
        if let Some(sym) = extract_default_export(node, source) {
            symbols.push(sym);
        }
    }
}

/// Extract named re-exports: `export { X, Y } from './mod'` or `export { X as Z }`
fn extract_export_clause(
    clause: Node,
    export_node: Node,
    source: &[u8],
    symbols: &mut Vec<ExtractedSymbol>,
) {
    // Determine if there's a source module
    let source_module = find_string_child(export_node, source);

    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if child.kind() == "export_specifier" {
            let name_node = child
                .child_by_field_name("name")
                .or_else(|| child.named_child(0));
            let alias_node = child.child_by_field_name("alias");

            if let Some(name_n) = name_node {
                let original_name = text(name_n, source);
                let exported_name = alias_node
                    .map(|a| text(a, source))
                    .unwrap_or_else(|| original_name.clone());

                let mut refs = Vec::new();
                if source_module.is_some() {
                    refs.push(ExtractedRef {
                        name: original_name,
                        kind: RefKind::Import,
                        line: child.start_position().row + 1,
                    });
                }

                symbols.push(ExtractedSymbol {
                    name: exported_name,
                    kind: SymbolKind::Var,
                    signature: None,
                    doc_comment: None,
                    start_line: child.start_position().row + 1,
                    end_line: child.end_position().row + 1,
                    children: Vec::new(),
                    references: refs,
                });
            }
        }
    }
}

/// Handle `export default <expression>` — create a synthetic "default" symbol
/// and recurse into the expression for refs
fn extract_default_export(node: Node, source: &[u8]) -> Option<ExtractedSymbol> {
    let mut cursor = node.walk();
    let mut default_expr = None;

    for child in node.children(&mut cursor) {
        match child.kind() {
            // Skip keywords
            "export" | "default" | ";" => {}
            _ => {
                default_expr = Some(child);
                break;
            }
        }
    }

    let expr = default_expr?;

    let mut refs = Vec::new();
    extract_calls(expr, source, &mut refs);

    // Also recurse into arrow functions / function expressions inside calls
    // e.g., `export default fp(async (fastify) => { ... })`
    extract_nested_callback_refs(expr, source, &mut refs);

    Some(ExtractedSymbol {
        name: "default".to_string(),
        kind: SymbolKind::Fn,
        signature: None,
        doc_comment: get_doc_comment(node, source),
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        children: Vec::new(),
        references: refs,
    })
}

/// Walk into callback arguments of call expressions to find deeper refs.
/// This handles patterns like `fp(async (fastify) => { body })`.
fn extract_nested_callback_refs(node: Node, source: &[u8], refs: &mut Vec<ExtractedRef>) {
    if node.kind() == "call_expression" {
        if let Some(args) = node.child_by_field_name("arguments") {
            let mut cursor = args.walk();
            for arg in args.children(&mut cursor) {
                if arg.kind() == "arrow_function"
                    || arg.kind() == "function_expression"
                    || arg.kind() == "function"
                {
                    if let Some(body) = arg.child_by_field_name("body") {
                        extract_calls(body, source, refs);
                    }
                }
            }
        }
    }
    // Recurse into children looking for nested call expressions
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() != "call_expression" {
            extract_nested_callback_refs(child, source, refs);
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
        match node.kind() {
            "call_expression" => {
                if let Some(func) = node.child_by_field_name("function") {
                    let name = extract_callable_name(func, source);
                    if !name.is_empty() {
                        refs.push(ExtractedRef {
                            name,
                            kind: RefKind::Call,
                            line: node.start_position().row + 1,
                        });
                    }
                }
            }
            "new_expression" => {
                if let Some(constructor) = node.child_by_field_name("constructor") {
                    let name = extract_callable_name(constructor, source);
                    if !name.is_empty() {
                        refs.push(ExtractedRef {
                            name,
                            kind: RefKind::Call,
                            line: node.start_position().row + 1,
                        });
                    }
                }
            }
            "binary_expression" => {
                // Capture `x instanceof Y`
                if let Some(op) = node.child_by_field_name("operator") {
                    if text(op, source) == "instanceof" {
                        if let Some(right) = node.child_by_field_name("right") {
                            let name = text(right, source);
                            if !name.is_empty() {
                                refs.push(ExtractedRef {
                                    name,
                                    kind: RefKind::TypeRef,
                                    line: node.start_position().row + 1,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
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

fn extract_callable_name(func: Node, source: &[u8]) -> String {
    match func.kind() {
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
    }
}

/// Extract import references from an import statement.
/// Returns refs without wrapping in a symbol — caller accumulates into a single `_imports` symbol.
fn extract_import_refs(node: Node, source: &[u8]) -> Vec<ExtractedRef> {
    let mut refs = Vec::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_clause" => {
                extract_import_clause_refs(child, source, &mut refs, node.start_position().row + 1);
            }
            // `import X from "mod"` — default import (identifier directly in import)
            "identifier" => {
                refs.push(ExtractedRef {
                    name: text(child, source),
                    kind: RefKind::Import,
                    line: child.start_position().row + 1,
                });
            }
            _ => {}
        }
    }

    refs
}

fn extract_import_clause_refs(
    clause: Node,
    source: &[u8],
    refs: &mut Vec<ExtractedRef>,
    line: usize,
) {
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                refs.push(ExtractedRef {
                    name: text(child, source),
                    kind: RefKind::Import,
                    line,
                });
            }
            "named_imports" => {
                let mut ic = child.walk();
                for spec in child.children(&mut ic) {
                    if spec.kind() == "import_specifier" {
                        // Use alias if present, otherwise use name
                        let imported = spec
                            .child_by_field_name("name")
                            .or_else(|| spec.named_child(0));
                        if let Some(name_n) = imported {
                            refs.push(ExtractedRef {
                                name: text(name_n, source),
                                kind: RefKind::Import,
                                line: spec.start_position().row + 1,
                            });
                        }
                    }
                }
            }
            "namespace_import" => {
                // `import * as X from "mod"` — the identifier is the alias
                if let Some(id) = child.named_child(0) {
                    refs.push(ExtractedRef {
                        name: text(id, source),
                        kind: RefKind::Import,
                        line,
                    });
                }
            }
            _ => {}
        }
    }
}

/// Handle `declare module "name" { ... }` augmentations
fn extract_ambient_declaration(node: Node, source: &[u8], symbols: &mut Vec<ExtractedSymbol>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "module" {
            let name = child
                .child_by_field_name("name")
                .map(|n| {
                    text(n, source)
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string()
                })
                .unwrap_or_else(|| "unknown".to_string());

            let mut children = Vec::new();
            if let Some(body) = child.child_by_field_name("body") {
                extract_from_node(body, source, &mut children);
            }

            symbols.push(ExtractedSymbol {
                name,
                kind: SymbolKind::Mod,
                signature: None,
                doc_comment: None,
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                children,
                references: Vec::new(),
            });
            return;
        }
        // Also handle `declare function`, `declare class`, etc.
        match child.kind() {
            "function_declaration" | "function_signature" => {
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
            _ => {}
        }
    }
}

/// Well-known test/callback wrapper names at top level.
const CALLBACK_WRAPPERS: &[&str] = &[
    "describe",
    "test",
    "it",
    "fp",
    "beforeEach",
    "afterEach",
    "beforeAll",
    "afterAll",
];

/// Collect refs from well-known callback patterns (describe/test/it/fp/etc.)
/// without creating per-block symbols. Refs are accumulated into `refs` and
/// the caller creates a single `_tests` symbol per file.
fn collect_callback_refs(node: Node, source: &[u8], refs: &mut Vec<ExtractedRef>) {
    // node is an expression_statement — get the call_expression children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "call_expression" {
            collect_callback_refs_from_call(child, source, refs);
        }
    }
}

fn collect_callback_refs_from_call(node: Node, source: &[u8], refs: &mut Vec<ExtractedRef>) {
    let func = match node.child_by_field_name("function") {
        Some(f) => f,
        None => return,
    };

    let func_name = match func.kind() {
        "identifier" => text(func, source),
        "member_expression" => func
            .child_by_field_name("property")
            .map(|n| text(n, source))
            .unwrap_or_default(),
        _ => return,
    };

    if !CALLBACK_WRAPPERS.contains(&func_name.as_str()) {
        return;
    }

    let args = match node.child_by_field_name("arguments") {
        Some(a) => a,
        None => return,
    };

    // Find the callback body and extract refs from it
    let mut ac = args.walk();
    for arg in args.children(&mut ac) {
        match arg.kind() {
            "arrow_function" | "function_expression" | "function" => {
                if let Some(body) = arg.child_by_field_name("body") {
                    extract_calls(body, source, refs);

                    // Recurse into nested describe/test/it blocks
                    let mut bc = body.walk();
                    for stmt in body.children(&mut bc) {
                        if stmt.kind() == "expression_statement" {
                            let mut sc = stmt.walk();
                            for child in stmt.children(&mut sc) {
                                if child.kind() == "call_expression" {
                                    collect_callback_refs_from_call(child, source, refs);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Find the first string literal child of a node (used for `from "module"`)
fn find_string_child(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string" {
            return Some(
                text(child, source)
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            );
        }
    }
    None
}
