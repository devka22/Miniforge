# MiniForge Prefab System

Estado real: formato schema 1 con prefabs simples, variantes básicas, diff y recuperación. Los prefabs usan JSON con extensión `.prefab`.

## Cabecera

```json
{
  "format": "miniforge.prefab",
  "schema_version": 1,
  "engine_version": "0.9.3",
  "version": "0.9.3"
}
```

`schema_version` es independiente de la versión del motor. Un prefab sin schema se trata como versión 0 y recibe la cabecera en memoria.

## Contrato mínimo

```json
{
  "prefab_name": "Hero",
  "entity": {
    "name": "Hero",
    "components": []
  }
}
```

El loader exige `entity` object, nombre no vacío y `components` array. Mantiene campos adicionales usados por `AdvancedPrefabSystem`, como `guid`, `variant`, `parent_guid`, `parent_source`, `dependencies`, `overrides` y `metadata`.

## Operaciones implementadas

- Crear prefab desde una entidad.
- Instanciar con ID runtime nuevo.
- Aplicar una instancia a su source.
- Revertir o desconectar una instancia.
- Crear variante con referencia al prefab padre.
- Calcular diferencias básicas y conteo de overrides.
- Recuperar JSON corrupto desde backups rotativos.
- Rechazar schemas futuros antes de usar un backup antiguo.

## Persistencia y migración

`PrefabManager`, `AdvancedPrefabSystem` y la aplicación de instancias emiten schema 1. Los prefabs legacy se migran en memoria y no se reescriben hasta que el usuario guarda. La escritura usa `ProjectStorage` y conserva tres versiones `.prefab.bak*` cuando se reemplaza un source.

## Límites actuales

Todavía no están terminados los prefabs anidados, resolución recursiva de variantes, overrides tipados por propiedad, detección completa de ciclos, actualización automática de todas las instancias y unpack parcial. Esos elementos permanecen en Fase 3; esta documentación no los presenta como disponibles.

Los golden tests viven en `tests/fixtures/formats/` y las pruebas de loader/saver en `tests/schema_versioning.rs`.
