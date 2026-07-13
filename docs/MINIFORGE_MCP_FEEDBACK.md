# MiniForge - Mejoras Recomendadas

Este archivo es backlog técnico, no documentación de capacidades. Mantiene el nombre
`MINIFORGE_MCP_FEEDBACK.md` porque el servidor MCP escribe feedback en esta ruta. Consulta el
[índice](README.md) y las guías canónicas antes de asumir que una recomendación sigue pendiente.

## Revisión De Estado 2026-07-13

| Ítem | Estado actual |
|---|---|
| 1. Loop compartido | Parcial: orden equivalente y tests; aún hay duplicación Game/EngineRuntime |
| 2. Límite editor/runtime | Implementado en lo esencial: `editor_core`, runtime-only gate y editor visual Rust retirado |
| 3. Versionado de documentos | Parcial: escenas/prefabs/assets/graphs cubiertos; quedan formatos secundarios |
| 4. Referencias GUID | Parcial: Asset Database sólido; varios componentes aún usan path |
| 5. Prefab bidireccional | Implementado: apply/revert/variant/detach y Prefab Studio; requiere escala real |
| 6. Registry tipado/generable | Parcial |
| 7. Unificación UI | Pendiente; conviven tres modelos compatibles |
| 8. Visual Graph registrado | Parcial: authoring/serializer/runtime integrados |
| 9. Sandbox de plugins | Pendiente más allá de Safe Mode/capabilities |
| 10. WGPU/Metal | Experimental |
| 11. Viewport Qt | Baseline implementado: Scene/Game, gizmos, selección múltiple y overlays |
| 12. Stubs/docs Luau | Implementado: `types/miniforge.luau`, API browser, completions y debugger |
| 13. Spatial index incremental | Parcial: índice/revisiones/queries existen |
| 14. Scheduler con budgets | Infraestructura implementada; adopción universal pendiente |
| 15. Fixtures export/packaging | Parcial |
| 16. Migrador Rhai | Pendiente; Luau es el runtime vigente |
| 17. Project Health Matrix | Implementado mediante Readiness/Runtime Health/Profiler |
| 18. Docs generadas | Pendiente; documentación canónica actualizada manualmente |
| 19. Prefabs avanzados | Implementado como workflow Qt/backend |
| 20-22. AI/UI/3D | Parcial o experimental según área |
| 23. MCP de producción | Parcial |

## Prioridad P0

### 1. Unificar El Loop De Frame Entre Editor Y Runtime

`Game::run_headless_once` y `EngineRuntime::run_headless_once` tienen mucho flujo duplicado: animacion, particulas, audio, graphs, Luau, runtime 2D, gameplay, RTS, movimiento, fisica, colisiones, camara, mundo, diagnosticos y profiler.

Mejora propuesta:

- Extraer un `FramePipeline` o usar `SystemScheduler` como unica orquestacion.
- Compartir orden, medicion y safe mode entre editor/runtime.
- Permitir que el editor agregue pasos propios sin duplicar runtime.

Beneficio:

- Menos regresiones cuando se cambia un sistema.
- Pruebas de orden de sistemas mas simples.
- Export runtime mas confiable.

### 2. Endurecer El Limite Editor/Runtime

Ya existe `EngineRuntime` sin `Game`, pero el motor tiene muchos modulos en `engine` mezclando editor, runtime y tooling.

Mejora propuesta:

- Agregar tests de compilacion `--no-default-features --features runtime` en CI.
- Crear una lista de modulos permitidos para runtime.
- Mantener APIs editor-only detrás de `#[cfg(feature = "editor_core")]` y `editor_ffi` cuando
  corresponda.

Beneficio:

- Builds exportados mas pequenos.
- Menos riesgo de dependencias UI/desktop en runtime.

### 3. Completar Versionado De Documentos

Escenas y prefabs ya tienen schemas. Faltan contratos igual de estrictos para algunos documentos 2D, UI, tilemaps, build profiles, plugin manifests y tools.

Mejora propuesta:

- Exigir `format`, `schema_version` y `engine_version` en todos los documentos generados por el editor.
- Agregar migradores con fixtures golden.
- Agregar rechazo de version futura para cada formato.

Beneficio:

- Proyectos viejos migran de forma segura.
- Los errores se reportan antes de corromper datos.

### 4. Consolidar Referencias Por GUID

`AssetDatabase` conserva GUIDs y reconcilia moves por hash, pero todavia hay muchas rutas string en componentes, escenas y scripts.

Mejora propuesta:

- Migrar `sprite_path`, `script`, `material_path`, tilemaps, UI images, audio y prefabs a referencias `{ guid, fallback_path }`.
- Mantener paths solo como fallback humano.
- Agregar herramienta de migracion y validacion.

Beneficio:

- Mover assets no rompe escenas.
- Export detecta dependencias reales con mayor precision.

### 5. Hacer El Prefab Workflow Bidireccional

El schema 2 ya guarda GUID, scripts/settings requeridos y dependencies. Falta una ruta completa para aplicar cambios entre prefab source e instancias.

Mejora propuesta:

- Implementar diff estable de overrides.
- Revert/apply overrides desde el editor.
- Validar dependencias antes de instanciar.
- Mostrar warnings de scripts/settings faltantes en inspector.

Beneficio:

- Prefabs dejan de ser solo "plantillas guardadas" y pasan a ser un workflow profesional.

## Prioridad P1

### 6. Registry De Componentes Tipado Y Generable

`default_component` es grande y centraliza defaults, categorias y parte del contrato. Es util, pero cada componente existe como string.

Mejora propuesta:

- Definir descriptors tipados para componentes.
- Generar defaults JSON, inspector fields, docs y validadores desde el mismo descriptor.
- Mantener compatibilidad con componentes dinamicos.

Beneficio:

- Menos errores de typo.
- Inspector mas rico.
- Documentacion automatica.

### 7. Unificar UIElement, UiCanvasRoot Y UiCanvas2D

Ahora conviven tres modelos UI. Eso ayuda a compatibilidad, pero complica runtime, editor y documentacion.

Mejora propuesta:

- Declarar `UiCanvas2D` como modelo moderno.
- Crear migrador desde `UIElement` y `UiCanvasRoot`.
- Mantener adapter legacy durante una ventana de compatibilidad.

Beneficio:

- Menos caminos de hit testing/layout.
- UI Designer y runtime hablan el mismo contrato.

### 8. Convertir Visual Graph En Sistema De Nodos Registrado

`VisualScriptRuntime` soporta muchos nodos en un match grande. Funciona, pero sera dificil crecer.

Mejora propuesta:

- Crear un registry de nodos con metadata, pins, validacion y executor.
- Documentar cada nodo desde el registry.
- Separar nodos runtime-safe de editor-only.

Beneficio:

- Menos deuda al agregar nodos.
- Mejor autocompletado y UI del graph.

### 9. Fortalecer Sandbox De Plugins

Hay TypeScript, C#, Python editor-only y native libraries. El poder de extension es alto, pero tambien el riesgo.

Mejora propuesta:

- Declarar permisos por plugin.
- Validar capabilities antes de cargar.
- Firmar o confiar explicitamente native libraries.
- Loggear operaciones sensibles.
- Separar plugin runtime-safe de plugin editor-only.

Beneficio:

- Proyectos de terceros mas seguros.
- Safe Mode mas explicito.

### 10. Completar Backend WGPU/Metal

La configuracion ya conoce WGPU/Metal, compute jobs, GPU particles y tile visibility, pero el backend estable es Macroquad.

Mejora propuesta:

- Definir un milestone pequeno para WGPU: clear, sprite batch, texture atlas, viewport.
- Luego tile chunks, UI, post process.
- Finalmente compute particles/tile culling.

Beneficio:

- Mejor rendimiento en proyectos grandes.
- Ruta clara para Apple Silicon y builds futuros.

### 11. Mejorar El Viewport Qt

QML ya tiene shell, paneles y placeholder de viewport. Falta un viewport real conectado a render/snapshot.

Mejora propuesta:

- Usar `mf_editor_viewport_snapshot_rgba` como primera integracion.
- Luego agregar input y seleccion.
- Finalmente render live con textura compartida o backend dedicado.

Beneficio:

- Editor Qt pasa de shell de datos a editor visual usable.

### 12. Generar Stubs Y Docs De Luau

Existe `types/miniforge.luau` y validacion Luau, pero el API real crece con comandos runtime.

Mejora propuesta:

- Generar stubs desde `ScriptCommand` y `GameAPI`.
- Publicar lifecycle, comandos, payloads de eventos y ejemplos desde fuente.
- Validar ejemplos en tests.

Beneficio:

- Menos drift entre codigo y docs.
- Mejor experiencia en script editor.

## Prioridad P2

### 13. Incrementalizar El Spatial Index

`RuntimeWorld` puede insertar/remover en indice, pero el frame loop reconstruye el indice al final.

Mejora propuesta:

- Medir costo con benchmarks reales.
- Mantener lista de entidades sucias.
- Reindexar incrementalmente cuando el porcentaje sucio sea bajo.
- Reconstruir completo solo sobre cambios masivos.

Beneficio:

- Mejor performance con mundos grandes.

### 14. Scheduler De Sistemas Con Presupuestos

Existe `SystemScheduler`, pero el loop de frame aun es manual.

Mejora propuesta:

- Registrar sistemas con grupo, prioridad, modo, safe mode y budget.
- Soportar fixed/update/late.
- Reportar saturacion por sistema.

Beneficio:

- Runtime mas predecible.
- Diagnosticos mas utiles.

### 15. Tests De Export Y Packaging Con Fixtures Reales

Export y packaging son rutas criticas. Hoy tienen validacion, pero conviene blindar mas.

Mejora propuesta:

- Fixture de proyecto minimo exportable.
- Fixture con asset faltante.
- Fixture con prefab dependency faltante.
- Test de `runtime_manifest.json`.
- Test de package sin runtime binary y con runtime binary simulado.

Beneficio:

- Menos fallos al distribuir juegos.

### 16. Migrador De Rhai

`rhai_scripting` esta deprecado, pero aun hay proyectos/scripts `.rhai`.

Mejora propuesta:

- Crear herramienta de auditoria para scripts Rhai.
- Marcar componentes y prefabs que referencian Rhai.
- Proveer migracion asistida a Luau o visual graph.

Beneficio:

- Limpieza gradual sin romper proyectos existentes.

### 17. Integrar Project Health Matrix Al Editor

Existen tools de health matrix y readiness. Conviene verlas en el editor.

Mejora propuesta:

- Panel "Project Health".
- Agrupar errores/warnings por area.
- Botones de auto-fix seguro.
- Acciones sugeridas por ForgeAI en dry-run.

Beneficio:

- Menos salto entre CLI y editor.

### 18. Documentacion Generada Desde Codigo

Esta consolidacion deja 4 documentos humanos mas este archivo de mejoras, pero el motor ya es suficientemente grande para generar secciones.

Mejora propuesta:

- Generar tabla de componentes desde `ComponentRegistry`.
- Generar comandos Luau desde `ScriptCommand`.
- Generar formatos desde serializers.
- Generar CLI desde `miniforge_dev`.
- Mantener una seccion manual para arquitectura y decisiones.

Beneficio:

- La documentacion no vuelve a quedar obsoleta.

## Prioridad P3

### 19. Editor De Prefabs Avanzado

Mejora propuesta:

- Abrir prefab como documento.
- Ver dependency report.
- Preview de entidad.
- Override diff por instancia.
- Apply/Revert.

### 20. Navegacion Y AI Mas Completa

Mejora propuesta:

- Unificar `NavAgent`, grid, flow fields y behavior trees.
- Exponer queries a Luau.
- Debug draw de path, influence y threat maps.

### 21. Runtime De UI Mas Profesional

Mejora propuesta:

- Layout responsive con breakpoints.
- Navegacion gamepad/keyboard.
- Accessibility audit integrado.
- Transiciones y animaciones UI.

### 22. Render 3D Hibrido Por Vertical Slice

Mejora propuesta:

- Primer slice: cube, camera, light, billboard y overlay 2D.
- Segundo: materiales y depth.
- Tercero: shadows.
- Mantener gameplay 2D como estable.

### 23. Mejorar MCP Como API De Produccion

Mejora propuesta:

- Separar generadores hardcodeados en templates declarativos.
- Usar `ProjectTemplates` nativo cuando sea posible.
- Exponer validate/export/readiness como tools.
- Mantener feedback en este archivo o en una seccion estructurada.

## Secuencia Recomendada

1. Extraer `FramePipeline` compartido.
2. Agregar checks runtime-only en CI.
3. Crear descriptors generables de componentes.
4. Migrar referencias de assets a `{ guid, fallback_path }`.
5. Completar prefab overrides.
6. Integrar viewport Qt por snapshot RGBA.
7. Generar docs/stubs desde registry y comandos.
8. Crear fixtures fuertes de export/packaging.

## Nota Sobre Feedback Historico

El feedback anterior incluia recomendaciones sobre:

- version visible del motor.
- generacion de proyectos como API/CLI nativa.
- respetar `start_scene`.
- sistema mensual de gran estrategia.
- mover templates generados a manifiestos declarativos.

Estado actual observado:

- La version visible esta centralizada en `engine::version`.
- `start_scene` se respeta en `Game` y `EngineRuntime`.
- Hay CLI de desarrollo y templates nativos.
- Hay componentes de gran estrategia.
- Aun conviene mover mas templates MCP a manifiestos declarativos.
