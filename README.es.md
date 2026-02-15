[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./README.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./README.es.md)

# ctxhelpr

![status: experimental](https://img.shields.io/badge/status-experimental-orange)

## Indexación semántica de código para Claude Code

Un servidor [MCP](https://modelcontextprotocol.io) que pre-indexa tu repositorio usando [tree-sitter](https://tree-sitter.github.io/) - funciones, clases, tipos, referencias, cadenas de llamadas - y guarda todo en una base de datos SQLite local. Claude Code navega tu código a través de herramientas específicas en lugar de leer miles de líneas de código crudo.

El resultado: construcción de contexto más rápida, menos tokens gastados, y Claude _entiende_ la estructura de tu código antes de tocarlo.

> [!WARNING]
> Este proyecto es **experimental** y está en desarrollo activo. No hay garantía de que el contexto indexado sea más efectivo que el que un agente de código construye por su cuenta. Usalo bajo tu propio riesgo.

## Primeros pasos

```text
curl -sSfL https://sh.ctxhelpr.dev | sh
```

Después habilitalo en Claude Code:

```text
ctxhelpr enable
```

Ejecutá `ctxhelpr --help` para todos los comandos del CLI.

## Aspectos destacados

- **Indexación incremental** - hashing SHA256 de contenido, solo se re-parsean archivos modificados
- **Búsqueda inteligente de código** - buscar "user" encuentra `getUserById`, `UserRepository`, `user_service`
- **Salida eficiente en tokens** - claves compactas, deduplicación de rutas, presupuestos configurables
- **11 herramientas MCP** para navegación estructural

## Privacidad

ctxhelpr se ejecuta completamente en tu máquina. Tu código nunca sale de tu entorno local - toda la indexación, almacenamiento y consultas ocurren localmente. El único acceso externo a la red ocurre cuando ejecutás explícitamente `ctxhelpr update` para buscar nuevas versiones.

## Soporte de lenguajes

- TypeScript / TSX / JavaScript / JSX
- Python
- Rust
- Ruby
- Markdown

## Documentación

- [Guía de Usuario](docs/user-guide.es.md) - instalación, configuración, referencia de herramientas, detalles del CLI
- [Guía de Desarrollo](docs/developer-guide.es.md) - compilación desde fuente, arquitectura, contribución
- [Estrategia de Indexación](docs/indexing-strategy.es.md) - profundización en la arquitectura de indexación
- [Changelog](CHANGELOG.md)

Toda la documentación está disponible en [inglés](README.md) también.

## Licencia

[MIT](LICENSE)
