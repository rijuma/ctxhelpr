use std::collections::HashMap;

use serde_json::{Value, json};

use crate::storage::*;

pub struct CompactFormatter;

impl CompactFormatter {
    pub fn format_index_result(stats: &IndexStats) -> String {
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

    pub fn format_update_result(stats: &IndexStats) -> String {
        json!({
            "status": "ok",
            "updated": stats.files_changed,
            "symbols": stats.symbols_count,
            "refs": stats.refs_count,
            "ms": stats.duration_ms,
        })
        .to_string()
    }

    pub fn format_overview(data: &OverviewData) -> String {
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

        let top_types: Vec<Value> = data.top_types.iter().map(Self::symbol_brief).collect();
        let entry_points: Vec<Value> = data.entry_points.iter().map(Self::symbol_brief).collect();

        json!({
            "repo": data.repo_name,
            "langs": langs,
            "mods": mods,
            "top_types": top_types,
            "entry_points": entry_points,
        })
        .to_string()
    }

    pub fn format_file_symbols(file: &str, symbols: &[SymbolRecord]) -> String {
        // Pre-group children by parent_id for O(n) lookup
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
                let mut v = Self::symbol_brief(s);
                if let Some(children) = children_by_parent.get(&s.id) {
                    let child_values: Vec<Value> =
                        children.iter().map(|c| Self::symbol_brief(c)).collect();
                    v.as_object_mut()
                        .unwrap()
                        .insert("children".to_string(), json!(child_values));
                }
                v
            })
            .collect();

        json!({"f": file, "syms": syms}).to_string()
    }

    pub fn format_symbol_detail(
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
                            v["from_f"] = json!(f);
                        }
                        v["kind"] = json!(r.ref_kind);
                        if let Some(l) = r.line {
                            v["line"] = json!(l);
                        }
                        v
                    })
                    .collect::<Vec<_>>()
            );
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

    pub fn format_search_results(query: &str, hits: &[SearchHit]) -> String {
        let results: Vec<Value> = hits
            .iter()
            .map(|h| {
                let mut v = json!({
                    "id": h.id,
                    "n": h.name,
                    "k": h.kind,
                    "f": h.file_rel_path,
                    "l": format!("{}-{}", h.start_line, h.end_line),
                });
                if let Some(sig) = &h.signature {
                    v["sig"] = json!(sig);
                }
                v
            })
            .collect();

        json!({"q": query, "hits": results}).to_string()
    }

    pub fn format_references(symbol_id: i64, refs: &[RefRecord]) -> String {
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
                    v["from_f"] = json!(f);
                }
                if let Some(l) = r.line {
                    v["line"] = json!(l);
                }
                v
            })
            .collect();

        json!({"id": symbol_id, "refs_to": results}).to_string()
    }

    pub fn format_dependencies(symbol_id: i64, deps: &[RefRecord]) -> String {
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

    pub fn format_index_status(status: &IndexStatus) -> String {
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

    fn symbol_brief(s: &SymbolRecord) -> Value {
        let mut v = json!({
            "id": s.id,
            "n": s.name,
            "k": s.kind,
            "f": s.file_rel_path,
            "l": format!("{}-{}", s.start_line, s.end_line),
        });
        if let Some(sig) = &s.signature {
            v["sig"] = json!(sig);
        }
        if let Some(doc) = &s.doc_comment {
            v["doc"] = json!(doc);
        }
        v
    }
}
