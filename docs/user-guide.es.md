[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./user-guide.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./user-guide.es.md)

# Guía de Usuario

[Volver al README](../README.es.md)

## Instalación

### Instalación rápida

```text
curl -sSfL https://sh.ctxhelpr.dev | sh
```

Detecta tu plataforma, descarga la última versión, verifica el checksum e instala en `~/.local/bin/`.

Opciones:

```text
curl -sSfL https://sh.ctxhelpr.dev | sh -s -- --version 1.1.0    # Versión específica
curl -sSfL https://sh.ctxhelpr.dev | sh -s -- --install-dir DIR   # Directorio personalizado
curl -sSfL https://sh.ctxhelpr.dev | sh -s -- --skip-setup        # Solo descargar, sin configuración
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

```text
tar xzf ctxhelpr-*.tar.gz
mv ctxhelpr ~/.local/bin/
```

**Windows:**

Extraé el archivo `.zip` y colocá `ctxhelpr.exe` en un directorio que esté en tu `PATH`.

### Gestores de paquetes

> [!NOTE]
> La distribución a través de gestores de paquetes (brew, apt, npm/pnpm, etc.) está planificada. Por ahora, descargá ctxhelpr desde la página de releases.

## Configuración inicial

### Integración con Claude Code

```text
ctxhelpr enable [-l | -g]
```

Registra el servidor MCP, instala el archivo de skill y el comando `/index`, ofrece otorgar permisos a las herramientas, y muestra la ruta de la base de datos. Usá `-l` / `--local` para el directorio `.claude/` del proyecto, o `-g` / `--global` para `~/.claude/`. Si no se especifica ninguno, se te preguntará cuál elegir.

### Gestión de permisos

```text
ctxhelpr perms [-l | -g] [-a | -r]
```

Gestiona qué herramientas de ctxhelpr puede llamar Claude Code sin preguntar. Sin flags, abre un checklist interactivo. `-a` / `--all` otorga todos los permisos; `-r` / `--remove` los revoca. Durante la configuración se te preguntará si querés otorgar todos; usá `ctxhelpr perms` para cambiarlos después.

### Deshabilitar

```text
ctxhelpr disable [-l | -g]
```

Pide confirmación antes de proceder. Elimina todas las integraciones y revoca permisos de herramientas. Ofrece eliminar las bases de datos de índice: deshabilitar localmente ofrece eliminar la DB del repo actual (por defecto: sí), globalmente ofrece eliminar todas las DBs (por defecto: sí). Si existe un `.ctxhelpr.json` en el directorio actual, ofrece eliminarlo (por defecto: no).

### Actualizar

```text
ctxhelpr update
```

Busca una versión más reciente en GitHub, descarga y verifica el release, y reemplaza el binario actual. También refresca los archivos de skill y comando si existen. Sugiere re-indexar los repositorios después de actualizar.

### Desinstalar

```text
ctxhelpr uninstall
```

Elimina completamente ctxhelpr de tu sistema. Pide confirmación, luego deshabilita todas las integraciones (global y local), y elimina el binario.

## Referencia de Herramientas MCP

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
| `list_repos`        | Listar todos los repositorios indexados con estadísticas                  |
| `delete_repos`      | Eliminar datos de índice de los repositorios especificados                |

## Soporte de Lenguajes

- **TypeScript / TSX / JavaScript / JSX** - funciones, clases, interfaces, enums, arrow functions, referencias de llamadas
- **Python** - funciones, clases, herencia, decoradores, docstrings, constantes
- **Rust** - funciones, structs, enums, traits, bloques impl, módulos, type aliases, constantes
- **Ruby** - clases, módulos, métodos, métodos singleton, herencia, constantes
- **Markdown** - jerarquía de encabezados como secciones con relaciones padre-hijo

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

```text
ctxhelpr config init                  # Crear un template .ctxhelpr.json en el directorio actual
ctxhelpr config validate [--path dir] # Validar .ctxhelpr.json (sintaxis y esquema)
ctxhelpr config show [--path dir]     # Mostrar configuración resuelta (defaults + overrides)
```

### Referencia de campos

| Campo                          | Tipo          | Default   | Descripción                                                 |
| ------------------------------ | ------------- | --------- | ----------------------------------------------------------- |
| `output.max_tokens`            | number o null | `null`    | Limitar tamaño de respuesta (aproximado, 1 token ~ 4 bytes) |
| `output.truncate_signatures`   | number        | `120`     | Largo máximo de firma antes de truncar                      |
| `output.truncate_doc_comments` | number        | `100`     | Largo máximo de doc comment en vistas resumidas             |
| `search.max_results`           | number        | `20`      | Máximo de resultados de búsqueda                            |
| `indexer.ignore`               | string[]      | `[]`      | Patrones glob adicionales de rutas a ignorar                |
| `indexer.max_file_size`        | number        | `1048576` | Omitir archivos más grandes que esto (bytes)                |

### Variables de entorno

| Variable   | Default | Descripción                         |
| ---------- | ------- | ----------------------------------- |
| `RUST_LOG` | -       | Nivel de log (ej. `ctxhelpr=debug`) |

## Presupuesto de Tokens

Las respuestas se pueden limitar con `max_tokens` - ya sea por proyecto en `.ctxhelpr.json` o por solicitud a través del parámetro de la herramienta MCP. Cuando una respuesta excede el presupuesto, los resultados se truncan progresivamente con un marcador `"truncated": true`.

## Búsqueda Inteligente de Código

La búsqueda entiende las convenciones de nombres de código. Buscar `"user"` encuentra `getUserById`, `UserRepository` y `user_service`. Esto funciona mediante identificadores pre-tokenizados que separan camelCase, PascalCase y snake_case en los límites de palabras.

## Cómo lo Usa Claude

Una vez configurado, el flujo es transparente:

1. Claude detecta que estás trabajando en código
2. Verifica si el repo está indexado (`index_status`)
3. Obtiene una visión general de la estructura (`get_overview`)
4. Profundiza en áreas específicas según sea necesario (`get_file_symbols`, `search_symbols`, `get_symbol_detail`)
5. Sigue cadenas de llamadas y dependencias (`get_references`, `get_dependencies`)
6. Después de que editás archivos, mantiene el índice actualizado (`update_files`)

Todo esto pasa automáticamente vía el archivo de skill — no se necesita configuración adicional.

## Referencia del CLI

```text
ctxhelpr                                    # Mostrar ayuda
ctxhelpr serve                              # Servidor MCP (usado internamente por Claude Code)
ctxhelpr enable [-l | -g]                   # Habilitar integración
ctxhelpr disable [-l | -g]                  # Deshabilitar integración
ctxhelpr perms [-l | -g] [-a | -r]          # Gestionar permisos
ctxhelpr config init                        # Crear template .ctxhelpr.json
ctxhelpr config validate [--path dir]       # Validar archivo de configuración
ctxhelpr config show [--path dir]           # Mostrar configuración resuelta
ctxhelpr repos list                         # Listar todos los repositorios indexados
ctxhelpr repos delete [paths...]            # Eliminar datos de índice (interactivo si no se dan paths)
ctxhelpr update                             # Actualizar a la última versión
ctxhelpr uninstall                          # Eliminar completamente ctxhelpr
```

`serve` no está pensado para ejecutarse manualmente. Claude Code lo inicia vía stdio; se detiene automáticamente cuando la sesión termina.

Cuando no se especifica `-l` ni `-g`: `enable` te pregunta cuál elegir; los otros comandos auto-detectan buscando primero un `.claude/settings.json` local, y si no existe, usan el global.
