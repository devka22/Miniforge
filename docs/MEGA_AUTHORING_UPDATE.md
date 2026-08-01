# Mega Authoring Update — fases 1 y 2

Esta fase convierte los sistemas reutilizables de MiniForge en una API de autoría
común para cualquier género 2D. El editor ya no mantiene una lista QML separada:
Rust es la única fuente de verdad y publica el catálogo completo al editor Qt.

## Catálogo unificado

`AuthoringCatalog2D::builtin()` incluye 49 presets para:

- jugadores top-down, plataformas, twin-stick, action RPG, roguelike, sigilo,
  supervivencia y gravedad cero;
- enemigos, jefes, compañeros y NPC con horarios;
- diálogo, misiones, comercio, inventario, equipamiento, combate, crafting,
  cosecha, loot, puertas, checkpoints, puzzles, plataformas y peligros;
- cámaras, audio, clima, parallax, iluminación, cinemáticas, HUD, streaming y
  generación procedural;
- RTS y gran estrategia;
- perfiles físicos top-down, plataforma, rebote, peso, sensores, one-way y CCD.

Cada preset declara componentes, overrides, parámetros editables, requisitos,
pasos guiados, recomendaciones, géneros, tags, madurez y tiempo estimado. Los
alias mantienen compatibilidad con los nombres anteriores.

## Flujo sin programar

El editor ofrece tres entradas al mismo catálogo:

1. pestaña **Systems**;
2. menú **Systems** con filtros por familia;
3. ventana **Add Component** del inspector.

El **Mega Authoring Hub** permite buscar, filtrar, marcar favoritos y aplicar un
sistema a uno o varios objetos. Los valores comunes —velocidad, vida, masa,
daño y capacidad de inventario— se configuran antes de aplicar el preset. El
paso de revisión calcula, sin mutar la escena, qué componentes se añadirán a
cada objeto, cuáles ya existen, si cambiará el mundo físico y cuáles son los
primeros pasos y requisitos. Así el modo rápido sigue siendo simple, pero el
creador puede comprobar el cambio exacto antes de confirmarlo. El
payload usa la misma acción pública que el resto del editor:

```json
{
  "bundle": "topdown_player",
  "parameters": {
    "movement_speed": 7.5,
    "maximum_health": 150
  }
}
```

El motor limita y normaliza cada valor usando el contrato del preset. Un
parámetro inválido no puede insertar `NaN`, infinito ni valores fuera de rango.

## Física 2D profesional

`Physics2DSettings` incorpora:

- frecuencia fija configurable;
- substeps adaptativos;
- iteraciones del solver;
- CCD global o solicitado por cuerpo;
- sleeping activable;
- gravedad, layers y matriz de colisiones;
- normalización y diagnóstico estructurado.

Los presets físicos pueden configurar simultáneamente el cuerpo seleccionado y
el mundo activo. `Physics2DSettings::validate()` detecta gravedad no finita,
frecuencias/substeps inválidos, costos extremos, capas duplicadas y referencias
desconocidas. `ProjectSettings2D::validate()` integra esos diagnósticos.

La deserialización conserva compatibilidad con configuraciones antiguas: los
campos nuevos tienen defaults seguros.

## API pública

La API de motor expone:

- `AuthoringCatalog2D`, `AuthoringPreset2D` y `AuthoringApplicationPlan2D`;
- búsqueda por texto y tipo;
- resolución por ID o alias;
- previsualización de componentes nuevos/existentes;
- configuración tipada de componentes;
- `PhysicsRuntimeTuning2D`;
- serialización JSON del catálogo;
- `mf_editor_authoring_catalog_json` en la ABI C;
- `mf_editor_authoring_plan_json` para previsualizar la aplicación a la
  selección actual;
- `MfBridge::authoringCatalogJson()` y `MfBridge::authoringPlanJson()` en Qt.

El catálogo puede consumirse desde otro frontend sin copiar reglas del editor.

## Tamaño del SDK

El peso del motor debe representar capacidad real. `target/` y `build/` son
artefactos regenerables, no contenido del producto. La meta de distribución de
5–10 GB se construirá con paquetes opcionales versionados —toolchains,
plantillas, shaders, materiales, sonidos, fuentes, assets de ejemplo y símbolos
de depuración— fuera del repositorio Git del motor.

Así GitHub mantiene sólo código y recursos del motor, nunca juegos de usuario,
y cada creador instala únicamente los paquetes que necesita.

La fase 2 incorpora el catálogo y el planificador reales: **Creator 2D** resuelve
aproximadamente 6.05 GiB y **Studio Heavy** aproximadamente 9.03 GiB. El editor
los muestra en la pestaña **SDK Packs**, calcula dependencias, descarga y tamaño
final, y exige manifests SHA-256 de release antes de una futura instalación
atómica. Consulta `docs/SDK_CONTENT_PACKS.md`.

## Compatibilidad

- Los bundles anteriores siguen funcionando por alias.
- El catálogo valida que todo componente exista en el registro.
- La API C es aditiva y no cambia layouts existentes.
- QML obtiene los datos desde la ABI; no contiene presets hardcodeados.
- Los proyectos y juegos permanecen fuera del alcance de esta actualización.
