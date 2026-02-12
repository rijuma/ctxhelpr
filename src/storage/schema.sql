PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- ============================================================
-- REPOSITORIES
-- ============================================================
CREATE TABLE IF NOT EXISTS repositories (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    abs_path    TEXT    NOT NULL UNIQUE,
    last_indexed_at TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- FILES
-- ============================================================
CREATE TABLE IF NOT EXISTS files (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_id         INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    rel_path        TEXT    NOT NULL,
    content_hash    TEXT    NOT NULL,
    language        TEXT    NOT NULL,
    last_indexed_at TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(repo_id, rel_path)
);

CREATE INDEX IF NOT EXISTS idx_files_repo     ON files(repo_id);
CREATE INDEX IF NOT EXISTS idx_files_hash     ON files(repo_id, content_hash);
CREATE INDEX IF NOT EXISTS idx_files_language  ON files(repo_id, language);

-- ============================================================
-- SYMBOLS
-- ============================================================
CREATE TABLE IF NOT EXISTS symbols (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id         INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    kind            TEXT    NOT NULL,
    signature       TEXT,
    doc_comment     TEXT,
    start_line      INTEGER NOT NULL,
    end_line        INTEGER NOT NULL,
    parent_symbol_id INTEGER REFERENCES symbols(id) ON DELETE SET NULL,
    file_rel_path   TEXT    NOT NULL,
    repo_id         INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_symbols_file    ON symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_symbols_name    ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_kind    ON symbols(repo_id, kind);
CREATE INDEX IF NOT EXISTS idx_symbols_parent  ON symbols(parent_symbol_id);

-- ============================================================
-- REFERENCES (edges between symbols)
-- ============================================================
CREATE TABLE IF NOT EXISTS refs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    from_symbol_id  INTEGER NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    to_symbol_id    INTEGER REFERENCES symbols(id) ON DELETE SET NULL,
    to_name         TEXT    NOT NULL,
    ref_kind        TEXT    NOT NULL,
    line            INTEGER,
    UNIQUE(from_symbol_id, to_name, ref_kind, line)
);

CREATE INDEX IF NOT EXISTS idx_refs_from    ON refs(from_symbol_id);
CREATE INDEX IF NOT EXISTS idx_refs_to      ON refs(to_symbol_id);
CREATE INDEX IF NOT EXISTS idx_refs_to_name ON refs(to_name);

-- ============================================================
-- FULL-TEXT SEARCH (FTS5)
-- ============================================================
CREATE VIRTUAL TABLE IF NOT EXISTS fts_symbols USING fts5(
    name,
    doc_comment,
    kind,
    file_rel_path,
    content='symbols',
    content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
    INSERT INTO fts_symbols(rowid, name, doc_comment, kind, file_rel_path)
    VALUES (new.id, new.name, new.doc_comment, new.kind, new.file_rel_path);
END;

CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
    INSERT INTO fts_symbols(fts_symbols, rowid, name, doc_comment, kind, file_rel_path)
    VALUES('delete', old.id, old.name, old.doc_comment, old.kind, old.file_rel_path);
END;

CREATE TRIGGER IF NOT EXISTS symbols_au AFTER UPDATE ON symbols BEGIN
    INSERT INTO fts_symbols(fts_symbols, rowid, name, doc_comment, kind, file_rel_path)
    VALUES('delete', old.id, old.name, old.doc_comment, old.kind, old.file_rel_path);
    INSERT INTO fts_symbols(rowid, name, doc_comment, kind, file_rel_path)
    VALUES (new.id, new.name, new.doc_comment, new.kind, new.file_rel_path);
END;
