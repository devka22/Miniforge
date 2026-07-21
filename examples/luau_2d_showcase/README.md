# Luau 2D Showcase

Este ejemplo demuestra la API Luau 2D pública de MiniForge:

- movimiento con `CharacterBody2D`;
- física, callbacks de colisión, triggers y `Physics2D.raycast`;
- `AnimatedSprite` / `AnimationPlayer`;
- `Camera.main():follow`, zoom, pixel-perfect y shake;
- proyectiles creados desde Luau;
- enemigos, eventos y timers;
- consultas/edición de `Tilemap`;
- hot reload al guardar archivos de `scripts/`.

Desde la raíz del motor:

```bash
scripts/run-editor --project examples/luau_2d_showcase --workspace Scripting
cargo run --no-default-features --features runtime --bin miniforge_headless \
  -- examples/luau_2d_showcase 120
```

Abre la escena principal, entra en Play Mode y edita `scripts/PlayerController.luau`. El watcher
recarga los scripts guardados. Usa Luau Studio para diagnostics, completions y debugger. Consulta
la [guía de scripting](../../docs/DATOS_SCRIPTING_Y_APIS.md#luau).
