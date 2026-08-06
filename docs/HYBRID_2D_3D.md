# Escenas híbridas 2D + 3D

MiniForge permite conservar gameplay, colisiones, navegación y guardado en 2D mientras una
presentación 3D consume las mismas entidades. El puente es parte del motor y no depende de Ashfall,
Macroquad ni de un tipo de juego concreto.

## Flujo desde el editor

1. Usa **Create > Hybrid 2D + 3D > Hybrid World 2D + 3D**. Se crean `HybridScene3D`, `Camera3D`,
   `Light3D` y `Saveable`, con física 2D autoritativa y overlay 2D activos.
2. Usa **Hybrid Billboard Actor** para un actor nuevo, o agrega `HybridAnchor2D3D` y `Billboard3D`
   a uno existente.
3. Asigna el sprite y el controlador `Animator2D` de siempre. `Billboard3D.use_2d_animation`
   reutiliza esos fotogramas en la presentación 3D.
4. Ajusta escala mundial, elevación, tamaño del billboard, pitch y yaw desde Inspector o desde los
   presets `hybrid_world_2d3d` y `hybrid_billboard_actor` del Authoring Hub.

**Create > Survival > Survival Actor 2D** ya incluye ancla híbrida, billboard y animador. El mismo
actor mantiene todos sus componentes de vida, necesidades, cuerpo, inventario y equipamiento.

## Componentes reutilizables

- `HybridScene3D`: activa el mundo, define conversión de unidades, suelo, cámara, buffer de
  profundidad, overlay 2D y la autoridad física.
- `HybridAnchor2D3D`: elige `from_2d`, `from_3d` o `manual`, más elevación, sesgo de profundidad y
  comportamiento de sombras.
- `Billboard3D`: sprite, ancho/alto mundial, orientación a cámara, bloqueo del eje vertical y uso de
  animación 2D.
- `Transform3D`, `MeshRenderer3D`, `Material3D`, `Camera3D` y `Light3D`: presentación 3D convencional
  para escenarios, suelo, utilería y luces.

## Puente de runtime

`HybridWorldSettings2D3D` realiza la conversión determinista 2D ↔ X/Z 3D. La altura 3D queda libre
para pisos, saltos visuales y objetos elevados. `sync_entity_hybrid_transform` sincroniza una
entidad según su modo de autoridad y crea `Transform3D` cuando hace falta.

`HybridFramePlan2D3D::from_entities` extrae un plan independiente del backend con:

- posición 2D y 3D por entidad;
- sprite y tamaño de billboard;
- banderas de cámara y sombras;
- conteo de billboards, mallas y overlays 2D;
- orden de profundidad estable por Z, elevación e ID.

Un renderer puede consumir el mismo plan en WGPU, Macroquad u otro backend sin conocer las reglas
del juego. Esto evita duplicar posiciones, IA, colisiones o guardado al mezclar arte 2D con suelo,
edificios, iluminación y cámara 3D.

```rust
use miniforge::engine::hybrid_scene::{
    HybridFramePlan2D3D, HybridWorldSettings2D3D, sync_entity_hybrid_transform,
};

let settings = HybridWorldSettings2D3D::from_entities(&world.entities);
for entity in &mut world.entities {
    sync_entity_hybrid_transform(entity, &settings);
}
let frame = HybridFramePlan2D3D::from_entities(&world.entities);
```

La simulación no necesita cambiar al adoptar otra presentación: un juego top-down, RPG, acción,
estrategia, plataformas con decorado 3D o supervivencia puede reutilizar el mismo flujo.
