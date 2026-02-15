[![en](https://img.shields.io/badge/lang-en-lightgray.svg)](./benchmark-instructions.md)
[![es](https://img.shields.io/badge/lang-es-green.svg)](./benchmark-instructions.es.md)

# Benchmarking de ctxhelpr

Este benchmark ejecuta una comparación estandarizada de ctxhelpr contra herramientas nativas (Grep, Glob, Read) a través de 10 tareas de navegación de código. Produce un reporte `ctxhelpr-benchmark.md` con tiempos, conteo de llamadas a herramientas, y corrección para cada tarea.

Estos resultados nos ayudan a entender dónde ctxhelpr agrega valor y dónde todavía se queda corto.

## Privacidad

Antes de compartir resultados, por favor revisalos en busca de información sensible:

- Reemplazá nombres de repositorios propietarios con identificadores genéricos (ej. "my-app")
- Redactá nombres de símbolos propietarios, rutas de archivos, o términos de dominio internos
- No incluyas fragmentos de código de repositorios privados

## Cómo ejecutar

Asegurate de que ctxhelpr esté actualizado y habilitado en el repositorio que querés benchmarkear:

```text
ctxhelpr update          # actualizar a la última versión
ctxhelpr enable          # habilitar para Claude Code
```

Consultá la sección [Integración con Claude Code](./user-guide.es.md#integración-con-claude-code) para más detalles.

Después copiá el contenido de [benchmark-prompt.md](./benchmark-prompt.md) en Claude Code en ese repositorio.

## Resultados

Una vez que la ejecución termina, vas a encontrar `ctxhelpr-benchmark.md` en la raíz de tu repositorio. Si querés compartirlo, envialo a **[marcos@rigoli.dev](mailto:marcos@rigoli.dev)** — estos reportes informan directamente las prioridades de desarrollo.
