# MiniForge Editor Plugin API Direction

Esta fase no implementa plugins TypeScript. Define la dirección para que el editor Qt no cierre esa puerta.

El contrato inicial vive en `editor-plugins/typescript/`:

- `api/miniforge-editor.d.ts` define la API versionada de plugins.
- `schema/plugin-manifest.schema.json` define permisos y metadata.
- `examples/hello-plugin/` muestra un comando mínimo.

## Runtime Propuesto

```text
TypeScript plugin -> esbuild/swc -> JavaScript -> QuickJS host
```

QuickJS es la opción inicial por costo bajo y portabilidad. V8 no se usará salvo que una fase futura justifique su peso.

## API Objetivo

```ts
miniforge.editor.registerCommand(...)
miniforge.editor.registerPanel(...)
miniforge.editor.registerImporter(...)
miniforge.editor.registerMenuItem(...)
miniforge.editor.getSelection()
miniforge.editor.executeCommand(...)
miniforge.assets.find(...)
miniforge.scene.query(...)
miniforge.notifications.show(...)
```

Los plugins no reciben punteros nativos. Toda mutación debe pasar por comandos de `EditorCore`.

## Manifest

```json
{
  "name": "example-plugin",
  "version": "1.0.0",
  "apiVersion": "1",
  "permissions": ["scene.read", "scene.write", "assets.read"]
}
```
