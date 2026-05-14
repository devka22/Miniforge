# MiniForge 0.8.0 Release Checklist

## Build

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --all-targets`
- [ ] GitHub Actions verde en pull request/tag.

## Manual QA

- [ ] Crear proyecto nuevo.
- [ ] Abrir proyecto existente 0.7.x.
- [ ] Corromper `engine_config.json` y verificar recuperacion.
- [ ] Crear `.rhai`, abrirlo, editarlo, guardar, entrar a Play Mode.
- [ ] Crear `.mfgraph`, abrirlo, validar y adjuntarlo a entidad.
- [ ] Guardar/cargar/reiniciar escena.
- [ ] Instanciar prefab y probar recuperacion desde `.prefab.bak`.
- [ ] Validar assets faltantes y referencias rotas.
- [ ] Exportar debug y release.
- [ ] Abrir runtime exportado con `miniforge_runtime`.

## Publicacion

- [ ] Actualizar version crate y `ENGINE_VERSION`.
- [ ] Revisar `README.md`, Developer Guide y release notes.
- [ ] Confirmar que no quedan runtimes legacy fuera de `.git`/`target`.
- [ ] Etiquetar `v0.8.0`.
- [ ] Adjuntar notas de compatibilidad y troubleshooting.
