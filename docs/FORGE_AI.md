# ForgeAI

ForgeAI is MiniForge's AI orchestration layer. It is intentionally separate
from the editor and runtime: requests become a visible plan, the plan becomes
typed actions, actions are previewed and validated, and execution happens only
through a stable host API.

## First vertical slice

The first implemented slice supports:

- Project context summaries for scenes, entities, components, assets, scripts,
  visual graphs, prefabs and physics counts.
- Typed actions for entity creation, component edits, Luau script generation,
  prefab creation, validation, tests and performance analysis.
- A deterministic local provider for known engine tasks.
- `Project Doctor` diagnostics with severity, evidence and proposed fixes.
- A MiniForge Luau API document used by validation and recommendation systems.
- A VS Code-style recommendation foundation based on structured API symbols.

## Required enemy case

The request:

```text
Crea un enemigo 2D con vida, patrulla, persecucion y ataque al jugador.
```

creates a plan that:

1. Creates `Enemy2D`.
2. Adds gameplay, physics, AI, navigation, scripting and lighting components.
3. Generates `scripts/enemy_controller.luau` with exported Inspector variables.
4. Saves `Enemy2D` as `assets/prefabs/Enemy2D.prefab`.
5. Runs project validation and `forge_ai_enemy_smoke`.

## Safety

Default permissions are `EditWithApproval`. ForgeAI can always produce previews,
but write operations require approval unless running in an explicit autonomous
sandbox. Paths are project-relative and checked before file writes.
