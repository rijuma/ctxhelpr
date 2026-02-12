[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## **Indexación semántica de código para Claude Code**

Cada vez que iniciás una nueva sesión de Claude Code, tiene que redescubrir todo tu codebase desde cero. Eso es lento, caro y se pierde información. **ctxhelpr** soluciona eso.

Es un servidor [MCP](https://modelcontextprotocol.io) que pre-indexa tu repositorio semánticamente - funciones, clases, tipos, referencias, cadenas de llamadas - y guarda todo en una base de datos SQLite local. Claude Code puede entonces navegar tu código a través de herramientas específicas en lugar de volcar miles de líneas de código crudo en el contexto.

El resultado: construcción de contexto más rápida, menos tokens gastados, y Claude _entiende_ la estructura de tu código antes de tocarlo.

## Aviso

> [!WARNING]
> Este proyecto es **experimental** y está en desarrollo activo. No ha sido probado exhaustivamente en diversos codebases, y no hay garantía de que el contexto indexado semánticamente sea más efectivo que el contexto que un agente de código construye por su cuenta. Usalo bajo tu propio riesgo.

Si encontrás problemas, tenés sugerencias o querés compartir tu experiencia, por favor [abrí un issue](https://github.com/rijuma/ctxhelpr/issues) o escribime a [marcos@rigoli.dev](mailto:marcos@rigoli.dev).

## Cómo funciona

1. **Indexa tu repo** usando [tree-sitter](https://tree-sitter.github.io/) para extraer símbolos, sus relaciones y documentación
2. **Almacena todo** en una base de datos SQLite por repositorio con búsqueda full-text FTS5
3. **Expone 9 herramientas MCP** que Claude Code usa para navegar tu código semánticamente
4. **Re-indexación incremental** - solo re-parsea archivos que realmente cambiaron (hashing SHA256 del contenido)

### Herramientas MCP

| Herramienta         | Qué hace                                                                  |
| ------------------- | ------------------------------------------------------------------------- |
| `index_repository`  | Indexación completa/re-indexación con verificación incremental de hash    |
| `update_files`      | Re-indexación rápida de archivos específicos después de ediciones (~50ms) |
| `get_overview`      | Estructura general del repo: lenguajes, módulos, tipos principales        |
| `get_file_symbols`  | Todos los símbolos de un archivo con firmas y rangos de líneas            |
| `get_symbol_detail` | Detalle completo: firma, docs, llamadas, invocadores, refs de tipos       |
| `search_symbols`    | Búsqueda full-text en nombres de símbolos y documentación                 |
| `get_references`    | Quién referencia un símbolo dado                                          |
| `get_dependencies`  | De qué depende un símbolo                                                 |
| `index_status`      | Verificar frescura del índice y detectar archivos desactualizados         |

## Soporte de lenguajes

Actualmente implementado:

- **TypeScript / TSX / JavaScript / JSX** - extracción completa

La infraestructura está lista para Python y Rust, pero los extractores todavía no están escritos.

## Primeros pasos

### Requisitos previos

Requiere Rust 1.85+ (edition 2024). Si tenés una versión anterior:

```bash
rustup update stable
```

### Compilar

```bash
cargo build --release
```

### Instalar

```bash
ctxhelpr install [-l | -g]
```

Registra el servidor MCP, instala el archivo de skill y el comando `/index`, ofrece otorgar permisos a las herramientas, y muestra la ruta de la base de datos. Usá `-l` / `--local` para el directorio `.claude/` del proyecto, o `-g` / `--global` para `~/.claude/`. Si no se especifica ninguno, se te preguntará cuál elegir.

### Desinstalar

```bash
ctxhelpr uninstall [-l | -g]
```

Elimina todas las integraciones y revoca permisos de herramientas.

### Permisos

```bash
ctxhelpr perms [-l | -g] [-a | -r]
```

Gestiona qué herramientas de ctxhelpr puede llamar Claude Code sin preguntar. Sin flags, abre un checklist interactivo. `-a` / `--all` otorga todos los permisos; `-r` / `--remove` los revoca. Durante la instalación se te preguntará si querés otorgar todos; usá `ctxhelpr perms` para cambiarlos después.

### CLI

```bash
ctxhelpr                                    # Mostrar ayuda
ctxhelpr serve                              # Servidor MCP (usado internamente por Claude Code)
ctxhelpr install [-l | -g]                  # Instalar integración
ctxhelpr uninstall [-l | -g]                # Eliminar integración
ctxhelpr perms [-l | -g] [-a | -r]          # Gestionar permisos
```

`serve` no está pensado para ejecutarse manualmente. Claude Code lo inicia vía stdio; se detiene automáticamente cuando la sesión termina.

Cuando no se especifica `-l` ni `-g`: `install` te pregunta cuál elegir; los otros comandos auto-detectan buscando primero un `.claude/settings.json` local, y si no existe, usan el global.

## Configuración

Toda la configuración es a través de variables de entorno - no se necesitan archivos de configuración.

| Variable                   | Default                             | Descripción                          |
| -------------------------- | ----------------------------------- | ------------------------------------ |
| `RUST_LOG`                 | -                                   | Nivel de log (ej. `ctxhelpr=debug`)  |
| `CTXHELPR_DB_DIR`          | `~/.cache/ctxhelpr/`                | Ubicación de la base de datos        |
| `CTXHELPR_MAX_FILE_SIZE`   | `1048576` (1MB)                     | Omitir archivos más grandes que esto |
| `CTXHELPR_IGNORE_PATTERNS` | `node_modules,target,.git,dist,...` | Directorios a ignorar                |

## Cómo lo usa Claude

Una vez configurado, el flujo es transparente:

1. Claude detecta que estás trabajando en código
2. Verifica si el repo está indexado (`index_status`)
3. Obtiene una visión general de la estructura (`get_overview`)
4. Profundiza en áreas específicas según sea necesario (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Sigue cadenas de llamadas y dependencias (`get_references`, `get_dependencies`)
6. Después de que editás archivos, mantiene el índice actualizado (`update_files`)

Todo esto pasa automáticamente a través del archivo de skill - no necesitás hacer nada especial.

## Stack tecnológico

- **Rust** (edition 2024) - porque el tiempo de inicio y la memoria importan para una herramienta que corre al lado de tu editor
- **tree-sitter** - parsing rápido y confiable entre lenguajes
- **SQLite + FTS5** - base de datos en un solo archivo con búsqueda full-text, sin dependencias externas
- **rmcp** - SDK oficial de Rust para el Model Context Protocol
- **tokio** - runtime async para el servidor MCP

## Estructura del proyecto

```text
src/
├── main.rs                 # Punto de entrada del CLI
├── cli/                    # Comandos install, uninstall, perms y permissions
├── server/                 # Servidor MCP (transporte stdio)
├── mcp/                    # Definiciones y handlers de herramientas
├── indexer/                # Lógica de indexación + extractores por lenguaje
│   └── languages/          # Extractores basados en tree-sitter
├── storage/                # Persistencia SQLite + esquema
├── output/                 # Formateo JSON eficiente en tokens
└── assets/                 # Templates embebidos de skill y comandos
```
