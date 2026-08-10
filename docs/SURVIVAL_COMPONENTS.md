# Componentes de supervivencia

MiniForge incluye sistemas de supervivencia configurables en el inspector. Estos componentes son
parte del motor: no contienen mapas, objetos, arte, balance ni reglas de un juego concreto.

## Flujo sin código

1. Crea un proyecto con la plantilla `Survival`.
2. Usa el arquetipo `survival_actor` para el personaje.
3. Coloca `survival_loot_container`, `survival_harvestable` y
   `survival_crafting_station` donde corresponda.
4. Configura objetos, pesos, efectos, tablas de loot y recetas editando las propiedades JSON de
   los componentes.
5. Conecta acciones desde Visual Script usando `UseInventoryItem`, `CraftRecipe`,
   `SetSurvivalNeed`, `ModifySurvivalNeed`, `BranchSurvivalNeed`, `EquipInventoryItem`,
   `UnequipToInventory`, `ApplyInjury`, `TreatInjury`, `SetCrouching`, `EmitNoise`,
   `DoorAction`, `AddBarricadeLayer`, `DamageBarricade` y `BranchAlertness`.

El `GameplaySystem` actualiza automáticamente `SurvivalNeeds` durante el modo PLAY. `Health` e
`Inventory` siguen siendo componentes independientes y pueden utilizarse en cualquier género.

## Componentes

- `Health`: vida máxima, vida actual, armadura y estado vivo.
- `Inventory`: ranuras, pilas, peso máximo opcional y orden por ID, categoría o peso.
- `SurvivalNeeds`: hambre, sed, energía, fatiga, resistencia, humedad, dolor, infección,
  sangrado, estrés, moral, higiene, enfermedad y oxígeno, con tasas configurables.
- `SurvivalEnvironment2D`: temperatura ambiente, viento, lluvia, refugio, fuente de calor,
  esfuerzo, calidad del aire, exposición a patógenos y luz diurna. Puede vivir en un actor o en una
  entidad global creada desde **Create > Survival > Survival Environment 2D**.
- `BodyCondition2D`: volumen de sangre, temperatura central, inmunidad y lesiones persistentes con
  zona corporal, sangrado, dolor, infección, gravedad y tiempo de curación.
- `NoiseEmitter2D`: radios separados para andar, correr, agacharse, interactuar y combatir;
  incorpora superficie, equipo y decaimiento para UI y gizmos.
- `StealthState2D`: agachado, cobertura, exposición lumínica y multiplicadores de movimiento,
  visibilidad y ruido.
- `Senses2D`: cono visual, alcance auditivo, etiquetas objetivo, alerta, memoria y último estímulo.
- `Door2D`: apertura progresiva, colisión sincronizada, cerradura, llave, ruido y cierre automático.
- `Barricade2D`: capas, material, herramienta, coste declarativo, resistencia y salud persistente.
- `Equipment`: ranuras de mano primaria/secundaria, cabeza, torso, manos, piernas, pies, espalda,
  abalorio y herramienta; admite objetos de varias ranuras, durabilidad, aislamiento, protección,
  impermeabilidad, peso y bonificadores de estadísticas.
- `SurvivalUIBinding`: enlaza una barra o etiqueta de `UIElement` con vida, una necesidad, peso o
  ranuras del inventario; se actualiza automáticamente durante PLAY.
- `LootContainer`: contenido persistente, tabla inicial, tabla oculta y estados de registro y
  rebusca.
- `CraftingBook`: recetas conocidas por un actor.
- `CraftingStation`: recetas disponibles en una estación y etiquetas de estación.
- `Harvestable`: recurso, cantidad, rendimiento, herramienta requerida y regeneración opcional.

## Objetos consumibles

Los efectos viven en los metadatos del objeto; no hace falta crear una clase nueva:

```json
{
  "id": "consumable_a",
  "quantity": 2,
  "metadata": {
    "category": "consumable",
    "weight": 0.5,
    "effects": {
      "thirst": 30.0,
      "health": 5.0,
      "infection": -2.0
    }
  }
}
```

`UseInventoryItem` o `GameAPI::use_item` consume una unidad y aplica automáticamente los efectos.

## Equipamiento declarativo y atómico

Un objeto del inventario puede indicar su carga completa en metadatos. No necesita una clase de
arma o armadura:

```json
{
  "id": "chaqueta_bombero",
  "quantity": 1,
  "metadata": {
    "category": "equipment",
    "weight": 4.2,
    "equipment": {
      "slot": "torso",
      "occupies": ["torso", "back"],
      "durability": 100.0,
      "protection": 18.0,
      "insulation": 0.72,
      "waterproofing": 0.85,
      "bonuses": {"strength": 2.0, "noise": 0.15}
    }
  }
}
```

`EquipInventoryItem` retira el objeto, libera y devuelve las piezas desplazadas, ocupa todas las
ranuras declaradas y revierte la operación completa si el inventario no puede recibir una pieza.
`UnequipToInventory` usa la misma transacción. La compatibilidad con `weapon` y `armor` se mantiene,
pero los juegos nuevos pueden usar las ranuras extendidas sin programar su propio gestor.

## Lesiones y ambiente

`ApplyInjury` crea una herida persistente por zona corporal. Cada tick puede producir pérdida de
sangre, dolor, infección y daño; inmunidad, higiene, exposición a patógenos y temperatura modifican
su progreso. Un consumible de tratamiento declara en sus metadatos qué reduce:

```json
{
  "id": "vendaje_esteril",
  "quantity": 2,
  "metadata": {
    "treatment": {
      "bleeding": 45.0,
      "infection": 12.0,
      "pain": 8.0,
      "healing_multiplier": 1.35
    }
  }
}
```

El tick ambiental combina aislamiento e impermeabilidad del equipo con temperatura, viento,
precipitación, refugio, calor y esfuerzo. También aplica encumbramiento, consumo de oxígeno,
estrés, moral, higiene y enfermedad con límites seguros ante valores inválidos.

## Sigilo, percepción y mundo barricable

Desde **Create > Survival** se pueden crear tres actores listos para editar:

- **Survival Actor 2D** incluye necesidades, cuerpo, inventario, equipo, sigilo y emisión de ruido.
- **Survival Hunter / Zombie 2D** incluye vista, oído, memoria de alerta, navegación y combate.
- **Barricadable Door 2D** incluye interacción, colisión, cerradura, audio, capas y guardado.

`SurvivalWorldSystems::emit_noise` devuelve un `NoiseEvent2D` serializable en coordenadas del
mundo. `perceive` y `update_perception` comparan candidatos con el cono visual y esos eventos de
ruido, seleccionan el estímulo más fuerte y actualizan la alerta sin depender del renderer. El
resultado sirve para IA de zombis, guardias, animales o enemigos de cualquier género.

Las acciones de puerta son deterministas: una puerta cerrada conserva colisión, una abierta la
libera, una cerradura valida `key_id` y una barricada impide abrir hasta que su salud llegue a cero.
`barricade_from_inventory` exige la herramienta y materiales declarados en `Barricade2D`, consume
los materiales sólo si se añade la capa y mantiene la operación atómica.

## Recetas declarativas

```json
{
  "id": "recipe_a",
  "ingredients": [
    {"id": "material_a", "quantity": 2}
  ],
  "outputs": [
    {
      "id": "crafted_a",
      "quantity": 1,
      "metadata": {"category": "tool", "weight": 0.8}
    }
  ]
}
```

La fabricación es atómica: si faltan ingredientes, ranuras o capacidad de peso, el inventario no
se modifica parcialmente.

## Tablas de loot

`loot_entries` y `hidden_entries` usan entradas ponderadas:

```json
[
  {
    "id": "material_a",
    "weight": 2.0,
    "min": 1,
    "max": 3,
    "metadata": {"category": "material", "weight": 0.2}
  }
]
```

El contenido se genera una sola vez de forma determinista y los estados `searched`, `rummaged` e
`items` se serializan con la entidad.

## API de alto nivel

Para integraciones avanzadas, `SurvivalSystems` y `GameAPI` exponen operaciones directas:

- `survival_state`, `survival_need`, `set_survival_need` y `modify_survival_need`.
- `tick_survival` y `tick_survival_in_environment`.
- `use_item`, `sort_inventory` e `inventory_weight`.
- `equip_inventory_item`, `unequip_to_inventory`, `equipment_summary`, `effective_stat` y
  `degrade_equipment`.
- `apply_injury` y `treat_injury`.
- `set_crouching`, `stealth_movement_multiplier`, `stealth_visibility`, `emit_noise`, `tick_noise`
  y `perceive`.
- `door_action`, `tick_door`, `add_barricade_layer`, `barricade_from_inventory` y
  `damage_barricade`.
- `search_loot_container`, `rummage_loot_container`, `take_container_item` y
  `take_all_container_items`.
- `can_craft`, `craft` y `craft_at`.
- `harvest` y `survival_interact`.

`survival_state` produce un modelo listo para enlazar a HUD: vida, necesidades, cuerpo, lesiones,
equipamiento, objetos, ranuras y peso. El diseñador de UI sugiere rutas como
`player.needs.hunger`, `player.needs.thirst` y
`player.inventory.weight`. La plantilla Survival incluye cinco `UIElement` reales enlazados con
`SurvivalUIBinding`: vida, hambre, sed, energía y resistencia. Se pueden mover o rediseñar sin
programar su actualización.
