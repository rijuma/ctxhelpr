[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./indexing-strategy.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./indexing-strategy.es.md)

# Estrategia de Indexación

Este documento explica cómo ctxhelpr indexa codebases, las decisiones de diseño detrás del enfoque y los tradeoffs conocidos.

## Visión General

ctxhelpr usa **tree-sitter** para parsear archivos fuente en árboles de sintaxis concretos (CSTs), luego extrae símbolos estructurales (funciones, clases, interfaces, etc.) y sus relaciones (llamadas, imports, referencias de tipos). Estos se almacenan en **SQLite** con búsqueda full-text **FTS5** y se sirven a agentes de IA vía herramientas MCP.

La idea clave: los agentes de IA no necesitan leer archivos fuente crudos para navegar código. Necesitan resúmenes estructurados y eficientes en tokens en los que pueden profundizar bajo demanda.

## Flujo de Datos

```text
Archivos en disco
    |
    v
Parsing con tree-sitter (gramáticas por lenguaje)
    |
    v
ExtractedSymbol / ExtractedRef (árboles recursivos)
    |
    v
Almacenamiento SQLite (símbolos, refs, índice FTS5)
    |
    v
Salida JSON compacta vía herramientas MCP
```

## Indexación Incremental

### Cómo Funciona

1. **Hashing de contenido SHA-256** - El contenido de cada archivo se hashea al momento de indexar
2. **Comparación de hashes** - Al re-indexar, los hashes existentes se comparan con el contenido actual
3. **Re-parsing selectivo** - Solo los archivos nuevos o modificados se re-parsean
4. **Detección de archivos eliminados** - Los archivos presentes en la DB pero ausentes del disco se eliminan
5. **Transacción única** - Todas las operaciones se envuelven en una sola transacción SQLite para atomicidad

### Características de Rendimiento

- **Primera indexación**: O(n) donde n = total de archivos. El parsing de tree-sitter es rápido (~1ms por archivo para la mayoría)
- **Re-indexación (sin cambios)**: O(n) para el recorrido de directorios + O(m) para búsquedas de hash, donde m = archivos indexados. No se realiza parsing.
- **Actualización parcial** (`update_files`): O(k) donde k = número de archivos especificados. Evita el recorrido de directorios por completo.
- **Batching de transacciones**: Todas las inserciones ocurren dentro de un solo `BEGIN IMMEDIATE`...`COMMIT`, evitando overhead de transacción por fila

### Selección de Archivos

Los archivos se seleccionan basándose en:

- **Mapeo de extensiones**: Cada extractor de lenguaje declara qué extensiones maneja (ej., `.ts`, `.tsx`, `.js`, `.jsx` para TypeScript)
- **Límite de tamaño**: Los archivos más grandes que 1 MiB (configurable vía `.ctxhelpr.json`) se omiten
- **Soporte de gitignore**: Los archivos `.gitignore` se respetan automáticamente (incluyendo gitignore anidados y globales). Los archivos ignorados por git se omiten durante la indexación.
- **Patrones de ignorar por defecto**: Como red de seguridad (para repos sin `.gitignore`), los directorios estándar también se excluyen: `node_modules`, `target`, `.git`, `dist`, `build`, `__pycache__`, `.venv`, `vendor`, `.next`, `.nuxt`, `coverage`, `.cache`
- **Patrones de configuración de usuario**: Se pueden configurar patrones de ignorar adicionales vía `.ctxhelpr.json` `indexer.ignore` — estos se aplican sobre `.gitignore` y la lista por defecto

## Extracción de Símbolos

### Extractores de Lenguaje

Cada lenguaje tiene un extractor dedicado que implementa el trait `LanguageExtractor`:

| Lenguaje   | Extractor           | Extensiones                      |
| ---------- | ------------------- | -------------------------------- |
| TypeScript | TypeScriptExtractor | .ts, .tsx, .js, .jsx, .mjs, .cjs |
| Python     | PythonExtractor     | .py, .pyi                        |
| Rust       | RustExtractor       | .rs                              |
| Ruby       | RubyExtractor       | .rb                              |
| Markdown   | MarkdownExtractor   | .md, .markdown                   |

### Tipos de Símbolos

- `fn` - Funciones y declaraciones de funciones independientes
- `method` - Métodos dentro de clases/impls
- `class` - Declaraciones de clase
- `interface` - Declaraciones de interfaz (TypeScript)
- `struct` - Declaraciones de struct (Rust)
- `enum` - Declaraciones de enum
- `trait` - Declaraciones de trait (Rust)
- `mod` - Declaraciones de módulo (Rust, Ruby)
- `const` - Constantes
- `var` - Variables y asignaciones
- `impl` - Bloques de implementación (Rust)
- `section` - Secciones de documento (encabezados Markdown)
- `type` - Alias de tipos

### Tipos de Referencia

- `call` - Llamadas a funciones/métodos
- `import` - Sentencias de import
- `type_ref` - Referencias de tipos en firmas
- `extends` - Herencia de clases/interfaces
- `implements` - Implementación de interfaces

### Estructura de Árbol Recursivo

Los símbolos se extraen como árboles recursivos: una clase contiene métodos, una interfaz contiene campos, un enum contiene variantes. El struct `ExtractedSymbol` tiene campos `children` y `references`. El almacenamiento los aplana en filas con claves foráneas `parent_symbol_id`.

## Búsqueda Full-Text (FTS5)

### Columnas Indexadas

La tabla virtual FTS5 indexa cinco columnas:

1. `name` - Nombre del símbolo tal cual
2. `doc_comment` - Strings de documentación
3. `kind` - Tipo de símbolo (fn, class, etc.)
4. `file_rel_path` - Ruta relativa del archivo
5. `name_tokens` - Sub-palabras pre-divididas del identificador

### Tokenización Inteligente de Código

El tokenizador por defecto de FTS5 `unicode61` trata `getUserById` como un solo token, haciendo imposible buscar "user" y encontrarlo. ctxhelpr resuelve esto con un enfoque de **pre-tokenización**:

Al momento de insertar, cada nombre de símbolo se divide en sub-palabras:

- `getUserById` -> `"get user by id getuserbyid"`
- `UserRepository` -> `"user repository userrepository"`
- `MAX_RETRIES` -> `"max retries max_retries"`
- `HTMLParser` -> `"html parser htmlparser"`

Estos tokens se almacenan en la columna `name_tokens` y son indexados por FTS5. El nombre original en minúsculas se agrega para que las búsquedas exactas sigan funcionando.

**Reglas de división:**

- Límites de camelCase: `getUser` -> `get`, `user`
- Límites de PascalCase: `UserRepo` -> `user`, `repo`
- Separadores de guion bajo/guion/punto: `user_repo` -> `user`, `repo`
- Límites de acrónimos: `HTMLParser` -> `html`, `parser` (se divide cuando una secuencia de mayúsculas encuentra una minúscula)

### Capacidades de Búsqueda

- **Coincidencia por prefijo**: `repo*` encuentra `UserRepository`
- **Operadores booleanos**: `user AND NOT admin`
- **Coincidencia de sub-palabras**: `"user"` encuentra `getUserById`, `UserRepository`, `user_service`
- **Búsqueda en doc comments**: Busca a través del texto de documentación
- **Resultados rankeados**: Ranking BM25 de FTS5, ordenados por relevancia

## Migración de Esquema

ctxhelpr maneja la evolución del esquema de forma elegante:

1. Una tabla `metadata` almacena la versión actual del esquema
2. Al hacer `open()`, el almacenamiento detecta si la DB es pre-migración (falta la columna `name_tokens`)
3. Si se necesita migración:
   - `ALTER TABLE` agrega la nueva columna
   - Los símbolos existentes se rellenan con `name_tokens` computados
   - Los triggers y la tabla FTS se reconstruyen
   - La versión del esquema se actualiza
4. `CREATE TABLE IF NOT EXISTS` asegura la aplicación idempotente del esquema

## Optimización de Salida

### Claves JSON Compactas

Toda la salida usa claves abreviadas para minimizar el consumo de tokens:

- `n` = name, `k` = kind, `f` = file, `l` = lines, `id` = symbol ID
- `sig` = signature, `doc` = doc comment, `p` = path

### Deduplicación de Rutas de Archivos

En respuestas con múltiples resultados (resultados de búsqueda, referencias), las rutas de archivos se deduplican:

```json
{"_f": ["src/a.rs", "src/b.rs"], "hits": [{"fi": 0, ...}, {"fi": 1, ...}]}
```

Cuando todos los resultados comparten un solo archivo, la ruta se incluye directamente (sin overhead de índice).

### Normalización de Firmas

Las firmas se normalizan para ahorrar tokens:

- Los espacios alrededor de `:`, `,` y abridores de corchetes se eliminan
- `(a: number, b: number): number` se convierte en `(a:number,b:number):number`
- Las firmas más largas que 120 caracteres (configurable vía `output.truncate_signatures`) se truncan con `...`

### Truncamiento de Doc Comments

En vistas resumidas (overview, resultados de búsqueda, símbolos de archivo), los doc comments se truncan (límite configurable vía `output.truncate_doc_comments`):

- Primera oración (terminando con `. `) si está bajo 100 caracteres (por defecto)
- Primera línea si está bajo 100 caracteres
- Truncamiento en límite de palabra con `...` en caso contrario

Las vistas de detalle (`get_symbol_detail`) devuelven firmas y docs completos, sin truncar.

### Presupuesto de Tokens

Las respuestas pueden limitarse por presupuesto:

- Por solicitud vía parámetro `max_tokens`
- Por proyecto vía `.ctxhelpr.json` `[output] max_tokens`
- Usa aproximación por longitud de bytes: `max_bytes = max_tokens * 4` (Claude promedia ~4 bytes/token)
- Truncamiento progresivo: elimina elementos del array hasta que la respuesta cabe, agrega marcador `"truncated": true`

## Arquitectura de Almacenamiento

### Bases de Datos por Repositorio

Cada repositorio obtiene su propia base de datos SQLite en `~/.cache/ctxhelpr/<sha256-prefix>.db`. Esto evita interferencia entre repos y hace la limpieza simple. Los repos indexados se pueden listar y eliminar vía los subcomandos `repos list` / `repos delete` del CLI o las herramientas MCP `list_repos` / `delete_repos`. El comando `disable` también elimina las bases de datos de índice relevantes, y `uninstall` elimina todo el directorio de caché.

### Modo WAL

SQLite se configura con `PRAGMA journal_mode=WAL` para rendimiento de lectura/escritura concurrente. Esto importa cuando el servidor MCP maneja múltiples llamadas a herramientas en paralelo.

### Triggers de FTS5

Tres triggers mantienen el índice FTS5 sincronizado:

- `symbols_ai` (after insert): Agrega nuevo símbolo al FTS
- `symbols_ad` (after delete): Elimina símbolo del FTS
- `symbols_au` (after update): Re-indexa símbolo en FTS

Esto significa que FTS siempre está consistente con la tabla de símbolos sin reconstrucciones manuales.

## Ventajas

1. **Actualizaciones incrementales rápidas** - Solo los archivos modificados se re-parsean. La detección de cambios basada en hash es confiable y rápida.
2. **Salida eficiente en tokens** - Claves compactas, deduplicación de rutas y truncamiento reducen el consumo de contexto de IA entre 30-60% comparado con salida cruda.
3. **Búsqueda inteligente de código** - Los identificadores pre-tokenizados hacen que las búsquedas de sub-strings funcionen a través de convenciones de nombres.
4. **Núcleo agnóstico de lenguaje** - Agregar un nuevo lenguaje requiere solo implementar `LanguageExtractor`. El almacenamiento, salida y búsqueda funcionan sin cambios.
5. **Sin dependencias externas** - SQLite viene integrado (vía `rusqlite`), las gramáticas de tree-sitter se compilan dentro. Binario único sin dependencias en tiempo de ejecución.
6. **Configurable por proyecto** - `.ctxhelpr.json` permite ajustes para necesidades específicas del proyecto.

## Desventajas

1. **Limitaciones de gramáticas tree-sitter** - Algunos constructos de lenguaje complejos o dinámicos pueden no parsearse correctamente. Las gramáticas de tree-sitter son "mejor esfuerzo" para cada lenguaje.
2. **Sin inferencia de tipos entre archivos** - Las referencias se resuelven por coincidencia de nombre (`refs.to_name = symbols.name`). Si dos símbolos comparten nombre, el incorrecto puede vincularse.
3. **Sin análisis runtime/dinámico** - El indexer solo ve código fuente estático. Los símbolos generados dinámicamente, metaprogramación o imports en runtime son invisibles.
4. **El presupuesto de tokens es aproximado** - La heurística de 4-bytes-por-token es un proxy aproximado. La tokenización real de Claude puede diferir entre 10-20%.
5. **Parsing single-threaded** - El parsing de archivos es secuencial dentro de una transacción. Repos muy grandes (100k+ archivos) pueden tardar varios segundos en la primera indexación.

## Casos Límite

### Problemas de Codificación

- Los archivos que no son UTF-8 válido en su ruta se omiten (`to_str()` retorna `None`)
- Los archivos binarios se leen pero típicamente no producen un parse válido de tree-sitter
- El truncamiento de firmas y doc comments es seguro para UTF-8 — los puntos de truncamiento se ajustan a límites de caracteres válidos para evitar panics con caracteres multi-byte (emoji, CJK, caracteres acentuados)

### Nombres de Símbolos Duplicados

- La resolución de referencias toma la primera coincidencia (`LIMIT 1`). Esto es correcto para la mayoría de los casos pero puede vincular incorrectamente en repos con muchos símbolos de nombre idéntico entre módulos.

### Archivos Vacíos

- Los archivos sin símbolos extraíbles igual obtienen una fila en `files` (se rastrean para detección de cambios) pero producen cero filas de símbolos/refs.

### Firmas Muy Largas

- Las firmas que superan el límite configurado (por defecto 120 caracteres) se truncan en vistas resumidas pero se preservan completas en vistas de detalle.

### Acceso Concurrente

- El modo WAL permite lecturas concurrentes durante la indexación. Sin embargo, `BEGIN IMMEDIATE` serializa las escrituras, por lo que dos operaciones de indexación simultáneas en el mismo repo se bloquearán.

### Actualizaciones de Esquema

- La migración de v1 (sin `name_tokens`) a v2 es automática. Futuros cambios de esquema deberían seguir el mismo patrón: detectar esquema viejo, alterar, rellenar, actualizar versión.

### Symlinks

- El crate `ignore` (usado para recorrer directorios) no sigue symlinks por defecto, evitando problemas de symlinks circulares.

### Monorepos Grandes

- El límite de tamaño de archivo (por defecto 1 MiB, configurable vía `indexer.max_file_size`) previene la indexación de bundles minificados o archivos generados grandes
- La lista de directorios ignorados omite `node_modules`, `target`, etc.
- Los patrones de ignorar personalizados se pueden configurar vía `.ctxhelpr.json`
