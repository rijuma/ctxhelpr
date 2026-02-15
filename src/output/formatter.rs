use crate::storage::*;

pub trait OutputFormatter: Send + Sync {
    fn format_index_result(&self, stats: &IndexStats) -> String;
    fn format_overview(&self, data: &OverviewData) -> String;
    fn format_file_symbols(&self, file: &str, symbols: &[SymbolRecord]) -> String;
    fn format_symbol_detail(
        &self,
        sym: &SymbolRecord,
        calls: &[RefRecord],
        called_by: &[RefRecord],
        type_refs: &[RefRecord],
    ) -> String;
    fn format_search_results(&self, query: &str, hits: &[SearchHit]) -> String;
    fn format_references(&self, symbol_id: i64, refs: &[RefRecord]) -> String;
    fn format_dependencies(&self, symbol_id: i64, deps: &[RefRecord]) -> String;
    fn format_index_status(&self, status: &IndexStatus) -> String;
}
