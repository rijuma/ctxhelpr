pub mod formatter;
pub mod token_budget;

use std::collections::HashMap;

use serde_json::{Value, json};

pub use formatter::OutputFormatter;
pub use token_budget::TokenBudget;

use crate::config::OutputConfig;
use crate::storage::*;

pub struct CompactFormatter {
    max_sig_len: usize,
    max_doc_brief_len: usize,
}

impl CompactFormatter {
    pub fn new(config: &OutputConfig) -> Self {
        Self {
            max_sig_len: config.truncate_signatures,
            max_doc_brief_len: config.truncate_doc_comments,
        }
    }
}

impl OutputFormatter for CompactFormatter {
    fn format_index_result(&self, stats: &IndexStats) -> String {
        json!({
            "status": "ok",
            "stats": {
                "files": stats.files_total,
                "symbols": stats.symbols_count,
                "refs": stats.refs_count,
                "ms": stats.duration_ms,
            },
            "new": stats.files_new,
            "changed": stats.files_changed,
            "unchanged": stats.files_unchanged,
            "deleted": stats.files_deleted,
        })
        .to_string()
    }

    fn format_update_result(&self, stats: &IndexStats) -> String {
        json!({
            "status": "ok",
            "updated": stats.files_changed,
            "symbols": stats.symbols_count,
            "refs": stats.refs_count,
            "ms": stats.duration_ms,
        })
        .to_string()
    }

    fn format_overview(&self, data: &OverviewData) -> String {
        let langs: Value = data
            .languages
            .iter()
            .map(|(l, c)| (l.clone(), json!(c)))
            .collect::<serde_json::Map<String, Value>>()
            .into();

        let mods: Vec<Value> = data
            .modules
            .iter()
            .map(|m| json!({"p": m.path, "files": m.file_count, "syms": m.symbol_count}))
            .collect();

        let top_types: Vec<Value> = data
            .top_types
            .iter()
            .map(|s| symbol_brief(s, true, self.max_sig_len, self.max_doc_brief_len))
            .collect();
        let entry_points: Vec<Value> = data
            .entry_points
            .iter()
            .map(|s| symbol_brief(s, true, self.max_sig_len, self.max_doc_brief_len))
            .collect();

        json!({
            "repo": data.repo_name,
            "langs": langs,
            "mods": mods,
            "top_types": top_types,
            "entry_points": entry_points,
        })
        .to_string()
    }

    fn format_file_symbols(&self, file: &str, symbols: &[SymbolRecord]) -> String {
        let mut children_by_parent: HashMap<i64, Vec<&SymbolRecord>> = HashMap::new();
        for s in symbols {
            if let Some(pid) = s.parent_symbol_id {
                children_by_parent.entry(pid).or_default().push(s);
            }
        }

        let syms: Vec<Value> = symbols
            .iter()
            .filter(|s| s.parent_symbol_id.is_none())
            .map(|s| {
                let mut v = symbol_brief(s, false, self.max_sig_len, self.max_doc_brief_len);
                if let Some(children) = children_by_parent.get(&s.id) {
                    let child_values: Vec<Value> = children
                        .iter()
                        .map(|c| symbol_brief(c, false, self.max_sig_len, self.max_doc_brief_len))
                        .collect();
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("children".to_string(), json!(child_values));
                    }
                }
                v
            })
            .collect();

        json!({"f": file, "syms": syms}).to_string()
    }

    fn format_symbol_detail(
        &self,
        sym: &SymbolRecord,
        calls: &[RefRecord],
        called_by: &[RefRecord],
        type_refs: &[RefRecord],
    ) -> String {
        let mut obj = json!({
            "id": sym.id,
            "n": sym.name,
            "k": sym.kind,
            "f": sym.file_rel_path,
            "l": format!("{}-{}", sym.start_line, sym.end_line),
        });

        if let Some(sig) = &sym.signature {
            obj["sig"] = json!(sig);
        }
        if let Some(doc) = &sym.doc_comment {
            obj["doc"] = json!(doc);
        }

        if !calls.is_empty() {
            obj["calls"] = json!(
                calls
                    .iter()
                    .map(|r| {
                        let mut v = json!({"n": r.to_name});
                        if let Some(id) = r.to_symbol_id {
                            v["id"] = json!(id);
                        }
                        v
                    })
                    .collect::<Vec<_>>()
            );
        }

        if !called_by.is_empty() {
            let mut path_index = PathIndex::new();
            obj["called_by"] = json!(
                called_by
                    .iter()
                    .map(|r| {
                        let mut v = json!({});
                        v["from_id"] = json!(r.from_symbol_id);
                        if let Some(n) = &r.from_name {
                            v["from_n"] = json!(n);
                        }
                        if let Some(f) = &r.from_file {
                            v["fi"] = json!(path_index.index(f));
                        }
                        v["kind"] = json!(r.ref_kind);
                        if let Some(l) = r.line {
                            v["line"] = json!(l);
                        }
                        v
                    })
                    .collect::<Vec<_>>()
            );
            if path_index.len() > 1 {
                obj["_f"] = json!(path_index.into_list());
            } else if let Some(only) = path_index.into_list().into_iter().next() {
                if let Some(called_by_arr) = obj["called_by"].as_array_mut() {
                    for item in called_by_arr {
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj.remove("fi");
                            item_obj.insert("from_f".to_string(), json!(only));
                        }
                    }
                }
            }
        }

        if !type_refs.is_empty() {
            obj["type_refs"] = json!(
                type_refs
                    .iter()
                    .map(|r| {
                        let mut v = json!({"n": r.to_name});
                        if let Some(id) = r.to_symbol_id {
                            v["id"] = json!(id);
                        } else {
                            v["external"] = json!(true);
                        }
                        v
                    })
                    .collect::<Vec<_>>()
            );
        }

        obj.to_string()
    }

    fn format_search_results(&self, query: &str, hits: &[SearchHit]) -> String {
        let mut path_index = PathIndex::new();

        let results: Vec<Value> = hits
            .iter()
            .map(|h| {
                let mut v = json!({
                    "id": h.id,
                    "n": h.name,
                    "k": h.kind,
                    "fi": path_index.index(&h.file_rel_path),
                    "l": format!("{}-{}", h.start_line, h.end_line),
                });
                if let Some(sig) = &h.signature {
                    v["sig"] = json!(normalize_signature(sig, self.max_sig_len));
                }
                v
            })
            .collect();

        let mut obj = json!({"q": query, "hits": results});
        if path_index.len() > 1 {
            obj["_f"] = json!(path_index.into_list());
        } else if let Some(only) = path_index.into_list().into_iter().next() {
            if let Some(arr) = obj["hits"].as_array_mut() {
                for item in arr {
                    if let Some(item_obj) = item.as_object_mut() {
                        item_obj.remove("fi");
                        item_obj.insert("f".to_string(), json!(only));
                    }
                }
            }
        }

        obj.to_string()
    }

    fn format_references(&self, symbol_id: i64, refs: &[RefRecord]) -> String {
        let mut path_index = PathIndex::new();

        let results: Vec<Value> = refs
            .iter()
            .map(|r| {
                let mut v = json!({
                    "from_id": r.from_symbol_id,
                    "kind": r.ref_kind,
                });
                if let Some(n) = &r.from_name {
                    v["from_n"] = json!(n);
                }
                if let Some(f) = &r.from_file {
                    v["fi"] = json!(path_index.index(f));
                }
                if let Some(l) = r.line {
                    v["line"] = json!(l);
                }
                v
            })
            .collect();

        let mut obj = json!({"id": symbol_id, "refs_to": results});
        if path_index.len() > 1 {
            obj["_f"] = json!(path_index.into_list());
        } else if let Some(only) = path_index.into_list().into_iter().next() {
            if let Some(arr) = obj["refs_to"].as_array_mut() {
                for item in arr {
                    if let Some(item_obj) = item.as_object_mut() {
                        item_obj.remove("fi");
                        item_obj.insert("from_f".to_string(), json!(only));
                    }
                }
            }
        }

        obj.to_string()
    }

    fn format_dependencies(&self, symbol_id: i64, deps: &[RefRecord]) -> String {
        let results: Vec<Value> = deps
            .iter()
            .map(|r| {
                let mut v = json!({"to_n": r.to_name, "kind": r.ref_kind});
                if let Some(id) = r.to_symbol_id {
                    v["to_id"] = json!(id);
                } else {
                    v["external"] = json!(true);
                }
                v
            })
            .collect();

        json!({"id": symbol_id, "deps": results}).to_string()
    }

    fn format_index_status(&self, status: &IndexStatus) -> String {
        let mut obj = json!({
            "repo": status.repo_path,
            "files": status.total_files,
            "symbols": status.total_symbols,
            "refs": status.total_refs,
        });

        if let Some(at) = &status.indexed_at {
            obj["indexed_at"] = json!(at);
        }

        if !status.stale_files.is_empty() {
            obj["stale"] = json!(status.stale_files.len());
            obj["stale_files"] = json!(status.stale_files);
        }

        if !status.deleted_files.is_empty() {
            obj["deleted"] = json!(status.deleted_files.len());
            obj["deleted_files"] = json!(status.deleted_files);
        }

        let langs: Value = status
            .languages
            .iter()
            .map(|(l, c)| (l.clone(), json!(c)))
            .collect::<serde_json::Map<String, Value>>()
            .into();
        obj["langs"] = langs;

        obj.to_string()
    }
}

// ── Shared helpers ──

fn symbol_brief(
    s: &SymbolRecord,
    include_file: bool,
    max_sig_len: usize,
    max_doc_brief_len: usize,
) -> Value {
    let mut v = json!({
        "id": s.id,
        "n": s.name,
        "k": s.kind,
        "l": format!("{}-{}", s.start_line, s.end_line),
    });
    if include_file {
        v["f"] = json!(s.file_rel_path);
    }
    if let Some(sig) = &s.signature {
        v["sig"] = json!(normalize_signature(sig, max_sig_len));
    }
    if let Some(doc) = &s.doc_comment {
        v["doc"] = json!(truncate_doc(doc, max_doc_brief_len));
    }
    v
}

// ── Path deduplication ──

struct PathIndex {
    paths: Vec<String>,
    index_map: HashMap<String, usize>,
}

impl PathIndex {
    fn new() -> Self {
        Self {
            paths: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    fn index(&mut self, path: &str) -> usize {
        if let Some(&idx) = self.index_map.get(path) {
            return idx;
        }
        let idx = self.paths.len();
        self.paths.push(path.to_string());
        self.index_map.insert(path.to_string(), idx);
        idx
    }

    fn len(&self) -> usize {
        self.paths.len()
    }

    fn into_list(self) -> Vec<String> {
        self.paths
    }
}

// ── Signature normalization ──

/// Find the largest byte index <= max_bytes that is a valid char boundary.
fn floor_char_boundary(s: &str, max_bytes: usize) -> usize {
    if max_bytes >= s.len() {
        return s.len();
    }
    let mut i = max_bytes;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn normalize_signature(sig: &str, max_sig_len: usize) -> String {
    let mut result = String::with_capacity(sig.len());
    let mut pending_space = false;

    for c in sig.chars() {
        if c.is_whitespace() {
            if !result.is_empty() {
                pending_space = true;
            }
            continue;
        }

        if pending_space {
            pending_space = false;
            if let Some(last) = result.chars().last() {
                let drop_after = matches!(last, '(' | '[' | '{' | '<' | ':' | ',');
                let drop_before = matches!(c, ')' | ']' | '}' | '>' | ':' | ',');
                if !drop_after && !drop_before {
                    result.push(' ');
                }
            }
        }
        result.push(c);
    }

    if result.len() > max_sig_len {
        let boundary = floor_char_boundary(&result, max_sig_len);
        let truncated = &result[..boundary];
        if let Some(pos) = truncated.rfind([',', ')', '>']) {
            return format!("{}...", &truncated[..=pos]);
        }
        return format!("{truncated}...");
    }

    result
}

// ── Doc comment truncation ──

fn truncate_doc(doc: &str, max_doc_brief_len: usize) -> String {
    let trimmed = doc.trim();

    if let Some(dot_pos) = trimmed.find(". ") {
        let first_sentence = &trimmed[..=dot_pos];
        if first_sentence.len() <= max_doc_brief_len {
            return first_sentence.to_string();
        }
    }

    if let Some(nl_pos) = trimmed.find('\n') {
        let first_line = trimmed[..nl_pos].trim();
        if first_line.len() <= max_doc_brief_len {
            return first_line.to_string();
        }
    }

    if trimmed.len() <= max_doc_brief_len {
        return trimmed.to_string();
    }

    let boundary = floor_char_boundary(trimmed, max_doc_brief_len);
    let truncated = &trimmed[..boundary];
    if let Some(space_pos) = truncated.rfind(' ') {
        return format!("{}...", &truncated[..space_pos]);
    }

    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MAX_SIG: usize = 120;
    const TEST_MAX_DOC: usize = 100;

    #[test]
    fn test_normalize_signature_strips_whitespace() {
        let sig = "(a: number, b: number): number";
        let result = normalize_signature(sig, TEST_MAX_SIG);
        assert_eq!(result, "(a:number,b:number):number");
    }

    #[test]
    fn test_normalize_signature_preserves_ident_spaces() {
        let sig = "fn add(a int, b int) int";
        let result = normalize_signature(sig, TEST_MAX_SIG);
        assert_eq!(result, "fn add(a int,b int) int");
    }

    #[test]
    fn test_normalize_signature_truncates() {
        let sig = "a".repeat(200);
        let result = normalize_signature(&sig, TEST_MAX_SIG);
        assert!(result.len() <= TEST_MAX_SIG + 3);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_normalize_signature_utf8_boundary() {
        // Emoji is 4 bytes — verify truncation doesn't panic
        let sig = "fn f(".to_string() + &"\u{1F600}".repeat(50) + ")";
        let result = normalize_signature(&sig, 20);
        assert!(result.ends_with("..."));
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn test_truncate_doc_first_sentence() {
        let doc = "Adds two numbers. Returns the sum.";
        assert_eq!(truncate_doc(doc, TEST_MAX_DOC), "Adds two numbers.");
    }

    #[test]
    fn test_truncate_doc_short() {
        let doc = "Simple doc";
        assert_eq!(truncate_doc(doc, TEST_MAX_DOC), "Simple doc");
    }

    #[test]
    fn test_truncate_doc_long() {
        let doc = "a ".repeat(100);
        let result = truncate_doc(&doc, TEST_MAX_DOC);
        assert!(result.len() <= TEST_MAX_DOC + 3);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_doc_utf8_boundary() {
        // CJK chars are 3 bytes each
        let doc = "\u{4e16}\u{754c}".repeat(50); // "世界" repeated
        let result = truncate_doc(&doc, 20);
        assert!(result.ends_with("..."));
        // Verify the result is valid UTF-8 (won't panic)
        let _ = result.chars().count();
    }

    #[test]
    fn test_floor_char_boundary() {
        let s = "hello\u{1F600}world"; // emoji at byte 5, 4 bytes
        assert_eq!(floor_char_boundary(s, 5), 5);
        assert_eq!(floor_char_boundary(s, 6), 5); // inside emoji
        assert_eq!(floor_char_boundary(s, 7), 5); // inside emoji
        assert_eq!(floor_char_boundary(s, 8), 5); // inside emoji
        assert_eq!(floor_char_boundary(s, 9), 9); // after emoji
        assert_eq!(floor_char_boundary(s, 100), s.len());
    }

    #[test]
    fn test_path_index_deduplication() {
        let mut pi = PathIndex::new();
        assert_eq!(pi.index("src/a.rs"), 0);
        assert_eq!(pi.index("src/b.rs"), 1);
        assert_eq!(pi.index("src/a.rs"), 0);
        assert_eq!(pi.len(), 2);
    }
}
