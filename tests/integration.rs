use std::path::PathBuf;

use ctxhelpr::indexer::Indexer;
use ctxhelpr::storage::{self, SqliteStorage};

fn empty_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typescript")
}

fn python_fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/python")
}

fn rust_fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rust")
}

fn ruby_fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ruby")
}

fn markdown_fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/markdown")
}

fn index_lang_fixtures(path: PathBuf) -> (SqliteStorage, String) {
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let path_str = path.to_str().unwrap().to_string();
    let indexer = Indexer::new();
    let stats = indexer
        .index(&path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");
    assert!(stats.files_total > 0, "No files were processed");
    assert!(stats.symbols_count > 0, "No symbols were extracted");
    (storage, path_str)
}

fn index_fixtures() -> (SqliteStorage, String) {
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let path = fixtures_path();
    let path_str = path.to_str().unwrap().to_string();
    let indexer = Indexer::new();
    let stats = indexer
        .index(&path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");
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

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    assert_eq!(stats.files_total, 9, "Should index 9 TypeScript files");
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
        .index(path_str, &storage, &[], u64::MAX)
        .expect("First index failed");
    assert!(stats1.files_total > 0);

    // Second index - same files, nothing changed
    let stats2 = indexer
        .index(path_str, &storage, &[], u64::MAX)
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
        .search_symbols(&path_str, "Server", 20)
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
    let fmt = ctxhelpr::output::CompactFormatter::new(&ctxhelpr::config::OutputConfig::default());
    let output = ctxhelpr::output::OutputFormatter::format_overview(&fmt, &overview);

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
    let fmt = ctxhelpr::output::CompactFormatter::new(&ctxhelpr::config::OutputConfig::default());
    let output =
        ctxhelpr::output::OutputFormatter::format_file_symbols(&fmt, "simple.ts", &symbols);

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

    // File hasn't changed since indexing — should be skipped
    let stats = Indexer::new()
        .update_files(
            &path_str,
            &["simple.ts".to_string()],
            &storage,
            &[],
            u64::MAX,
        )
        .expect("update_files failed");

    assert_eq!(stats.files_changed, 0, "Unchanged file should be skipped");
}

#[test]
fn test_update_files_new_file() {
    // Fresh storage — file has never been indexed
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let path = fixtures_path();
    let path_str = path.to_str().unwrap().to_string();

    let stats = Indexer::new()
        .update_files(
            &path_str,
            &["simple.ts".to_string()],
            &storage,
            &[],
            u64::MAX,
        )
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
            &[],
            u64::MAX,
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

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing empty dir failed");

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
        .search_symbols(&path_str, "zzz_nonexistent_symbol_xyz", 20)
        .expect("search should not error on no results");

    assert!(
        results.is_empty(),
        "Should return empty for nonexistent term"
    );
}

// ==================== Python Tests ====================

#[test]
fn test_python_index_repository() {
    let path = python_fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    assert_eq!(stats.files_total, 1, "Should index 1 Python file");
    assert!(stats.symbols_count > 0, "Should extract symbols");
}

#[test]
fn test_python_function_signatures() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.py")
        .expect("get_file_symbols failed");

    let add = symbols
        .iter()
        .find(|s| s.name == "add")
        .expect("Should find 'add'");
    assert_eq!(add.kind, "fn");
    let sig = add.signature.as_deref().unwrap_or("");
    assert!(
        sig.contains("int"),
        "add signature should mention 'int', got: {}",
        sig
    );
}

#[test]
fn test_python_class_with_methods() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.py")
        .expect("get_file_symbols failed");

    let animal = symbols
        .iter()
        .find(|s| s.name == "Animal")
        .expect("Should find 'Animal'");
    assert_eq!(animal.kind, "class");

    let children: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(animal.id))
        .collect();
    let method_names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
    assert!(
        method_names.contains(&"speak"),
        "Animal should have 'speak' method, got: {:?}",
        method_names
    );
}

#[test]
fn test_python_inheritance() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.py")
        .expect("get_file_symbols failed");

    let dog = symbols
        .iter()
        .find(|s| s.name == "Dog")
        .expect("Should find 'Dog'");
    assert_eq!(dog.kind, "class");

    let refs = storage
        .get_dependencies(&path_str, dog.id)
        .expect("get_dependencies failed");
    assert!(
        refs.iter().any(|r| r.to_name == "Animal"),
        "Dog should extend Animal, got: {:?}",
        refs.iter().map(|r| &r.to_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_python_docstrings() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.py")
        .expect("get_file_symbols failed");

    let add = symbols
        .iter()
        .find(|s| s.name == "add")
        .expect("Should find 'add'");
    assert!(add.doc_comment.is_some(), "'add' should have a docstring");
    assert!(
        add.doc_comment
            .as_deref()
            .unwrap()
            .contains("Adds two numbers"),
        "Docstring should contain 'Adds two numbers'"
    );
}

#[test]
fn test_python_constants() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.py")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"MAX_RETRIES"),
        "Should find 'MAX_RETRIES' constant, got: {:?}",
        names
    );
}

// ==================== Rust Tests ====================

#[test]
fn test_rust_index_repository() {
    let path = rust_fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    assert_eq!(stats.files_total, 1, "Should index 1 Rust file");
    assert!(stats.symbols_count > 0, "Should extract symbols");
}

#[test]
fn test_rust_functions() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let distance = symbols
        .iter()
        .find(|s| s.name == "distance")
        .expect("Should find 'distance'");
    assert_eq!(distance.kind, "fn");
    assert!(
        distance.doc_comment.is_some(),
        "'distance' should have doc comment"
    );
}

#[test]
fn test_rust_struct_with_fields() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("Should find 'Point'");
    assert_eq!(point.kind, "struct");

    let fields: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(point.id))
        .collect();
    let field_names: Vec<&str> = fields.iter().map(|s| s.name.as_str()).collect();
    assert!(field_names.contains(&"x"), "Point should have 'x' field");
    assert!(field_names.contains(&"y"), "Point should have 'y' field");
}

#[test]
fn test_rust_enum_with_variants() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let shape = symbols
        .iter()
        .find(|s| s.name == "Shape")
        .expect("Should find 'Shape'");
    assert_eq!(shape.kind, "enum");

    let variants: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(shape.id))
        .collect();
    let variant_names: Vec<&str> = variants.iter().map(|s| s.name.as_str()).collect();
    assert!(
        variant_names.contains(&"Circle"),
        "Should have Circle variant"
    );
    assert!(
        variant_names.contains(&"Rectangle"),
        "Should have Rectangle variant"
    );
    assert!(
        variant_names.contains(&"Triangle"),
        "Should have Triangle variant"
    );
}

#[test]
fn test_rust_trait() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let has_area = symbols
        .iter()
        .find(|s| s.name == "HasArea")
        .expect("Should find 'HasArea'");
    assert_eq!(has_area.kind, "trait");

    let methods: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(has_area.id))
        .collect();
    let method_names: Vec<&str> = methods.iter().map(|s| s.name.as_str()).collect();
    assert!(
        method_names.contains(&"area"),
        "HasArea should have 'area' method"
    );
    assert!(
        method_names.contains(&"perimeter"),
        "HasArea should have 'perimeter' method"
    );
}

#[test]
fn test_rust_impl_blocks() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names
            .iter()
            .any(|n| n.contains("HasArea") && n.contains("Shape")),
        "Should find 'HasArea for Shape' impl, got: {:?}",
        names
    );
    assert!(
        names.contains(&"Shape"),
        "Should find inherent 'Shape' impl"
    );
}

#[test]
fn test_rust_doc_comments() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let point = symbols
        .iter()
        .find(|s| s.name == "Point")
        .expect("Should find 'Point'");
    assert!(point.doc_comment.is_some(), "Point should have doc comment");
    assert!(
        point.doc_comment.as_deref().unwrap().contains("2D space"),
        "Doc should mention '2D space'"
    );
}

#[test]
fn test_rust_modules_types_constants() {
    let (storage, path_str) = index_lang_fixtures(rust_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rs")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"utils"), "Should find 'utils' module");
    assert!(names.contains(&"Result"), "Should find 'Result' type alias");
    assert!(
        names.contains(&"MAX_SIZE"),
        "Should find 'MAX_SIZE' constant"
    );
    assert!(
        names.contains(&"GLOBAL_NAME"),
        "Should find 'GLOBAL_NAME' static"
    );
}

// ==================== Ruby Tests ====================

#[test]
fn test_ruby_index_repository() {
    let path = ruby_fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    assert_eq!(stats.files_total, 1, "Should index 1 Ruby file");
    assert!(stats.symbols_count > 0, "Should extract symbols");
}

#[test]
fn test_ruby_class_with_methods() {
    let (storage, path_str) = index_lang_fixtures(ruby_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rb")
        .expect("get_file_symbols failed");

    let animal = symbols
        .iter()
        .find(|s| s.name == "Animal")
        .expect("Should find 'Animal'");
    assert_eq!(animal.kind, "class");

    let children: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(animal.id))
        .collect();
    let method_names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
    assert!(
        method_names.contains(&"speak"),
        "Animal should have 'speak' method, got: {:?}",
        method_names
    );
    assert!(
        method_names.contains(&"initialize"),
        "Animal should have 'initialize' method, got: {:?}",
        method_names
    );
}

#[test]
fn test_ruby_inheritance() {
    let (storage, path_str) = index_lang_fixtures(ruby_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rb")
        .expect("get_file_symbols failed");

    let dog = symbols
        .iter()
        .find(|s| s.name == "Dog")
        .expect("Should find 'Dog'");
    assert_eq!(dog.kind, "class");

    let refs = storage
        .get_dependencies(&path_str, dog.id)
        .expect("get_dependencies failed");
    assert!(
        refs.iter().any(|r| r.to_name == "Animal"),
        "Dog should extend Animal, got: {:?}",
        refs.iter().map(|r| &r.to_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_ruby_module() {
    let (storage, path_str) = index_lang_fixtures(ruby_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rb")
        .expect("get_file_symbols failed");

    let formatter = symbols
        .iter()
        .find(|s| s.name == "Formatter")
        .expect("Should find 'Formatter'");
    assert_eq!(formatter.kind, "mod");

    let children: Vec<_> = symbols
        .iter()
        .filter(|s| s.parent_symbol_id == Some(formatter.id))
        .collect();
    assert!(!children.is_empty(), "Formatter module should have methods");
}

#[test]
fn test_ruby_singleton_method() {
    let (storage, path_str) = index_lang_fixtures(ruby_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rb")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.iter().any(|n| n.contains("breed_info")),
        "Should find 'breed_info' singleton method, got: {:?}",
        names
    );
}

#[test]
fn test_ruby_constants() {
    let (storage, path_str) = index_lang_fixtures(ruby_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.rb")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"MAX_RETRIES"),
        "Should find 'MAX_RETRIES' constant, got: {:?}",
        names
    );
}

// ==================== Code-Aware Search Tests ====================

#[test]
fn test_search_camel_case_subword() {
    let (storage, path_str) = index_fixtures();

    // "getUserById" should be found when searching for "user"
    let results = storage
        .search_symbols(&path_str, "user", 20)
        .expect("search failed");
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names
            .iter()
            .any(|n| n.contains("User") || n.contains("user")),
        "Searching 'user' should find camelCase/PascalCase symbols containing 'user', got: {:?}",
        names
    );
}

#[test]
fn test_search_finds_pascal_case_class() {
    let (storage, path_str) = index_fixtures();

    // Searching for "repository" should find "UserRepository" and "AdminUserRepository"
    let results = storage
        .search_symbols(&path_str, "repository", 20)
        .expect("search failed");
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names.contains(&"UserRepository"),
        "Searching 'repository' should find 'UserRepository', got: {:?}",
        names
    );
}

#[test]
fn test_search_finds_snake_case_parts() {
    let (storage, path_str) = index_lang_fixtures(python_fixtures_path());

    // Searching for "retries" should find "MAX_RETRIES"
    let results = storage
        .search_symbols(&path_str, "retries", 20)
        .expect("search failed");
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names.contains(&"MAX_RETRIES"),
        "Searching 'retries' should find 'MAX_RETRIES', got: {:?}",
        names
    );
}

#[test]
fn test_search_prefix_on_subwords() {
    let (storage, path_str) = index_fixtures();

    // Prefix search "repo*" should find UserRepository via name_tokens
    let results = storage
        .search_symbols(&path_str, "repo*", 20)
        .expect("search failed");
    let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
    assert!(
        names.iter().any(|n| n.contains("Repository")),
        "Prefix search 'repo*' should find Repository symbols, got: {:?}",
        names
    );
}

// ==================== Markdown Tests ====================

#[test]
fn test_markdown_index_repository() {
    let path = markdown_fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    assert_eq!(stats.files_total, 1, "Should index 1 Markdown file");
    assert!(stats.symbols_count > 0, "Should extract sections");
}

#[test]
fn test_markdown_heading_extraction() {
    let (storage, path_str) = index_lang_fixtures(markdown_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.md")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"Project Overview"),
        "Should find 'Project Overview' heading, got: {:?}",
        names
    );
    assert!(
        names.contains(&"Installation"),
        "Should find 'Installation' heading, got: {:?}",
        names
    );
}

#[test]
fn test_markdown_heading_hierarchy() {
    let (storage, path_str) = index_lang_fixtures(markdown_fixtures_path());

    let symbols = storage
        .get_file_symbols(&path_str, "sample.md")
        .expect("get_file_symbols failed");

    // H1 "Project Overview" should be top-level
    let overview = symbols
        .iter()
        .find(|s| s.name == "Project Overview")
        .expect("Should find 'Project Overview'");
    assert_eq!(overview.kind, "section");
    assert!(
        overview.parent_symbol_id.is_none(),
        "H1 should be top-level (no parent)"
    );

    // H2 "Installation" should be a child of H1
    let installation = symbols
        .iter()
        .find(|s| s.name == "Installation")
        .expect("Should find 'Installation'");
    assert_eq!(
        installation.parent_symbol_id,
        Some(overview.id),
        "H2 'Installation' should be child of H1 'Project Overview'"
    );

    // H3 "Prerequisites" should be a child of H2 "Installation"
    let prereqs = symbols
        .iter()
        .find(|s| s.name == "Prerequisites")
        .expect("Should find 'Prerequisites'");
    assert_eq!(
        prereqs.parent_symbol_id,
        Some(installation.id),
        "H3 'Prerequisites' should be child of H2 'Installation'"
    );
}

// ==================== Minified File Skipping Tests ====================

#[test]
fn test_minified_files_skipped_by_default() {
    let path = fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    // vendor.min.js exists in fixtures but should be skipped
    assert_eq!(
        stats.files_total, 9,
        "Should index 9 files (vendor.min.js should be skipped)"
    );
}

#[test]
fn test_custom_ignore_patterns() {
    let path = fixtures_path();
    let path_str = path.to_str().unwrap();
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();

    let ignore = vec!["*.ts".to_string()];
    let stats = indexer
        .index(path_str, &storage, &ignore, u64::MAX)
        .expect("Indexing failed");

    // All .ts files should be ignored, only .min.js remains (which is also skipped by default)
    assert_eq!(
        stats.files_total, 0,
        "All .ts files should be ignored by custom pattern"
    );
}

#[test]
fn test_update_files_skips_ignored() {
    // Fresh storage so files appear new (not skipped by hash check)
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let path = fixtures_path();
    let path_str = path.to_str().unwrap().to_string();

    let stats = Indexer::new()
        .update_files(
            &path_str,
            &["vendor.min.js".to_string(), "simple.ts".to_string()],
            &storage,
            &["*.min.js".to_string()],
            u64::MAX,
        )
        .expect("update_files failed");

    assert_eq!(
        stats.files_changed, 1,
        "Should update only simple.ts, not vendor.min.js"
    );
}

// ==================== New Expression Tests ====================

#[test]
fn test_new_expression_captured() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "new-expression.ts")
        .expect("get_file_symbols failed");

    let create_manager = symbols
        .iter()
        .find(|s| s.name == "createManager")
        .expect("Should find 'createManager'");

    let deps = storage
        .get_dependencies(&path_str, create_manager.id)
        .expect("get_dependencies failed");

    assert!(
        deps.iter().any(|r| r.to_name == "TokenManager"),
        "'createManager' should reference 'new TokenManager()', got: {:?}",
        deps.iter().map(|r| &r.to_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_new_expression_in_method() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "new-expression.ts")
        .expect("get_file_symbols failed");

    // TokenManager.constructor calls `new Map()`
    let constructor = symbols
        .iter()
        .find(|s| s.name == "constructor" && s.parent_symbol_id.is_some())
        .expect("Should find constructor");

    let deps = storage
        .get_dependencies(&path_str, constructor.id)
        .expect("get_dependencies failed");

    assert!(
        deps.iter().any(|r| r.to_name == "Map"),
        "constructor should reference 'new Map()', got: {:?}",
        deps.iter().map(|r| &r.to_name).collect::<Vec<_>>()
    );
}

#[test]
fn test_instanceof_captured() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "new-expression.ts")
        .expect("get_file_symbols failed");

    let handle_error = symbols
        .iter()
        .find(|s| s.name == "handleError")
        .expect("Should find 'handleError'");

    let deps = storage
        .get_dependencies(&path_str, handle_error.id)
        .expect("get_dependencies failed");

    let type_refs: Vec<&str> = deps
        .iter()
        .filter(|r| r.ref_kind == "type_ref")
        .map(|r| r.to_name.as_str())
        .collect();

    assert!(
        type_refs.contains(&"TokenRefreshError"),
        "handleError should have type_ref to TokenRefreshError, got: {:?}",
        type_refs
    );
    assert!(
        type_refs.contains(&"Error"),
        "handleError should have type_ref to Error, got: {:?}",
        type_refs
    );
}

#[test]
fn test_new_expression_resolves_references() {
    let (storage, path_str) = index_fixtures();

    // TokenManager should be referenceable — find it by name
    let symbols = storage
        .get_file_symbols(&path_str, "new-expression.ts")
        .expect("get_file_symbols failed");

    let token_manager = symbols
        .iter()
        .find(|s| s.name == "TokenManager")
        .expect("Should find TokenManager");

    let refs = storage
        .get_references(&path_str, token_manager.id)
        .expect("get_references failed");

    assert!(
        !refs.is_empty(),
        "TokenManager should have references from new expressions"
    );
}

// ==================== Export Default Tests ====================

#[test]
fn test_export_default_expression() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "export-default.ts")
        .expect("get_file_symbols failed");

    assert!(
        !symbols.is_empty(),
        "export-default.ts should have symbols (was producing 0 before)"
    );

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"default"),
        "Should have a 'default' symbol for `export default fp(...)`, got: {:?}",
        names
    );
    assert!(
        names.contains(&"loadConfig"),
        "Should find 'loadConfig' function, got: {:?}",
        names
    );
    assert!(
        names.contains(&"fetchValue"),
        "Should find 'fetchValue' function, got: {:?}",
        names
    );
}

#[test]
fn test_export_default_refs_into_callback() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "export-default.ts")
        .expect("get_file_symbols failed");

    let default_sym = symbols
        .iter()
        .find(|s| s.name == "default")
        .expect("Should find 'default' symbol");

    let deps = storage
        .get_dependencies(&path_str, default_sym.id)
        .expect("get_dependencies failed");

    let dep_names: Vec<&str> = deps.iter().map(|r| r.to_name.as_str()).collect();
    assert!(
        dep_names.contains(&"fp"),
        "default should call 'fp', got: {:?}",
        dep_names
    );
    assert!(
        dep_names.contains(&"loadConfig"),
        "default callback should call 'loadConfig', got: {:?}",
        dep_names
    );
}

// ==================== Barrel Re-export Tests ====================

#[test]
fn test_barrel_named_reexports() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "barrel.ts")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"TokenManager"),
        "Barrel should re-export 'TokenManager', got: {:?}",
        names
    );
    assert!(
        names.contains(&"TokenRefreshError"),
        "Barrel should re-export 'TokenRefreshError', got: {:?}",
        names
    );
    assert!(
        names.contains(&"loadConfig"),
        "Barrel should re-export 'loadConfig', got: {:?}",
        names
    );
}

#[test]
fn test_barrel_aliased_reexport() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "barrel.ts")
        .expect("get_file_symbols failed");

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"currency"),
        "Barrel should re-export 'formatCurrency as currency', got: {:?}",
        names
    );
}

#[test]
fn test_barrel_star_reexport() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "barrel.ts")
        .expect("get_file_symbols failed");

    let star_exports: Vec<&str> = symbols
        .iter()
        .filter(|s| s.name.starts_with("* from"))
        .map(|s| s.name.as_str())
        .collect();

    assert!(
        !star_exports.is_empty(),
        "Barrel should have '* from' symbols, got: {:?}",
        symbols.iter().map(|s| s.name.as_str()).collect::<Vec<_>>()
    );
}

// ==================== Import Reference Tests ====================

#[test]
fn test_import_creates_refs() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "imports.ts")
        .expect("get_file_symbols failed");

    // Should have a single _imports symbol (not per-import synthetic symbols)
    let import_sym = symbols
        .iter()
        .find(|s| s.name == "_imports")
        .expect("imports.ts should have an _imports symbol");

    // No per-import symbols like import(fs), import(path), etc.
    let old_import_syms: Vec<&str> = symbols
        .iter()
        .filter(|s| s.name.starts_with("import("))
        .map(|s| s.name.as_str())
        .collect();
    assert!(
        old_import_syms.is_empty(),
        "Should not have per-import symbols, got: {:?}",
        old_import_syms
    );

    // _imports should have all import refs aggregated
    let deps = storage
        .get_dependencies(&path_str, import_sym.id)
        .expect("get_dependencies failed");

    assert!(
        deps.len() >= 3,
        "_imports should have refs for readFileSync, writeFileSync, path, Config; got {}",
        deps.len()
    );
    assert!(
        deps.iter().all(|r| r.ref_kind == "import"),
        "All refs should have kind 'import'"
    );
}

// ==================== Test File / Callback Tests ====================

#[test]
fn test_describe_blocks_extracted() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "test-file.ts")
        .expect("get_file_symbols failed");

    // Should have a single _tests symbol, not per-describe/test blocks
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"_tests"),
        "test-file.ts should have a _tests symbol, got: {:?}",
        names
    );

    // No per-block symbols like describe(add function), test(adds two positive numbers), etc.
    let old_blocks: Vec<&str> = names
        .iter()
        .filter(|n| n.starts_with("describe(") || n.starts_with("test(") || n.starts_with("it("))
        .copied()
        .collect();
    assert!(
        old_blocks.is_empty(),
        "Should not have per-block symbols, got: {:?}",
        old_blocks
    );
}

#[test]
fn test_nested_describe_refs_collected() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "test-file.ts")
        .expect("get_file_symbols failed");

    let tests_sym = symbols
        .iter()
        .find(|s| s.name == "_tests")
        .expect("Should find _tests symbol");

    // _tests should have refs from both top-level and nested callbacks
    let deps = storage
        .get_dependencies(&path_str, tests_sym.id)
        .expect("get_dependencies failed");

    let dep_names: Vec<&str> = deps.iter().map(|r| r.to_name.as_str()).collect();

    // `add` is called inside describe("add function") tests
    assert!(
        dep_names.contains(&"add"),
        "_tests should have ref to 'add' from nested test bodies, got: {:?}",
        dep_names
    );

    // `expect` is called inside test blocks
    assert!(
        dep_names.contains(&"expect"),
        "_tests should have ref to 'expect', got: {:?}",
        dep_names
    );
}

#[test]
fn test_test_blocks_have_refs() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "test-file.ts")
        .expect("get_file_symbols failed");

    let tests_sym = symbols
        .iter()
        .find(|s| s.name == "_tests")
        .expect("Should find _tests symbol");

    let deps = storage
        .get_dependencies(&path_str, tests_sym.id)
        .expect("get_dependencies failed");

    let dep_names: Vec<&str> = deps.iter().map(|r| r.to_name.as_str()).collect();
    assert!(
        dep_names.contains(&"add"),
        "_tests should have ref to 'add', got: {:?}",
        dep_names
    );
}

// ==================== Declare Module Tests ====================

#[test]
fn test_declare_module_symbols() {
    let (storage, path_str) = index_fixtures();

    let symbols = storage
        .get_file_symbols(&path_str, "declare-module.d.ts")
        .expect("get_file_symbols failed");

    assert!(
        !symbols.is_empty(),
        "declare-module.d.ts should have symbols"
    );

    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"fastify"),
        "Should find 'fastify' module declaration, got: {:?}",
        names
    );
}

// ==================== Dotted Name Resolution Tests ====================

#[test]
fn test_dotted_name_resolves_this_only() {
    let (storage, path_str) = index_fixtures();

    // In complex.ts, methods call `this.cache.get(id)`, `this.db.query(...)`,
    // and `this.findAll()`. Only `this.X` (single dot) patterns should resolve.
    // Multi-dot patterns like `this.cache.get` should NOT resolve via pass 2.
    let symbols = storage
        .get_file_symbols(&path_str, "complex.ts")
        .expect("get_file_symbols failed");

    // AdminUserRepository.findAdmins calls `this.findAll()` — this.findAll is a
    // single-dot this.X pattern, so it should resolve to the findAll method.
    let find_admins = symbols
        .iter()
        .find(|s| s.name == "findAdmins" && s.parent_symbol_id.is_some())
        .expect("Should find findAdmins method");

    let deps = storage
        .get_dependencies(&path_str, find_admins.id)
        .expect("get_dependencies failed");

    let dep_names: Vec<&str> = deps.iter().map(|r| r.to_name.as_str()).collect();

    // this.findAll should appear as a dependency (unresolved name is "this.findAll")
    assert!(
        dep_names.contains(&"this.findAll"),
        "findAdmins should call 'this.findAll', got: {:?}",
        dep_names
    );

    // And it should resolve (to_symbol_id should be set)
    let resolved_deps: Vec<_> = deps
        .iter()
        .filter(|r| r.to_name == "this.findAll" && r.to_symbol_id.is_some())
        .collect();
    assert!(
        !resolved_deps.is_empty(),
        "'this.findAll' should resolve to findAll symbol"
    );
}

// ==================== Auto-Index Helper Tests ====================

#[test]
fn test_has_index_db_nonexistent() {
    assert!(
        !storage::has_index_db("/nonexistent/repo/path/abc123"),
        "has_index_db should return false for non-existent repo"
    );
}

#[test]
fn test_is_repo_indexed_empty_db() {
    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    assert!(
        !storage.is_repo_indexed("/some/path"),
        "is_repo_indexed should return false for empty DB"
    );
}

#[test]
fn test_is_repo_indexed_after_indexing() {
    let (storage, path_str) = index_fixtures();
    assert!(
        storage.is_repo_indexed(&path_str),
        "is_repo_indexed should return true after indexing"
    );
}

#[test]
fn test_is_repo_indexed_wrong_path() {
    let (storage, _path_str) = index_fixtures();
    assert!(
        !storage.is_repo_indexed("/wrong/path"),
        "is_repo_indexed should return false for a different path"
    );
}

#[test]
fn test_gitignore_respected() {
    use std::fs;
    use std::process::Command;

    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let root = tmp.path();

    // Initialize a git repo so the ignore crate recognizes .gitignore
    Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output()
        .expect("git init failed");

    // Create a .gitignore that excludes "generated/"
    fs::write(root.join(".gitignore"), "generated/\n").unwrap();

    // Create a tracked file
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/main.ts"),
        "export function hello(): string { return 'hi'; }\n",
    )
    .unwrap();

    // Create a gitignored file
    fs::create_dir_all(root.join("generated")).unwrap();
    fs::write(
        root.join("generated/output.ts"),
        "export function generated(): void {}\n",
    )
    .unwrap();

    let storage = SqliteStorage::open_memory().expect("Failed to create in-memory DB");
    let indexer = Indexer::new();
    let path_str = root.to_str().unwrap();

    let stats = indexer
        .index(path_str, &storage, &[], u64::MAX)
        .expect("Indexing failed");

    // Should only index the tracked file, not the gitignored one
    assert_eq!(
        stats.files_total, 1,
        "Should only index 1 file (not the gitignored one)"
    );

    // Verify the tracked symbol exists
    let results = storage.search_symbols(path_str, "hello", 20).unwrap();
    assert!(!results.is_empty(), "Should find 'hello' from tracked file");

    // Verify the gitignored symbol does NOT exist
    let results = storage.search_symbols(path_str, "generated", 20).unwrap();
    assert!(
        results.is_empty(),
        "Should NOT find 'generated' from gitignored file"
    );
}
