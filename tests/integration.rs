use std::path::PathBuf;

use ctxhelpr::indexer::Indexer;
use ctxhelpr::storage::SqliteStorage;

fn empty_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript")
}

fn index_fixtures() -> (SqliteStorage, String) {
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let path = fixtures_path();
    let path_str = path.to_str().unwrap().to_string();
    let indexer = Indexer::new();
    let stats = indexer.index(&path_str, &storage).expect("Indexing failed");
    assert!(stats.files_total > 0, "No files were processed");
    assert!(stats.symbols_count > 0, "No symbols were extracted");
    (storage, path_str)
}

#[test]
fn test_index_repository() {
    let path = fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer.index(path_str, &storage).expect("Indexing failed");

    assert_eq!(stats.files_total, 4, "Should index 4 TypeScript files");
    assert!(stats.symbols_count > 10, "Should extract multiple symbols");
    assert!(stats.refs_count > 0, "Should extract references");
    assert_eq!(stats.files_deleted, 0);
}

#[test]
fn test_incremental_reindex() {
    let path = fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    // First index
    let stats1 = indexer
        .index(path_str, &storage)
        .expect("First index failed");
    assert!(stats1.files_total > 0);

    // Second index - same files, nothing changed
    let stats2 = indexer
        .index(path_str, &storage)
        .expect("Second index failed");
    assert_eq!(
        stats2.files_unchanged, stats1.files_total,
        "All files should be unchanged on re-index"
    );
}

#[test]
fn test_get_overview() {
    let (storage, path_str) = index_fixtures();

    let overview = storage
        .get_overview(&path_str)
        .expect("get_overview failed");

    // Should detect TypeScript
    assert!(!overview.languages.is_empty(), "Should detect languages");
    let ts_count: i64 = overview
        .languages
        .iter()
        .filter(|(lang, _)| lang == "typescript")
        .map(|(_, count)| *count)
        .sum();
    assert!(ts_count > 0, "Should detect TypeScript files");

    // Should find top types (Config, Server, Repository, User, etc.)
    assert!(!overview.top_types.is_empty(), "Should find top types");
}

#[test]
fn test_get_file_symbols() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    // simple.ts has: add, multiply, Config, Server, Handler, Status, DEFAULT_PORT, greet
    assert!(
        symbols.len() >= 6,
        "simple.ts should have at least 6 top-level symbols, got {}",
        symbols.len()
    );

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"add"), "Should find 'add' function");
    assert!(names.contains(&"Config"), "Should find 'Config' interface");
    assert!(names.contains(&"Server"), "Should find 'Server' class");
}

#[test]
fn test_function_signature() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let add = symbols
        .iter()
        .find(|s| s.name == "add")
        .expect("Should find 'add'");
    assert_eq!(add.kind, "fn");
    let sig = add.signature.as_deref().unwrap_or("");
    assert!(
        sig.contains("number"),
        "add signature should mention 'number', got: {}",
        sig
    );
}

#[test]
fn test_class_with_methods() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let server = symbols
        .iter()
        .find(|s| s.name == "Server")
        .expect("Should find 'Server'");
    assert_eq!(server.kind, "class");

    // Server should have children (methods)
    let children: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(server.id))
        .collect();
    assert!(
        !children.is_empty(),
        "Server class should have child symbols (methods/fields)"
    );

    let method_names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
    assert!(
        method_names.contains(&"start") || method_names.contains(&"constructor"),
        "Server should have methods, got: {:?}",
        method_names
    );
}

#[test]
fn test_doc_comments() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let add = symbols
        .iter()
        .find(|s| s.name == "add")
        .expect("Should find 'add'");
    assert!(add.doc_comment.is_some(), "'add' should have a doc comment");
    assert!(
        add.doc_comment
            .as_deref()
            .unwrap()
            .contains("Adds two numbers"),
        "Doc comment should contain 'Adds two numbers'"
    );
}

#[test]
fn test_search_symbols() {
    let (storage, path_str) = index_fixtures();

    let results = storage
        .search_symbols(&path_str, "Server")
        .expect("search_symbols failed");

    assert!(!results.is_empty(), "Should find results for 'Server'");
    assert!(
        results.iter().any(|r| r.name == "Server"),
        "Should find Server class"
    );
}

#[test]
fn test_arrow_functions() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "arrow-functions.ts")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"formatCurrency"),
        "Should find 'formatCurrency' arrow function"
    );
}

#[test]
fn test_interface_members() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let config = symbols
        .iter()
        .find(|s| s.name == "Config")
        .expect("Should find 'Config'");
    assert_eq!(config.kind, "interface");

    let fields: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(config.id))
        .collect();
    assert!(
        fields.len() >= 2,
        "Config interface should have at least 2 fields, got {}",
        fields.len()
    );
}

#[test]
fn test_enum_extraction() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let status = symbols
        .iter()
        .find(|s| s.name == "Status")
        .expect("Should find 'Status' enum");
    assert_eq!(status.kind, "enum");
}

#[test]
fn test_call_references() {
    let (storage, path_str) = index_fixtures();

    // The Server.start method calls listen(), so there should be a reference
    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let start = symbols
        .iter()
        .find(|s| s.name == "start")
        .expect("Should find 'start' method");

    // Check dependencies of the start method
    let deps = storage
        .get_dependencies(&path_str, start.id)
        .expect("get_dependencies failed");

    assert!(
        !deps.is_empty(),
        "'start' method should have call references"
    );
    assert!(
        deps.iter().any(|r| r.to_name == "listen"),
        "'start' should reference 'listen', got: {:?}",
        deps.iter().map(|r| &r.to_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_complex_class_hierarchy() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "complex.ts")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"UserRepository"),
        "Should find UserRepository"
    );
    assert!(
        names.contains(&"AdminUserRepository"),
        "Should find AdminUserRepository"
    );
    assert!(names.contains(&"User"), "Should find User interface");
}

#[test]
fn test_compact_output() {
    let (storage, path_str) = index_fixtures();

    let overview = storage
        .get_overview(&path_str)
        .expect("get_overview failed");
    let output = ctxhelpr::output::CompactFormatter::format_overview(&overview);

    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("Overview output should be valid JSON");

    assert!(
        parsed.get("repo").is_some(),
        "Output should have 'repo' key"
    );
    assert!(
        parsed.get("langs").is_some(),
        "Output should have 'langs' key"
    );
    assert!(
        parsed.get("mods").is_some(),
        "Output should have 'mods' key"
    );
}

#[test]
fn test_file_symbols_compact_output() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");
    let output = ctxhelpr::output::CompactFormatter::format_file_symbols("simple.ts", &symbols);

    let parsed: serde_json::Value =
        serde_json::from_str(&output).expect("File symbols output should be valid JSON");

    assert_eq!(parsed["f"], "simple.ts");
    assert!(parsed["syms"].is_array(), "Should have 'syms' array");

    let syms = parsed["syms"].as_array().unwrap();
    assert!(!syms.is_empty(), "Should have symbols");

    // Check compact key format
    let first = &syms[0];
    assert!(
        first.get("n").is_some(),
        "Symbol should have 'n' (name) key"
    );
    assert!(
        first.get("k").is_some(),
        "Symbol should have 'k' (kind) key"
    );
    assert!(first.get("id").is_some(), "Symbol should have 'id' key");
}

#[test]
fn test_update_files() {
    let (storage, path_str) = index_fixtures();

    let stats = Indexer::new()
        .update_files(&path_str, &["simple.ts".to_string()], &storage)
        .expect("update_files failed");

    assert_eq!(stats.files_changed, 1, "Should update 1 file");
    assert!(stats.symbols_count > 0, "Should re-extract symbols");
}

#[test]
fn test_update_files_nonexistent_file() {
    let (storage, path_str) = index_fixtures();

    let stats = Indexer::new()
        .update_files(
            &path_str,
            &["nonexistent_file.ts".to_string()],
            &storage,
        )
        .expect("update_files should handle missing files gracefully");

    assert_eq!(stats.files_changed, 0, "No files should be updated");
    assert_eq!(stats.symbols_count, 0, "No symbols should be extracted");
}

#[test]
fn test_index_empty_directory() {
    let dir = empty_dir();
    let path_str = dir.path().to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer.index(path_str, &storage).expect("Indexing empty dir failed");

    assert_eq!(stats.files_total, 0, "No files in empty dir");
    assert_eq!(stats.symbols_count, 0, "No symbols in empty dir");
    assert_eq!(stats.files_deleted, 0);
}

#[test]
fn test_symbol_detail_with_references() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "simple.ts")
        .expect("get_file_symbols failed");

    let start = symbols
        .iter()
        .find(|s| s.name == "start")
        .expect("Should find 'start' method");

    let detail = storage
        .get_symbol_detail(&path_str, start.id)
        .expect("get_symbol_detail failed");
    assert_eq!(detail.name, "start");
    assert_eq!(detail.kind, "method");

    let deps = storage
        .get_dependencies(&path_str, start.id)
        .expect("get_dependencies failed");
    assert!(!deps.is_empty(), "start should have dependencies");

    let refs = storage
        .get_references(&path_str, start.id)
        .unwrap_or_default();
    // refs may or may not be empty depending on whether other symbols call start
    // but the call should succeed
    let _ = refs;
}

#[test]
fn test_search_no_results() {
    let (storage, path_str) = index_fixtures();

    let results = storage
        .search_symbols(&path_str, "zzz_nonexistent_symbol_xyz")
        .expect("search should not error on no results");

    assert!(results.is_empty(), "Should return empty for nonexistent term");
}
