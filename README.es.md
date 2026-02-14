[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## **Indexación semántica de código para Claude Code**

Cada vez que iniciás una nueva sesión de Claude Code, tiene que redescubrir todo tu codebase desde cero. Eso es lento, caro y se pierde información. **ctxhelpr** trata de mitigar eso.

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

- **TypeScript / TSX / JavaScript / JSX** - funciones, clases, interfaces, enums, arrow functions, referencias de llamadas
- **Python** - funciones, clases, herencia, decoradores, docstrings, constantes
- **Rust** - funciones, structs, enums, traits, bloques impl, módulos, type aliases, constantes
- **Ruby** - clases, módulos, métodos, métodos singleton, herencia, constantes
- **Markdown** - jerarquía de encabezados como secciones con relaciones padre-hijo

## Primeros pasos

### Instalación rápida

```bash
curl -sSf https://sh.ctxhelpr.dev | sh
```

Detecta tu plataforma, descarga la última versión, verifica el checksum e instala en `~/.local/bin/`.

Opciones:

```bash
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --version 1.1.0    # Versión específica
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --install-dir DIR   # Directorio personalizado
curl -sSf https://sh.ctxhelpr.dev | sh -s -- --skip-setup        # Solo el binario
```

### Instalación manual

Descargá la última versión para tu plataforma desde la [página de releases](https://github.com/rijuma/ctxhelpr/releases/latest).

| SO      | Arquitectura  | Archivo                         |
| ------- | ------------- | ------------------------------- |
| Linux   | x86_64        | `ctxhelpr-*-linux-x64.tar.gz`   |
| Linux   | ARM64         | `ctxhelpr-*-linux-arm64.tar.gz` |
| macOS   | Apple Silicon | `ctxhelpr-*-macos-arm64.tar.gz` |
| macOS   | Intel         | `ctxhelpr-*-macos-x64.tar.gz`   |
| Windows | x86_64        | `ctxhelpr-*-windows-x64.zip`    |

**Linux / macOS:**

```bash
tar xzf ctxhelpr-*.tar.gz
mv ctxhelpr ~/.local/bin/
```

**Windows:**

Extraé el archivo `.zip` y colocá `ctxhelpr.exe` en un directorio que esté en tu `PATH`.

### Configurar la integración con Claude Code

```bash
ctxhelpr install [-l | -g]
```

Registra el servidor MCP, instala el archivo de skill y el comando `/index`, ofrece otorgar permisos a las herramientas, y muestra la ruta de la base de datos. Usá `-l` / `--local` para el directorio `.claude/` del proyecto, o `-g` / `--global` para `~/.claude/`. Si no se especifica ninguno, se te preguntará cuál elegir.

### Desinstalar

```bash
ctxhelpr uninstall [-l | -g]
```

Elimina todas las integraciones y revoca permisos de herramientas.

### Gestionar permisos

```bash
ctxhelpr perms [-l | -g] [-a | -r]
```

Gestiona qué herramientas de ctxhelpr puede llamar Claude Code sin preguntar. Sin flags, abre un checklist interactivo. `-a` / `--all` otorga todos los permisos; `-r` / `--remove` los revoca. Durante la instalación se te preguntará si querés otorgar todos; usá `ctxhelpr perms` para cambiarlos después.

### Gestores de paquetes

> [!NOTE]
> La distribución a través de gestores de paquetes (brew, apt, npm/pnpm, etc.) está planificada. Por ahora, descargá el binario pre-compilado desde la página de releases.

## Configuración

### Configuración por proyecto (`.ctxhelpr.json`)

Colocá un archivo `.ctxhelpr.json` en la raíz de tu repositorio para personalizar el comportamiento por proyecto. Todos los campos son opcionales y usan valores predeterminados sensatos.

```json
{
  "output": {
    "max_tokens": 2000,
    "truncate_signatures": 120,
    "truncate_doc_comments": 100
  },
  "search": {
    "max_results": 20
  },
  "indexer": {
    "ignore": ["generated/", "*.min.js"],
    "max_file_size": 1048576
  }
}
```

### CLI de configuración

```bash
ctxhelpr config init                  # Crear un template .ctxhelpr.json en el directorio actual
ctxhelpr config validate [--path dir] # Validar .ctxhelpr.json (sintaxis y esquema)
ctxhelpr config show [--path dir]     # Mostrar configuración resuelta (defaults + overrides)
```

### Referencia de campos

| Campo | Tipo | Default | Descripción |
|-------|------|---------|-------------|
| `output.max_tokens` | number o null | `null` | Limitar tamaño de respuesta (aproximado, 1 token ~ 4 bytes) |
| `output.truncate_signatures` | number | `120` | Largo máximo de firma antes de truncar |
| `output.truncate_doc_comments` | number | `100` | Largo máximo de doc comment en vistas resumidas |
| `search.max_results` | number | `20` | Máximo de resultados de búsqueda |
| `indexer.ignore` | string[] | `[]` | Patrones glob adicionales de rutas a ignorar |
| `indexer.max_file_size` | number | `1048576` | Omitir archivos más grandes que esto (bytes) |

### Variables de entorno

| Variable   | Default | Descripción                         |
| ---------- | ------- | ----------------------------------- |
| `RUST_LOG` | -       | Nivel de log (ej. `ctxhelpr=debug`) |

### Presupuesto de tokens

Las respuestas se pueden limitar con `max_tokens` - ya sea por proyecto en `.ctxhelpr.json` o por solicitud a través del parámetro de la herramienta MCP. Cuando una respuesta excede el presupuesto, los resultados se truncan progresivamente con un marcador `"truncated": true`.

### Búsqueda inteligente de código

La búsqueda entiende las convenciones de nombres de código. Buscar `"user"` encuentra `getUserById`, `UserRepository` y `user_service`. Esto funciona mediante identificadores pre-tokenizados que separan camelCase, PascalCase y snake_case en los límites de palabras.

## Cómo lo usa Claude

Una vez configurado, el flujo es transparente:

1. Claude detecta que estás trabajando en código
2. Verifica si el repo está indexado (`index_status`)
3. Obtiene una visión general de la estructura (`get_overview`)
4. Profundiza en áreas específicas según sea necesario (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Sigue cadenas de llamadas y dependencias (`get_references`, `get_dependencies`)
6. Después de que editás archivos, mantiene el índice actualizado (`update_files`)

Todo esto pasa automáticamente a través del archivo de skill - no necesitás hacer nada especial.

## Referencia del CLI

```bash
ctxhelpr                                    # Mostrar ayuda
ctxhelpr serve                              # Servidor MCP (usado internamente por Claude Code)
ctxhelpr install [-l | -g]                  # Instalar integración
ctxhelpr uninstall [-l | -g]                # Eliminar integración
ctxhelpr perms [-l | -g] [-a | -r]          # Gestionar permisos
ctxhelpr config init                        # Crear template .ctxhelpr.json
ctxhelpr config validate [--path dir]       # Validar archivo de configuración
ctxhelpr config show [--path dir]           # Mostrar configuración resuelta
```

`serve` no está pensado para ejecutarse manualmente. Claude Code lo inicia vía stdio; se detiene automáticamente cuando la sesión termina.

Cuando no se especifica `-l` ni `-g`: `install` te pregunta cuál elegir; los otros comandos auto-detectan buscando primero un `.claude/settings.json` local, y si no existe, usan el global.

## Desarrollo

Para contribuidores que quieran compilar desde el código fuente o trabajar en ctxhelpr.

### Requisitos previos

Requiere Rust 1.85+ (edition 2024). Si tenés una versión anterior:

```bash
rustup update stable
```

### Compilar desde el código fuente

```bash
cargo build --release
```

### Estructura del proyecto

```text
src/
├── main.rs                 # Punto de entrada del CLI
├── config.rs               # Configuración por proyecto (.ctxhelpr.json)
├── cli/                    # Comandos install, uninstall, perms y permissions
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

Para documentación detallada sobre la arquitectura de indexación, ver [docs/indexing-strategy.md](docs/indexing-strategy.md).

### Stack tecnológico

- **Rust** (edition 2024) - porque el tiempo de inicio y la memoria importan para una herramienta que corre al lado de tu editor
- **tree-sitter** - parsing rápido y confiable entre lenguajes
- **SQLite + FTS5** - base de datos en un solo archivo con búsqueda full-text, sin dependencias externas
- **rmcp** - SDK oficial de Rust para el Model Context Protocol
- **tokio** - runtime async para el servidor MCP
