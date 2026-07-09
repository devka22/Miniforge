# MiniForge TypeScript Plugin Contract

This folder defines the first public contract for editor plugins. Runtime hosting is intentionally not implemented in this cut; future phases compile TypeScript with esbuild and execute JavaScript inside a sandboxed QuickJS host.

## Files

- `api/miniforge-editor.d.ts`: versioned API surface available to plugins.
- `schema/plugin-manifest.schema.json`: manifest validation contract.
- `examples/hello-plugin`: minimal command and notification example.

## Local Checks

```bash
npm install
npm run typecheck
npm run build:example
```

Plugins never receive native pointers. All mutations must go through editor commands exposed by Rust `EditorCore`.
