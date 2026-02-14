[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./developer-guide.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./developer-guide.es.md)

# Guía de Desarrollo

[Volver al README](../README.es.md)

## Requisitos Previos

Requiere Rust 1.85+ (edition 2024). Si tenés una versión anterior:

```bash
rustup update stable
```

## Compilar desde el Código Fuente

```bash
cargo build --release
```

Usa SQLite integrado vía rusqlite - no se necesitan dependencias externas.

## Ejecución y Testing

### Comandos

```bash
cargo build --release                        # Compilar
cargo test                                   # Ejecutar todos los tests (unit + integración)
cargo test test_name                         # Ejecutar un test individual
cargo test test_name -- --nocapture          # Ejecutar con stdout/stderr visible
RUST_LOG=ctxhelpr=debug cargo run -- serve   # Ejecutar servidor MCP con logging de debug
```

ctxhelpr tiene seis subcomandos: `serve`, `install`, `uninstall`, `perms`, `config`, `repos`.

### Testing

Los tests de integración en `tests/integration.rs` usan `SqliteStorage::open_memory()` e indexan archivos de fixture bajo `tests/fixtures/`. Los tests cubren: indexación, re-indexación incremental, extracción de símbolos (funciones, clases, interfaces, enums, arrow functions), doc comments, referencias de llamadas, búsqueda y formato de salida compacto.

### Formateo y Linting

Después de hacer cambios en el código, siempre ejecutá estas verificaciones y corregí cualquier problema antes de considerar la tarea terminada:

1. `cargo fmt --all -- --check` - corregí con `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings` - corregí todos los warnings

## Arquitectura

### Flujo de datos

```text
Archivos en disco → parsing con tree-sitter → ExtractedSymbol/ExtractedRef → almacenamiento SQLite → salida JSON compacta vía herramientas MCP
```

### Estructura del proyecto

```text
src/
├── main.rs                 # Punto de entrada del CLI
├── config.rs               # Configuración por proyecto (.ctxhelpr.json)
├── cli/                    # Comandos install, uninstall, perms, permissions y repos
├── server/                 # Servidor MCP (transporte stdio)
├── mcp/                    # Definiciones y handlers de herramientas
├── indexer/                # Lógica de indexación + extractores por lenguaje
│   └── languages/          # Extractores basados en tree-sitter (TS, Python, Rust, Ruby, MD)
├── storage/                # Persistencia SQLite + esquema + tokenizador de código
├── output/                 # Formateo JSON eficiente en tokens + presupuesto
│   ├── formatter.rs        # Trait OutputFormatter
│   └── token_budget.rs     # Control de presupuesto de tokens
└── assets/                 # Templates embebidos de skill y comandos
```

### Módulos principales

- **`mcp/`** - `CtxhelprServer` implementa `ServerHandler` vía macros de rmcp (`#[tool_router]`, `#[tool_handler]`, `#[tool]`). Cada herramienta MCP es un método. Todas las herramientas toman una ruta de repo y abren el almacenamiento bajo demanda. Todos los handlers loguean con `tracing::info!` al iniciar con los parámetros relevantes.
- **`indexer/`** - `Indexer` recorre el repo, delega a extractores de lenguaje vía el trait `LanguageExtractor`, maneja la re-indexación incremental vía hashing SHA256 de contenido. Los árboles de `ExtractedSymbol` son recursivos (hijos + referencias).
- **`indexer/languages/`** - Un módulo por lenguaje (TypeScript, Python, Rust, Ruby, Markdown). Cada extractor devuelve `Vec<ExtractedSymbol>` del recorrido del AST de tree-sitter.
- **`storage/`** - `SqliteStorage` envuelve rusqlite. El esquema está en `schema.sql` (cargado vía `include_str!`). La DB es por repo, almacenada en `~/.cache/ctxhelpr/<hash>.db`. La tabla virtual FTS5 con triggers mantiene el índice full-text sincronizado. Provee `begin_transaction()`/`commit()` para batching - el indexer envuelve todas las operaciones en una sola transacción por rendimiento.
- **`output/`** - `CompactFormatter` produce JSON eficiente en tokens con claves cortas (`n`, `k`, `f`, `l`, `sig`, `doc`, `id`).
- **`cli/`** - `install.rs` registra el servidor MCP, instala un archivo de skill y el comando `/index` en `~/.claude/`. `uninstall.rs` elimina el registro, el archivo de skill y el comando.
- **`assets/`** - Templates markdown embebidos para el skill y slash command (incluidos en tiempo de compilación).

`lib.rs` re-exporta `indexer`, `output` y `storage` para uso en tests de integración.

### Stack tecnológico

- **Rust** (edition 2024) - porque el tiempo de inicio y la memoria importan para una herramienta que corre al lado de tu editor
- **tree-sitter** - parsing rápido y confiable entre lenguajes
- **SQLite + FTS5** - base de datos en un solo archivo con búsqueda full-text, sin dependencias externas
- **rmcp** - SDK oficial de Rust para el Model Context Protocol
- **tokio** - runtime async para el servidor MCP

## Agregar un Nuevo Extractor de Lenguaje

1. Crear `src/indexer/languages/<lang>.rs` implementando `LanguageExtractor`
2. Registrarlo en `src/indexer/languages/mod.rs` (agregar al match de `detect_language`)
3. Agregar la instancia del extractor en `Indexer::new()` (`src/indexer/mod.rs`)
4. Agregar fixtures de test bajo `tests/fixtures/<lang>/`

## Principios de Código

- Preferimos soluciones simples, limpias y mantenibles sobre las ingeniosas o complejas.
- La legibilidad y mantenibilidad son preocupaciones primarias.
- Nombres y código auto-documentado. Solo usar comentarios adicionales cuando sea necesario.
- Funciones pequeñas.
- Seguir el principio de responsabilidad única en clases y funciones.
- La cobertura de código es primordial.

## Documentación

Todos los archivos de documentación tienen versiones en inglés (`.md`) y español (`.es.md`). Al actualizar cualquier archivo de documentación, actualizá ambas versiones con los mismos cambios estructurales y de contenido. El inglés es la fuente de verdad.

Estructura de documentación:
- `README.md` / `README.es.md` - Visión general del proyecto e inicio rápido
- `docs/user-guide.md` / `docs/user-guide.es.md` - Configuración, referencia de herramientas, detalles del CLI
- `docs/developer-guide.md` / `docs/developer-guide.es.md` - Compilación, arquitectura, contribución
- `docs/indexing-strategy.md` / `docs/indexing-strategy.es.md` - Profundización en la arquitectura de indexación

## Lectura Adicional

- [Estrategia de Indexación](./indexing-strategy.es.md) - profundización en la arquitectura de indexación
- [Guía de Usuario](./user-guide.es.md) - configuración, referencia de herramientas, detalles del CLI
