#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import * as z from "zod/v4";
import { execFile } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ENGINE_ROOT = path.resolve(process.env.MINIFORGE_ENGINE_ROOT || path.join(__dirname, "../../.."));
const PROJECTS_DIR = path.join(ENGINE_ROOT, "projects");
const FEEDBACK_PATH = path.join(ENGINE_ROOT, "docs", "MINIFORGE_MCP_FEEDBACK.md");
const SERVER_VERSION = "0.2.0";

function textResult(text, structuredContent) {
  return {
    content: [{ type: "text", text: typeof text === "string" ? text : JSON.stringify(text, null, 2) }],
    ...(structuredContent ? { structuredContent } : {})
  };
}

function readText(filePath) {
  return readFileSync(filePath, "utf8");
}

function writeText(filePath, text) {
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, text, "utf8");
}

function writeJson(filePath, data) {
  writeText(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function writeBase64(filePath, data) {
  mkdirSync(path.dirname(filePath), { recursive: true });
  writeFileSync(filePath, Buffer.from(data, "base64"));
}

function rel(filePath) {
  return path.relative(ENGINE_ROOT, filePath) || ".";
}

function matchConst(source, name) {
  const match = source.match(new RegExp(`pub\\s+const\\s+${name}\\s*:\\s*&str\\s*=\\s*"([^"]+)"`));
  return match?.[1] || null;
}

function readEngineInfo() {
  const cargoToml = readText(path.join(ENGINE_ROOT, "Cargo.toml"));
  const versionRs = readText(path.join(ENGINE_ROOT, "src", "engine", "version.rs"));
  const cargoVersion = cargoToml.match(/version\s*=\s*"([^"]+)"/)?.[1] || "unknown";
  const packageName = cargoToml.match(/name\s*=\s*"([^"]+)"/)?.[1] || "miniforge";
  const engineVersion = matchConst(versionRs, "ENGINE_VERSION") || cargoVersion;
  const streamVersion = matchConst(versionRs, "ENGINE_STREAM_VERSION") || cargoVersion;
  const codename = matchConst(versionRs, "ENGINE_CODENAME") || "unknown";
  return {
    engineRoot: ENGINE_ROOT,
    packageName,
    cargoVersion,
    engineVersion,
    engineStreamVersion: streamVersion,
    codename,
    versionLabel: `MiniForge ${engineVersion} - ${codename}`,
    recommendedEditorCommand: `cargo run --bin miniforge_editor -- --project projects/MCP_AstroHarvester --no-launcher`,
    recommendedHeadlessCommand: `cargo run --bin miniforge -- --project projects/MCP_AstroHarvester --runtime --no-launcher --headless-once`
  };
}

function safeProjectName(name) {
  const cleaned = String(name || "MCP_AstroHarvester")
    .trim()
    .replace(/[^a-z0-9 _.-]/gi, "_")
    .replace(/\s+/g, "_")
    .slice(0, 72);
  return cleaned || "MCP_AstroHarvester";
}

function projectPaths(projectPath) {
  return {
    root: projectPath,
    assets: path.join(projectPath, "assets"),
    data: path.join(projectPath, "assets", "data"),
    prefabs: path.join(projectPath, "assets", "prefabs"),
    sprites: path.join(projectPath, "assets", "sprites"),
    audio: path.join(projectPath, "assets", "audio"),
    scripts: path.join(projectPath, "scripts"),
    graphs: path.join(projectPath, "scripts", "visual_graphs"),
    scenes: path.join(projectPath, "saves", "scenes"),
    settings: path.join(projectPath, "settings"),
    logs: path.join(projectPath, "logs"),
    systems: path.join(projectPath, "systems"),
    components: path.join(projectPath, "components"),
    templates: path.join(projectPath, "templates"),
    builds: path.join(projectPath, "builds")
  };
}

function ensureProjectFolders(paths) {
  for (const folder of Object.values(paths)) {
    mkdirSync(folder, { recursive: true });
  }
}

function component(componentType, data = {}) {
  return { component_type: componentType, enabled: true, ...data };
}

function baseComponents(x, y, tint = [255, 255, 255]) {
  return [
    component("Transform", { x, y, rotation: 0.0, scale_x: 1.0, scale_y: 1.0 }),
    component("Selectable", { selectable: true, selection_radius: 0.55 }),
    component("SpriteRenderer", { tint, visible: true, sorting_order: 0 }),
    component("Collider2D", { shape: "rect", width: 1.0, height: 1.0, radius: 0.5, is_trigger: false })
  ];
}

function entity({
  id,
  name,
  entityType = "GameObject",
  x = 0,
  y = 0,
  width = 1,
  height = 1,
  radius = 0.5,
  speed = 3.5,
  tag = "Untagged",
  layer = "Default",
  script = null,
  visible = true,
  locked = false,
  tint = [255, 255, 255],
  extraComponents = [],
  scripts = []
}) {
  const components = baseComponents(x, y, tint).concat(extraComponents);
  return {
    type: entityType,
    id,
    name,
    enabled: true,
    active: true,
    visible,
    locked,
    x,
    y,
    position: [x, y],
    rotation: 0.0,
    scale: [1.0, 1.0],
    scale_x: 1.0,
    scale_y: 1.0,
    size: [width, height],
    width,
    height,
    speed,
    radius,
    sprite_name: null,
    sprite_guid: null,
    script,
    tag,
    layer,
    state: "IDLE",
    command: "IDLE",
    path: [],
    parent_id: null,
    local_x: 0.0,
    local_y: 0.0,
    prefab_source: null,
    prefab_guid: null,
    is_prefab_instance: false,
    scene_name: null,
    patrol_points: [],
    patrol_index: 0,
    follow_target_id: null,
    guard_target_id: null,
    attack_move_target: null,
    gather_target_id: null,
    components,
    scripts
  };
}

function unit(options) {
  return entity({
    entityType: "Unit",
    speed: 4.5,
    extraComponents: [
      component("RTSMovement", { speed: 4.5, separation: true, allow_pathfinding: true }),
      ...(options.extraComponents || [])
    ],
    ...options
  });
}

function uiLabel(id, name, text, x, y, width = 360) {
  return entity({
    id,
    name,
    x: 0,
    y: 0,
    visible: true,
    locked: true,
    tag: "Untagged",
    layer: "UI",
    extraComponents: [
      component("UIElement", {
        element_type: "Label",
        text,
        x,
        y,
        width,
        height: 34,
        color: [20, 24, 32],
        text_color: [235, 244, 255],
        opacity: 0.95,
        text_align: "left",
        padding: 8,
        border_radius: 7
      })
    ]
  });
}

function makeAstroScene(engineVersion) {
  const entities = [
    entity({
      id: 1,
      name: "GameRules",
      x: 0,
      y: 0,
      visible: false,
      locked: true,
      layer: "EditorOnly",
      extraComponents: [
        component("GameMode2D", { default_pawn: "Pilot", start_scene: "saves/scenes/main.scene" }),
        component("GameState2D", { phase: "collect_crystals", score: { crystals: 0 } }),
        component("RuntimeBudget2D", { max_entities: 5000, max_visible_sprites: 2000 })
      ]
    }),
    unit({
      id: 10,
      name: "Pilot",
      x: 6,
      y: 7,
      tag: "Player",
      layer: "Units",
      script: "PilotController.rhai",
      tint: [90, 190, 255],
      extraComponents: [
        component("Health", { max_health: 120.0, health: 120.0 }),
        component("Stats", { attack: 14.0, agility: 8.0, defense: 2.0 }),
        component("Inventory", { capacity: 16, currency: { Crystal: 0 } }),
        component("Equipment"),
        component("Ability", {
          ability_id: "pulse_dash",
          display_name: "Pulse Dash",
          cooldown: 1.5,
          range: 4.0,
          power: 12.0
        }),
        component("CharacterController2D", { mode: "topdown", walk_speed: 5.5, run_speed: 7.0 }),
        component("NavAgent", { speed: 4.5 }),
        component("Commandable", { can_gather: true, can_build: false }),
        component("Vision", { radius: 8.0 }),
        component("Saveable", { save_key: "pilot" }),
        component("ParticleEmitter", { looped: true, rate: 10.0, burst_count: 4 }),
        component("Material2D", { material: "PilotGlow", shader: "sprite_lit_fog", emission: [35, 95, 180] }),
        component("CameraFollow", { smoothness: 7.0, zoom: 1.1 })
      ]
    }),
    entity({
      id: 20,
      name: "ForgeBase",
      x: 4,
      y: 5,
      width: 2.4,
      height: 2.0,
      radius: 1.2,
      tag: "Building",
      layer: "Buildings",
      tint: [130, 210, 255],
      extraComponents: [
        component("Health", { max_health: 450.0, health: 450.0 }),
        component("Team", { team_id: 1, team_name: "Player", color: [80, 160, 255] }),
        component("EconomyWallet", { resources: { Crystal: 75, Gold: 250, Wood: 120 } }),
        component("ProductionRecipeBook", {
          preferred_recipe: "Drone",
          recipes: [
            { unit_type: "Drone", display_name: "Scout Drone", build_time: 3.0, cost: { Crystal: 25 } },
            { unit_type: "Turret", display_name: "Light Turret", build_time: 6.0, cost: { Crystal: 55, Gold: 40 } }
          ]
        }),
        component("ProductionQueue", { rally_x: 7.0, rally_y: 5.0, max_queue: 5 }),
        component("Buildable", { display_name: "Forge Base", footprint_w: 3, footprint_h: 2 }),
        component("Commandable", { can_move: false, can_produce: true }),
        component("Vision", { radius: 10.0 })
      ]
    }),
    entity({
      id: 30,
      name: "CrystalNode_A",
      x: 10,
      y: 6,
      tag: "Resource",
      layer: "Ground",
      tint: [80, 255, 210],
      extraComponents: [
        component("ResourceNode", { resource_type: "Crystal", amount: 500.0, max_amount: 500.0, gather_rate: 14.0 }),
        component("ObjectiveMarker", { label: "Crystal", color: [90, 255, 220] }),
        component("Light2D", { color: [90, 255, 220], radius: 5.0, intensity: 1.35, flicker: true })
      ]
    }),
    entity({
      id: 31,
      name: "CrystalNode_B",
      x: 14,
      y: 10,
      tag: "Resource",
      layer: "Ground",
      tint: [120, 255, 180],
      extraComponents: [
        component("ResourceNode", { resource_type: "Crystal", amount: 650.0, max_amount: 650.0, gather_rate: 10.0 }),
        component("ObjectiveMarker", { label: "Rich Crystal", color: [120, 255, 180] }),
        component("Light2D", { color: [120, 255, 180], radius: 6.0, intensity: 1.15, flicker: true })
      ]
    }),
    unit({
      id: 40,
      name: "EnemyDrone_A",
      x: 17,
      y: 8,
      tag: "Enemy",
      layer: "Units",
      script: "DronePatrol.rhai",
      tint: [255, 92, 110],
      extraComponents: [
        component("Health", { max_health: 65.0, health: 65.0 }),
        component("Team", { team_id: 2, team_name: "Enemy", color: [255, 95, 95] }),
        component("Stats", { attack: 8.0, agility: 6.0 }),
        component("AIController", { behavior: "patrol", home_x: 17.0, home_y: 8.0, detection_radius: 6.0 }),
        component("DamageDealer", { damage: 8.0, target_tags: ["Player"] }),
        component("CombatTarget", { target_tags: ["Player"], aggro_radius: 7.0 }),
        component("ThreatSource", { strength: 6.0, radius: 4.0 }),
        component("Vision", { radius: 7.0 })
      ]
    }),
    unit({
      id: 41,
      name: "EnemyDrone_B",
      x: 20,
      y: 12,
      tag: "Enemy",
      layer: "Units",
      script: "DronePatrol.rhai",
      tint: [255, 120, 92],
      extraComponents: [
        component("Health", { max_health: 80.0, health: 80.0 }),
        component("Team", { team_id: 2, team_name: "Enemy", color: [255, 95, 95] }),
        component("Stats", { attack: 10.0, defense: 1.0 }),
        component("AIController", { behavior: "guard", home_x: 20.0, home_y: 12.0, detection_radius: 7.5 }),
        component("DamageDealer", { damage: 10.0, target_tags: ["Player"] }),
        component("CombatTarget", { target_tags: ["Player"], aggro_radius: 8.0 }),
        component("ThreatSource", { strength: 8.0, radius: 5.0 }),
        component("Vision", { radius: 7.0 })
      ]
    }),
    entity({
      id: 50,
      name: "NorthGateCheckpoint",
      x: 22,
      y: 5,
      tag: "Neutral",
      layer: "Ground",
      tint: [255, 222, 120],
      extraComponents: [
        component("Checkpoint", {
          checkpoint_id: "north_gate",
          respawn_x: 6.0,
          respawn_y: 7.0,
          activation_radius: 1.5
        }),
        component("ObjectiveMarker", { label: "Gate", color: [255, 222, 120] })
      ]
    }),
    uiLabel(100, "HUD_Status", "Astro Harvester: WASD mueve, Space emite pulso, recolecta cristales.", 24, 24, 560),
    uiLabel(101, "HUD_Objective", "Objetivo: explora los nodos de cristal y defiende ForgeBase.", 24, 64, 520)
  ];

  return {
    version: engineVersion,
    engine_version: engineVersion,
    scene_name: "main",
    mode: "EDITOR",
    active_tool: "Select",
    tile_brush: 0,
    brush_size: 1,
    camera: { x: 120.0, y: 110.0, zoom: 1.08 },
    control_groups: {},
    grid: { width: 60, height: 40, tile_size: 32, chunk_size: 8 },
    tiles: [],
    tilemap_layers: [],
    settings: {
      genre: "topdown_rts_hybrid",
      win_condition: "Collect 250 Crystal and keep ForgeBase alive.",
      generated_by: "miniforge-mcp"
    },
    entities,
    editor_view_settings: {},
    ui_canvases: []
  };
}

function makeInputMap() {
  return {
    actions: {
      Move: { display_name: "Move", category: "Gameplay", devices: ["keyboard", "gamepad"], description: "Directional movement vector." },
      Attack: { display_name: "Attack", category: "Gameplay", devices: ["mouse", "keyboard"], description: "Primary attack or pulse." },
      Interact: { display_name: "Interact", category: "Gameplay", devices: ["keyboard"], description: "Harvest, inspect or activate." },
      Pause: { display_name: "Pause", category: "System", devices: ["keyboard"], description: "Pause gameplay." },
      Command: { display_name: "Command", category: "RTS", devices: ["mouse"], description: "Context command." },
      Select: { display_name: "Select", category: "RTS", devices: ["mouse"], description: "Select units or confirm UI." }
    },
    bindings: {
      Move: ["keyboard:wasd", "keyboard:arrows", "gamepad:left_stick"],
      Attack: ["mouse_left", "space"],
      Interact: ["e"],
      Pause: ["escape"],
      Command: ["mouse_right"],
      Select: ["mouse_left"],
      move_left: ["a", "left"],
      move_right: ["d", "right"],
      move_up: ["w", "up"],
      move_down: ["s", "down"],
      attack: ["space", "mouse1"],
      interact: ["e"],
      pause: ["escape"],
      play: ["f5"],
      save: ["s"]
    }
  };
}

function makeManifest(engineVersion) {
  return {
    engine_version: engineVersion,
    runtime: "rust",
    scenes: ["saves/scenes/main.scene"],
    scripts: [
      "scripts/PilotController.rhai",
      "scripts/DronePatrol.rhai",
      "scripts/visual_graphs/AstroHarvestLoop.mfgraph"
    ],
    assets: [
      "assets/data/AstroBalance.json",
      "assets/data/GameDesignNotes.json",
      "assets/prefabs/Pilot.prefab",
      "assets/prefabs/EnemyDrone.prefab"
    ],
    components: [],
    systems: []
  };
}

function createPrefab(name, tint, tag) {
  return {
    version: readEngineInfo().engineVersion,
    engine_version: readEngineInfo().engineVersion,
    prefab_name: name,
    entity: unit({
      id: 9000,
      name,
      x: 0,
      y: 0,
      tag,
      layer: "Units",
      tint,
      extraComponents: [component("Health"), component("Stats"), component("NavAgent")]
    })
  };
}

function createAstroHarvester({ name = "MCP_AstroHarvester", overwrite = false, projectDirectory } = {}) {
  const projectName = safeProjectName(name);
  const projectPath = path.resolve(projectDirectory || path.join(PROJECTS_DIR, projectName));
  if (existsSync(projectPath)) {
    if (!overwrite) {
      throw new Error(`Project already exists: ${projectPath}. Pass overwrite=true to recreate it.`);
    }
    rmSync(projectPath, { recursive: true, force: true });
  }

  const info = readEngineInfo();
  const paths = projectPaths(projectPath);
  ensureProjectFolders(paths);

  writeJson(path.join(projectPath, "project.json"), {
    project_name: projectName,
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    start_scene: "main.scene",
    current_scene: "main.scene",
    author: "MiniForge MCP",
    description: "Top-down RTS hybrid demo generated by the MiniForge MCP."
  });
  writeJson(path.join(projectPath, "engine_config.json"), {
    engine_name: "MiniForge",
    engine_alt_name: "MiniForge",
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    project_name: projectName,
    start_scene: "main.scene",
    autosave: true,
    autosave_interval_seconds: 60,
    safe_mode: true,
    config_version: 2,
    editor: { script_hot_reload: true, open_created_assets: true, fallback_assets: true },
    rendering: {
      backend: "macroquad",
      pixel_perfect: true,
      sprite_batching: true,
      tilemap_chunk_batching: true,
      post_processing: true,
      enable_3d: false,
      vsync: true
    },
    logs: {
      level: "info",
      file: "logs/miniforge.log",
      engine: "logs/engine.log",
      error: "logs/error.log"
    }
  });
  writeJson(path.join(projectPath, "manifest.json"), makeManifest(info.engineVersion));
  writeJson(path.join(paths.settings, "runtime_config.json"), {
    game_name: "Astro Harvester",
    start_scene: "main.scene",
    window_width: 1280,
    window_height: 760,
    fullscreen: false,
    target_fps: 60,
    debug: true,
    quality_preset: "balanced",
    performance_class: "auto",
    worker_threads: "auto",
    parallel_asset_scan: true,
    prefer_metal_on_macos: true
  });
  writeJson(path.join(paths.settings, "build_settings.json"), {
    game_name: "Astro Harvester",
    start_scene: "main.scene",
    target_fps: 60,
    window_width: 1280,
    window_height: 760,
    fullscreen: false,
    debug_mode: true,
    export_folder: "builds"
  });
  writeJson(path.join(paths.settings, "build_profiles.json"), {
    active: "Development",
    profiles: {
      Development: { debug_mode: true, target_fps: 60 },
      Release: { debug_mode: false, target_fps: 60 },
      Shipping: { debug_mode: false, target_fps: 60, strip_debug: true }
    }
  });
  writeJson(path.join(paths.settings, "input_map.json"), makeInputMap());
  writeJson(path.join(paths.settings, "tags.json"), {
    items: ["Untagged", "Player", "Enemy", "Resource", "Building", "Neutral", "Projectile"]
  });
  writeJson(path.join(paths.settings, "layers.json"), {
    items: ["Default", "Ground", "Units", "Buildings", "UI", "Effects", "IgnoreSelection", "EditorOnly"]
  });
  writeJson(path.join(paths.scenes, "main.scene"), makeAstroScene(info.engineVersion));
  writeText(path.join(paths.scripts, "PilotController.rhai"), `fn on_start() {
    set_ui_text("HUD_Status", "Pilot online. WASD mueve, Space dispara pulso.");
}

fn on_update(dt) {
    if input_pressed("A") { move(-5.0 * dt, 0.0); }
    if input_pressed("D") { move(5.0 * dt, 0.0); }
    if input_pressed("W") { move(0.0, -5.0 * dt); }
    if input_pressed("S") { move(0.0, 5.0 * dt); }
    if input_pressed("Space") {
        set_ui_text("HUD_Status", "Pulso de defensa emitido. Revisa enemigos cercanos.");
    }
}

fn on_collision_enter(other) {
    set_ui_text("HUD_Status", "Contacto detectado: " + other);
}
`);
  writeText(path.join(paths.scripts, "DronePatrol.rhai"), `fn on_start() {
    ui_text("Drone activo");
}

fn on_update(dt) {
    move(-0.35 * dt, 0.0);
}
`);
  writeJson(path.join(paths.graphs, "AstroHarvestLoop.mfgraph"), {
    version: info.engineVersion,
    kind: "MiniForgeVisualGraph",
    runtime: "rust_visual_graph",
    name: "AstroHarvestLoop",
    variables: {
      crystals_required: 250,
      base_health_required: 1
    },
    nodes: [
      { id: "start", type: "EventStart", next: "log" },
      { id: "update", type: "EventUpdate", next: "branch" },
      { id: "log", type: "Log", message: "Astro Harvester graph ready", next: null },
      { id: "branch", type: "BranchVariable", variable: "crystals_required", operator: ">", value: 0, true_next: "objective", false_next: "win" },
      { id: "objective", type: "Log", message: "Keep harvesting crystals", next: null },
      { id: "win", type: "Log", message: "Forge stabilized", next: null }
    ]
  });
  writeJson(path.join(paths.data, "AstroBalance.json"), {
    resources: { Crystal: { target: 250, node_respawn_seconds: 90 } },
    player: { max_health: 120, pulse_damage: 12, dash_cooldown: 1.5 },
    enemies: { drone_health: 65, drone_damage: 8, detection_radius: 7.0 },
    production: {
      Drone: { build_time: 3.0, cost: { Crystal: 25 } },
      Turret: { build_time: 6.0, cost: { Crystal: 55, Gold: 40 } }
    }
  });
  writeJson(path.join(paths.data, "GameDesignNotes.json"), {
    title: "Astro Harvester",
    loop: ["Explore crystal nodes", "Avoid drones", "Return resources to ForgeBase", "Queue defensive units"],
    uses_engine_features: ["Rhai scripting", "UIElement HUD", "ResourceNode", "ProductionQueue", "CharacterController2D", "AIController", "Visual graph"]
  });
  writeJson(path.join(paths.prefabs, "Pilot.prefab"), createPrefab("Pilot", [90, 190, 255], "Player"));
  writeJson(path.join(paths.prefabs, "EnemyDrone.prefab"), createPrefab("EnemyDrone", [255, 92, 110], "Enemy"));
  writeText(path.join(projectPath, "README.md"), `# Astro Harvester

Juego demo generado por el MCP de MiniForge usando MiniForge ${info.engineVersion} / stream ${info.engineStreamVersion}.

## Ejecutar editor

\`\`\`bash
cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher
\`\`\`

## Validar sin ventana

\`\`\`bash
cargo run --bin miniforge -- --project ${rel(projectPath)} --runtime --no-launcher --headless-once
\`\`\`

## Controles

- WASD: mover piloto.
- Space: pulso defensivo.
- E: interactuar cuando el flujo del motor lo conecte.

## Que prueba

- Escena jugable con HUD, scripts Rhai y componentes avanzados.
- Loop hibrido top-down/RTS con recursos, base, drones enemigos y produccion.
- Datos de balance en \`assets/data/AstroBalance.json\`.
`);

  return {
    projectName,
    projectPath,
    engineVersion: info.engineVersion,
    engineStreamVersion: info.engineStreamVersion,
    createdFiles: [
      "project.json",
      "engine_config.json",
      "manifest.json",
      "saves/scenes/main.scene",
      "scripts/PilotController.rhai",
      "scripts/DronePatrol.rhai",
      "scripts/visual_graphs/AstroHarvestLoop.mfgraph",
      "assets/data/AstroBalance.json",
      "assets/data/GameDesignNotes.json",
      "assets/prefabs/Pilot.prefab",
      "assets/prefabs/EnemyDrone.prefab",
      "README.md"
    ].map((item) => path.join(projectPath, item)),
    openCommand: `cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher`,
    validateCommand: `cargo run --bin miniforge -- --project ${rel(projectPath)} --runtime --no-launcher --headless-once`
  };
}

function makeStoryTilemapLayers(width = 54, height = 32) {
  const makeLayer = (name, fill = 0, visible = true, locked = false) => ({
    name,
    visible,
    locked,
    tiles: Array.from({ length: height }, () => Array.from({ length: width }, () => fill))
  });
  const layers = [
    makeLayer("Ground", 1),
    makeLayer("Decoration", 0),
    makeLayer("Collision", 0),
    makeLayer("Overlay", 0)
  ];

  for (let x = 0; x < width; x += 1) {
    layers[2].tiles[0][x] = 4;
    layers[2].tiles[height - 1][x] = 4;
  }
  for (let y = 0; y < height; y += 1) {
    layers[2].tiles[y][0] = 4;
    layers[2].tiles[y][width - 1] = 4;
  }
  for (let x = 8; x < width - 8; x += 1) {
    layers[1].tiles[14][x] = 2;
    layers[1].tiles[15][x] = 2;
  }
  for (let y = 4; y < height - 4; y += 4) {
    layers[1].tiles[y][12] = 3;
    layers[1].tiles[y][38] = 3;
  }
  for (let x = 18; x < 34; x += 1) {
    layers[3].tiles[10][x] = 5;
  }
  return { width, height, active_layer: 0, layers };
}

function makeDialogueLines(speaker, lines, choices = []) {
  return component("Dialogue", {
    speaker,
    lines,
    index: 0,
    is_active: false,
    auto_advance: false,
    choices,
    on_complete_graph: "scripts/visual_graphs/LoveStoryActLoop.mfgraph"
  });
}

function storyUiLabel(id, name, text, x, y, width = 620, height = 38) {
  return entity({
    id,
    name,
    x: 0,
    y: 0,
    visible: true,
    locked: true,
    tag: "UI",
    layer: "UI",
    extraComponents: [
      component("UIElement", {
        element_type: "Label",
        text,
        x,
        y,
        width,
        height,
        color: [24, 28, 36],
        text_color: [246, 240, 231],
        opacity: 0.94,
        text_align: "left",
        padding: 9,
        border_radius: 6,
        border_color: [120, 144, 170],
        sorting_order: 20
      })
    ]
  });
}

function makeLoveStoryScene(engineVersion) {
  const storyGraph = [
    { id: "start", type: "EventStart", next: "add_quest" },
    {
      id: "add_quest",
      type: "AddQuest",
      quest: "letters_under_rain",
      title: "Letters Under Rain",
      objectives: [
        { id: "meet", text: "Meet Mara at the station", progress: 0, target: 1 },
        { id: "letter", text: "Find the unsent letter", progress: 0, target: 1 },
        { id: "choice", text: "Choose what Sol says at the bridge", progress: 0, target: 1 }
      ],
      next: "state"
    },
    { id: "state", type: "SetState", state: "Act1Station", next: null },
    { id: "update", type: "EventUpdate", next: null },
    { id: "pulse", type: "TriggerAbility", true_next: "progress", false_next: "recharge" },
    {
      id: "progress",
      type: "QuestProgress",
      quest: "letters_under_rain",
      objective: "meet",
      progress: 1,
      next: "complete"
    },
    { id: "complete", type: "CompleteQuest", quest: "letters_under_rain", next: null },
    { id: "recharge", type: "RechargeAbility", amount: 1, next: null }
  ];

  const entities = [
    entity({
      id: 1,
      name: "StoryDirector",
      x: 0,
      y: 0,
      visible: false,
      locked: true,
      layer: "EditorOnly",
      script: "StoryDirector.luau",
      extraComponents: [
        component("GameMode2D", {
          default_pawn: "Sol",
          start_scene: "saves/scenes/main.scene",
          rules: { genre: "interactive_story_lab", acts: 3, ending_count: 3 }
        }),
        component("GameState2D", {
          phase: "act_1_station",
          score: { affection: 35, courage: 20, honesty: 40, memory_fragments: 0 }
        }),
        component("Blackboard", {
          values: {
            current_act: 1,
            current_beat: "station_reunion",
            affection: 35,
            last_choice: "none",
            debug_lab_mode: true
          }
        }),
        component("RuntimeBudget2D", { max_entities: 8000, max_visible_sprites: 2400, max_script_ms: 5.0 }),
        component("Sequencer2D", {
          sequence: "assets/sequences/IntroStation.seq2d.json",
          playing: false,
          time: 0.0,
          duration: 18.0,
          loop: false,
          auto_play: true
        }),
        component("VisualScript", {
          graph_name: "LoveStoryActLoop",
          run_in_editor: true,
          variables: { affection: 35, act: 1 },
          nodes: storyGraph,
          enabled_events: ["start", "update", "trigger"]
        })
      ]
    }),
    entity({
      id: 10,
      name: "Sol",
      entityType: "Unit",
      x: 8,
      y: 14,
      tag: "Player",
      layer: "Characters",
      script: "SolController.luau",
      tint: [92, 174, 230],
      extraComponents: [
        component("Rigidbody2D", { body_type: "dynamic", use_gravity: false, drag: 0.2, mass: 1.0 }),
        component("CharacterController2D", { mode: "topdown", walk_speed: 4.2, run_speed: 5.8, dash_speed: 10.0 }),
        component("Pawn2D", { auto_possess: true, movement_mode: "topdown", input_enabled: true, camera_follow: true }),
        component("PlayerController2D", { input_context: "settings/input_map.json", cursor_visible: true, click_to_move: false }),
        component("CameraFollow", { target_id: 10, smoothness: 7.5, zoom: 1.08, dead_zone: 0.15 }),
        component("QuestLog", {
          active_quest_id: "letters_under_rain",
          completed_count: 0,
          quests: [
            {
              id: "letters_under_rain",
              title: "Letters Under Rain",
              state: "active",
              objectives: [
                { id: "meet", text: "Meet Mara at the station", progress: 0, target: 1 },
                { id: "letter", text: "Find the unsent letter", progress: 0, target: 1 },
                { id: "choice", text: "Choose what Sol says at the bridge", progress: 0, target: 1 }
              ]
            }
          ]
        }),
        component("Inventory", {
          capacity: 8,
          items: [
            { id: "old_ticket", quantity: 1, stackable: false, metadata: { label: "Old station ticket" } }
          ]
        }),
        component("Ability", {
          ability_id: "remember",
          display_name: "Remember",
          cooldown: 2.0,
          charges: 1,
          current_charges: 1,
          power: 1.0
        }),
        component("Checkpoint", { checkpoint_id: "station_start", active: true, respawn_x: 8.0, respawn_y: 14.0 }),
        component("Saveable", { save_key: "sol", include_components: true, persistent: true, autosave: true }),
        component("StateMachine", {
          initial_state: "Walking",
          current_state: "Walking",
          states: ["Walking", "Listening", "Choosing", "Remembering"],
          transitions: []
        }),
        component("Light2D", { color: [118, 180, 240], radius: 5.5, intensity: 0.85, flicker: false }),
        component("Material2D", { material: "SolRaincoat", shader: "sprite_lit_fog", emission: [12, 38, 70] })
      ]
    }),
    entity({
      id: 11,
      name: "Mara",
      x: 19,
      y: 14,
      tag: "LoveInterest",
      layer: "Characters",
      script: "DialogueProbe.luau",
      tint: [238, 120, 168],
      extraComponents: [
        component("Interaction", {
          prompt: "Talk to Mara",
          radius: 1.8,
          action_name: "interact",
          action_graph: "scripts/visual_graphs/LoveStoryActLoop.mfgraph",
          quest_id: "letters_under_rain",
          objective_id: "meet",
          requires_tag: "Player"
        }),
        makeDialogueLines("Mara", [
          "You still keep arriving before the rain stops.",
          "I found your letter, Sol. I just never knew if it was meant to be sent.",
          "Tonight we can stop rehearsing goodbye."
        ], [
          { id: "honest", text: "Tell her the truth", affection_delta: 15, courage_delta: 5 },
          { id: "gentle", text: "Ask about her dream first", affection_delta: 8, honesty_delta: 8 }
        ]),
        component("ObjectiveMarker", { label: "Mara", color: [238, 120, 168], pulse: true }),
        component("Saveable", { save_key: "mara", include_components: true, persistent: true }),
        component("StateMachine", {
          initial_state: "Waiting",
          current_state: "Waiting",
          states: ["Waiting", "Talking", "Hurt", "Hopeful"],
          transitions: []
        }),
        component("Light2D", { color: [255, 140, 190], radius: 5.0, intensity: 0.95, flicker: true })
      ]
    }),
    entity({
      id: 12,
      name: "Tomas",
      x: 31,
      y: 18,
      tag: "Friend",
      layer: "Characters",
      script: "DialogueProbe.luau",
      tint: [255, 196, 104],
      extraComponents: [
        component("Interaction", { prompt: "Ask Tomas", radius: 1.6, action_name: "interact", requires_tag: "Player" }),
        makeDialogueLines("Tomas", [
          "I kept the cafe open because both of you always return to places with bad coffee.",
          "The letter is on the bench. Read it before you speak."
        ]),
        component("ObjectiveMarker", { label: "Tomas", color: [255, 196, 104], pulse: false })
      ]
    }),
    entity({
      id: 20,
      name: "UnsentLetter",
      x: 25,
      y: 12,
      tag: "Memory",
      layer: "Props",
      tint: [246, 226, 180],
      extraComponents: [
        component("Interaction", {
          prompt: "Read letter",
          radius: 1.25,
          action_name: "interact",
          action_graph: "scripts/visual_graphs/LoveStoryActLoop.mfgraph",
          quest_id: "letters_under_rain",
          objective_id: "letter",
          single_use: true
        }),
        component("Inventory", { capacity: 1, items: [{ id: "unsent_letter", quantity: 1, stackable: false }] }),
        component("ObjectiveMarker", { label: "Letter", color: [246, 226, 180], pulse: true }),
        component("Saveable", { save_key: "unsent_letter", include_components: true, persistent: true })
      ]
    }),
    entity({
      id: 21,
      name: "SharedUmbrella",
      x: 16,
      y: 21,
      tag: "Memory",
      layer: "Props",
      tint: [132, 216, 204],
      extraComponents: [
        component("Interaction", { prompt: "Take umbrella", radius: 1.2, action_name: "interact", single_use: true }),
        component("Ability", { ability_id: "shelter", display_name: "Shelter", cooldown: 4.0, charges: 1, current_charges: 1 }),
        component("ObjectiveMarker", { label: "Umbrella", color: [132, 216, 204], pulse: false })
      ]
    }),
    entity({
      id: 30,
      name: "StationMemoryTrigger",
      x: 14,
      y: 14,
      width: 4,
      height: 4,
      tag: "Trigger",
      layer: "Triggers",
      visible: false,
      tint: [130, 190, 255],
      extraComponents: [
        component("Trigger2D", {
          shape: "rect",
          width: 4.0,
          height: 4.0,
          layer: "Trigger",
          overlap_mask: ["Player"],
          on_enter_graph: "scripts/visual_graphs/LoveStoryActLoop.mfgraph"
        }),
        component("Dialogue", {
          speaker: "Memory",
          lines: ["The station clock stopped at the minute she left."],
          index: 0,
          is_active: false
        })
      ]
    }),
    entity({
      id: 31,
      name: "BridgeChoiceTrigger",
      x: 39,
      y: 14,
      width: 5,
      height: 5,
      tag: "Trigger",
      layer: "Triggers",
      visible: false,
      tint: [240, 140, 210],
      extraComponents: [
        component("Trigger2D", {
          shape: "rect",
          width: 5.0,
          height: 5.0,
          layer: "Trigger",
          overlap_mask: ["Player"],
          on_enter_graph: "scripts/visual_graphs/LoveStoryActLoop.mfgraph"
        }),
        component("Checkpoint", { checkpoint_id: "bridge_choice", respawn_x: 38.0, respawn_y: 14.0, activation_radius: 2.4 })
      ]
    }),
    entity({
      id: 40,
      name: "RainAmbience",
      x: 0,
      y: 0,
      visible: false,
      locked: true,
      tag: "Audio",
      layer: "Audio",
      extraComponents: [
        component("AudioSource", {
          audio_name: "RainTheme",
          volume: 0.72,
          pitch: 1.0,
          bus: "Music",
          play_on_start: true,
          loop: true,
          spatial_blend: 0.0
        })
      ]
    }),
    entity({
      id: 41,
      name: "RainParticles",
      x: 24,
      y: 12,
      width: 12,
      height: 8,
      tag: "Effects",
      layer: "Effects",
      tint: [140, 170, 220],
      extraComponents: [
        component("ParticleEmitter", {
          looped: true,
          rate: 42.0,
          burst_count: 8,
          lifetime: 1.4,
          velocity: [0.0, 1.5],
          spread: 0.35
        }),
        component("ParallaxLayer", { factor_x: 0.35, factor_y: 0.15, repeat_x: true, repeat_y: true, sorting_order: -5 })
      ]
    }),
    storyUiLabel(100, "HUD_Title", "Letters Under Rain - Narrative 2D Lab", 24, 22, 520),
    storyUiLabel(101, "HUD_Status", "WASD moves Sol. E talks, Space remembers. Test dialogue, quests, save, audio, particles and triggers.", 24, 64, 830),
    storyUiLabel(102, "HUD_Dialogue", "Mara waits under the station awning.", 24, 606, 820, 52),
    storyUiLabel(103, "HUD_ChoiceA", "[1] Tell the truth", 24, 666, 300, 36),
    storyUiLabel(104, "HUD_ChoiceB", "[2] Ask about her dream", 340, 666, 330, 36),
    entity({
      id: 105,
      name: "HUD_Affection",
      x: 0,
      y: 0,
      visible: true,
      locked: true,
      tag: "UI",
      layer: "UI",
      extraComponents: [
        component("UIElement", {
          element_type: "ProgressBar",
          text: "Affection",
          x: 930,
          y: 24,
          width: 260,
          height: 26,
          color: [42, 48, 58],
          text_color: [246, 240, 231],
          progress: 0.35,
          max_progress: 1.0,
          opacity: 0.95,
          sorting_order: 20
        })
      ]
    })
  ];

  return {
    format: "miniforge.scene",
    schema_version: 1,
    version: engineVersion,
    engine_version: engineVersion,
    scene_name: "main",
    mode: "EDITOR",
    active_tool: "Select",
    tile_brush: 0,
    brush_size: 1,
    camera: { x: 250.0, y: 214.0, zoom: 1.02 },
    control_groups: {},
    grid: { width: 54, height: 32, tile_size: 32, chunk_size: 8 },
    tiles: [],
    tilemap_layers: makeStoryTilemapLayers(),
    settings: {
      genre: "interactive_love_story_lab",
      story: "Two people meet again during one rainy night and choose whether honesty can become a future.",
      lab_focus: [
        "2D topdown movement",
        "Dialogue",
        "QuestLog",
        "VisualScript",
        "UIElement",
        "AudioSource",
        "ParticleEmitter",
        "Trigger2D",
        "Checkpoint",
        "Saveable",
        "Sequencer2D",
        "TilemapLayers"
      ],
      generated_by: "miniforge-mcp"
    },
    entities,
    editor_view_settings: {},
    ui_canvases: []
  };
}

function makeLoveStoryManifest(engineVersion) {
  return {
    engine_version: engineVersion,
    runtime: "rust",
    scenes: ["saves/scenes/main.scene"],
    scripts: [
      "scripts/StoryDirector.luau",
      "scripts/SolController.luau",
      "scripts/DialogueProbe.luau",
      "scripts/visual_graphs/LoveStoryActLoop.mfgraph"
    ],
    assets: [
      "assets/data/StoryBible.json",
      "assets/data/DialogueBeats.json",
      "assets/data/ChoiceMatrix.json",
      "assets/data/EngineLabChecklist.json",
      "assets/audio/RainTheme.audio.json",
      "assets/audio/HeartbeatChoice.audio.json",
      "assets/sequences/IntroStation.seq2d.json",
      "assets/sprites/LoveLabCharacters.png",
      "assets/sprites/LoveLabCharacters.sprite.json",
      "assets/animations/LoveLabCharacters.spriteframes",
      "assets/ui/hud.ui2d.json",
      "assets/prefabs/Sol.prefab",
      "assets/prefabs/Mara.prefab",
      "assets/prefabs/MemoryTrigger.prefab"
    ],
    components: [
      "Dialogue",
      "QuestLog",
      "Interaction",
      "VisualScript",
      "UIElement",
      "AudioSource",
      "ParticleEmitter",
      "Trigger2D",
      "Checkpoint",
      "Saveable",
      "Sequencer2D"
    ],
    systems: []
  };
}

function createStoryPrefab(name, tint, tag, extraComponents = []) {
  const info = readEngineInfo();
  return {
    format: "miniforge.prefab",
    schema_version: 1,
    guid: `prefab-love-${name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
    version: info.engineVersion,
    engine_version: info.engineVersion,
    prefab_name: name,
    entity: entity({
      id: 9100,
      name,
      x: 0,
      y: 0,
      tag,
      layer: tag === "Trigger" ? "Triggers" : "Characters",
      script: name === "Sol" ? "SolController.luau" : name === "Mara" ? "DialogueProbe.luau" : null,
      tint,
      extraComponents
    })
  };
}

function makeLoveStoryGraph(engineVersion) {
  return {
    format: "miniforge.visual-graph",
    schema_version: 1,
    engine_version: engineVersion,
    version: engineVersion,
    kind: "MiniForgeVisualGraph",
    runtime: "rust_visual_graph",
    name: "LoveStoryActLoop",
    variables: {
      act: 1,
      affection: 35,
      honesty: 40,
      courage: 20,
      selected_choice: "none"
    },
    nodes: [
      { id: "start", type: "EventStart", next: "quest" },
      {
        id: "quest",
        type: "AddQuest",
        quest: "letters_under_rain",
        title: "Letters Under Rain",
        objectives: [
          { id: "meet", text: "Meet Mara at the station", progress: 0, target: 1 },
          { id: "letter", text: "Find the unsent letter", progress: 0, target: 1 },
          { id: "choice", text: "Choose what Sol says at the bridge", progress: 0, target: 1 }
        ],
        next: "status"
      },
      { id: "status", type: "SetState", state: "Act1Station", next: "log" },
      { id: "log", type: "Log", message: "Love story laboratory systems loaded", next: null },
      { id: "update", type: "EventUpdate", next: null },
      { id: "remember", type: "TriggerAbility", true_next: "meet_progress", false_next: "recharge" },
      { id: "meet_progress", type: "QuestProgress", quest: "letters_under_rain", objective: "meet", progress: 1, next: "choice_progress" },
      { id: "choice_progress", type: "QuestProgress", quest: "letters_under_rain", objective: "choice", progress: 1, next: "complete" },
      { id: "complete", type: "CompleteQuest", quest: "letters_under_rain", next: null },
      { id: "recharge", type: "RechargeAbility", amount: 1, next: null }
    ]
  };
}

function makeAudioEvent(engineVersion, name, bus, loop = false, volume = 1.0) {
  return {
    version: engineVersion,
    kind: "MiniForgeAudioEvent",
    runtime: "kira",
    name,
    bus,
    volume,
    fade_seconds: loop ? 1.5 : 0.0,
    actions: [{ type: bus === "Music" ? "play_music" : "play_sfx", cue: name, loop }]
  };
}

function createLoveStoryLab({ name = "MCP_LoveStoryLab", overwrite = false, projectDirectory } = {}) {
  const projectName = safeProjectName(name);
  const projectPath = path.resolve(projectDirectory || path.join(PROJECTS_DIR, projectName));
  if (existsSync(projectPath)) {
    if (!overwrite) {
      throw new Error(`Project already exists: ${projectPath}. Pass overwrite=true to recreate it.`);
    }
    rmSync(projectPath, { recursive: true, force: true });
  }

  const info = readEngineInfo();
  const paths = projectPaths(projectPath);
  ensureProjectFolders(paths);

  const startScene = "main.scene";
  mkdirSync(path.join(paths.assets, "sequences"), { recursive: true });
  const spriteDirectory = path.join(paths.assets, "sprites");
  const animationDirectory = path.join(paths.assets, "animations");
  mkdirSync(spriteDirectory, { recursive: true });
  mkdirSync(animationDirectory, { recursive: true });

  writeJson(path.join(projectPath, "project.json"), {
    project_name: projectName,
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    start_scene: startScene,
    current_scene: startScene,
    author: "MiniForge MCP",
    plugin_policy: "vanilla",
    description: "2D narrative love story laboratory for testing MiniForge gameplay, UI, audio, visual scripting, persistence, triggers and tilemap systems."
  });
  writeJson(path.join(projectPath, "engine_config.json"), {
    engine_name: "MiniForge",
    engine_alt_name: "MiniForge",
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    project_name: projectName,
    start_scene: startScene,
    autosave: true,
    autosave_interval_seconds: 35,
    safe_mode: true,
    config_version: 2,
    editor: { script_hot_reload: true, open_created_assets: true, fallback_assets: true },
    rendering: {
      backend: "macroquad",
      pixel_perfect: true,
      sprite_batching: true,
      tilemap_chunk_batching: true,
      post_processing: true,
      enable_3d: false,
      vsync: true
    },
    logs: {
      level: "info",
      file: "logs/miniforge.log",
      engine: "logs/engine.log",
      error: "logs/error.log"
    }
  });
  writeJson(path.join(projectPath, "manifest.json"), makeLoveStoryManifest(info.engineVersion));
  writeJson(path.join(paths.settings, "runtime_config.json"), {
    game_name: "Letters Under Rain",
    start_scene: startScene,
    window_width: 1280,
    window_height: 720,
    fullscreen: false,
    target_fps: 60,
    debug: true,
    quality_preset: "balanced",
    performance_class: "auto",
    worker_threads: "auto",
    parallel_asset_scan: true,
    prefer_metal_on_macos: true
  });
  writeJson(path.join(paths.settings, "build_settings.json"), {
    game_name: "Letters Under Rain",
    start_scene: startScene,
    target_fps: 60,
    window_width: 1280,
    window_height: 720,
    fullscreen: false,
    debug_mode: true,
    export_folder: "builds"
  });
  writeJson(path.join(paths.settings, "build_profiles.json"), {
    active: "Development",
    profiles: {
      Development: { debug_mode: true, target_fps: 60, narrative_debug: true },
      Release: { debug_mode: false, target_fps: 60 },
      Shipping: { debug_mode: false, target_fps: 60, strip_debug: true }
    }
  });
  writeJson(path.join(paths.settings, "input_map.json"), {
    actions: {
      Move: { display_name: "Move", category: "Gameplay", devices: ["keyboard", "gamepad"], description: "Move Sol in top-down scenes." },
      Interact: { display_name: "Interact", category: "Narrative", devices: ["keyboard"], description: "Talk, inspect or confirm a story beat." },
      Remember: { display_name: "Remember", category: "Narrative", devices: ["keyboard"], description: "Trigger memory ability and quest graph." },
      ChoiceA: { display_name: "Choice A", category: "Narrative", devices: ["keyboard"], description: "Pick the first dialogue choice." },
      ChoiceB: { display_name: "Choice B", category: "Narrative", devices: ["keyboard"], description: "Pick the second dialogue choice." },
      Pause: { display_name: "Pause", category: "System", devices: ["keyboard"], description: "Pause story playback." }
    },
    bindings: {
      Move: ["keyboard:wasd", "keyboard:arrows", "gamepad:left_stick"],
      Interact: ["e", "enter"],
      Remember: ["space"],
      ChoiceA: ["1"],
      ChoiceB: ["2"],
      Pause: ["escape"],
      move_left: ["a", "left"],
      move_right: ["d", "right"],
      move_up: ["w", "up"],
      move_down: ["s", "down"],
      interact: ["e", "enter"],
      remember: ["space"],
      choice_a: ["1"],
      choice_b: ["2"],
      pause: ["escape"],
      play: ["f5"],
      save: ["s"]
    }
  });
  writeJson(path.join(paths.settings, "tags.json"), {
    items: ["Untagged", "Player", "LoveInterest", "Friend", "Memory", "Trigger", "Audio", "Effects", "UI"]
  });
  writeJson(path.join(paths.settings, "layers.json"), {
    items: ["Default", "Ground", "Characters", "Props", "Triggers", "UI", "Effects", "Audio", "IgnoreSelection", "EditorOnly"]
  });
  writeJson(path.join(paths.settings, "physics2d.json"), {
    format: "miniforge.physics2d-settings",
    schema_version: 1,
    engine_version: info.engineVersion,
    gravity: [0, 0],
    fixed_timestep: 1 / 60,
    continuous_collision: true,
    trigger_events: true,
    layers: ["World", "Player", "Characters", "Props", "Trigger"]
  });
  writeJson(path.join(paths.settings, "editor_layout.json"), {
    format: "miniforge.editor-layout",
    schema_version: 1,
    engine_version: info.engineVersion,
    workspace: "2D",
    panels: ["Scene", "WorldOutliner", "Details", "ContentBrowser", "Blueprint", "SpriteEditor", "Problems", "Profiler"]
  });
  writeJson(path.join(paths.settings, "render_graph_2d.json"), {
    format: "miniforge.render-graph-2d",
    schema_version: 1,
    engine_version: info.engineVersion,
    kind: "RenderGraph2D",
    batcher: "SpriteBatcher",
    atlas: "TextureAtlas2D",
    passes: ["Tilemap", "WorldSprites", "Particles", "Lighting2D", "UI"]
  });
  writeJson(path.join(paths.assets, "ui", "hud.ui2d.json"), {
    format: "miniforge.ui-canvas-2d",
    schema_version: 1,
    engine_version: info.engineVersion,
    name: "LoveStoryHUD",
    reference_size: [1280, 720],
    scaling: "scale_with_screen",
    screen_manager: "ScreenManager2D",
    screen: "HUDScreen",
    widgets: ["HUD_Title", "HUD_Status", "HUD_Dialogue", "HUD_ChoiceA", "HUD_ChoiceB", "HUD_Affection"]
  });
  writeJson(path.join(paths.scenes, startScene), makeLoveStoryScene(info.engineVersion));

  writeBase64(
    path.join(spriteDirectory, "LoveLabCharacters.png"),
    "iVBORw0KGgoAAAANSUhEUgAAAIAAAAAgCAYAAADaInAlAAAAxElEQVR42u3SMQrCMBiG4Z7EyRN4CkdHB8FLFI/gUZwVehY3FyfB1T0uBjJpSFKK9HnhG8PfwtN1kiRJkiRJn+67Q4h7Xc9ZS9/U3n8etyEu9376pvb+/nILcbn30ze190/9I8Tl3k/fAAAAAAAAAAAAAFQDKFlLACVrCaBkLQGUDAAAAAAAAAAAAAAAAAAAAAAAAAAgu/WmD982NYBf3zf2/08NYOz/BwAAAAAAAAAAAABgjgAkzbHFcjW0nPv/dV8z6Q1Jq4BLzj2lGgAAAABJRU5ErkJggg=="
  );
  writeJson(path.join(spriteDirectory, "LoveLabCharacters.sprite.json"), {
    version: info.engineVersion,
    kind: "SpriteImport",
    name: "LoveLabCharacters",
    source: "assets/sprites/LoveLabCharacters.png",
    animations: [{ name: "idle", asset: "assets/animations/LoveLabCharacters.spriteframes", frames: 4 }],
    settings: { filter: "nearest", pixels_per_unit: 32, pivot: "center" }
  });
  writeJson(path.join(animationDirectory, "LoveLabCharacters.spriteframes"), {
    name: "LoveLabCharacters",
    texture: "assets/sprites/LoveLabCharacters.png",
    animations: [{
      name: "idle",
      fps: 4,
      looped: true,
      ping_pong: true,
      tags: ["love_lab", "character", "idle"],
      frames: [0, 1, 2, 3].map((index) => ({
        rect: { x: index * 32, y: 0, width: 32, height: 32 },
        duration: 0.25,
        pivot: [16, 16],
        hitboxes: [],
        hurtboxes: [],
        collision_shapes: [],
        events: []
      }))
    }]
  });

  writeText(path.join(paths.scripts, "StoryDirector.luau"), `--!strict
local affection = 35

function on_start()
    set_ui_text("HUD_Status", "ACT I · Meet Mara, recover the letter, then choose at the bridge.")
    set_ui_text("HUD_Dialogue", "Rain taps on the platform roof. Mara is waiting.")
    set_ui_progress_for("HUD_Affection", affection, 100)
    set_blackboard("affection", affection)
    play_sound("RainTheme", "Music", 0.72, true)
end

function on_key_down(key: string)
    if key == "Space" then
        set_ui_text("HUD_Dialogue", "MEMORY · The letter was written on the night the trains stopped.")
        play_sound("HeartbeatChoice", "SFX", 0.65, false)
    end
end
`);
  writeText(path.join(paths.scripts, "SolController.luau"), `--!strict
function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
end

function on_update(_dt: number)
    if input_pressed("A") then face_left() elseif input_pressed("D") then face_right() end
end

function on_collision_enter(other: string)
    set_ui_text("HUD_Status", "Sol touched " .. other)
end
`);
  writeText(path.join(paths.scripts, "DialogueProbe.luau"), `--!strict
function on_start()
    set_sprite("assets/sprites/LoveLabCharacters.sprite.json")
    play_sprite_animation("assets/animations/LoveLabCharacters.spriteframes", "idle")
end
`);
  writeJson(path.join(paths.graphs, "LoveStoryActLoop.mfgraph"), makeLoveStoryGraph(info.engineVersion));
  writeJson(path.join(paths.data, "StoryBible.json"), {
    title: "Letters Under Rain",
    format: "2D interactive love story",
    theme: "Love as a choice made after fear, distance and memory.",
    cast: {
      Sol: { role: "player", wound: "left without explaining", need: "speak honestly" },
      Mara: { role: "love_interest", wound: "waited for a letter that never arrived", need: "be chosen in the present" },
      Tomas: { role: "friend", function: "keeps the cafe and the truth open" }
    },
    acts: [
      { id: 1, name: "Station", goal: "reunion and first dialogue" },
      { id: 2, name: "Letter", goal: "recover context through object interaction" },
      { id: 3, name: "Bridge", goal: "branch ending by choice and affection score" }
    ]
  });
  writeJson(path.join(paths.data, "DialogueBeats.json"), {
    beats: [
      { id: "station_reunion", speaker: "Mara", line: "You still keep arriving before the rain stops.", emotion: "guarded" },
      { id: "letter_found", speaker: "Sol", line: "I wrote it and then trusted silence more than you.", emotion: "ashamed" },
      { id: "bridge_choice", speaker: "Mara", line: "Do you want a memory, or do you want tomorrow?", emotion: "hopeful" }
    ]
  });
  writeJson(path.join(paths.data, "ChoiceMatrix.json"), {
    variables: ["affection", "honesty", "courage", "memory_fragments"],
    choices: [
      { id: "tell_truth", label: "Tell her the truth", effects: { affection: 15, honesty: 20, courage: 10 }, ending_bias: "reconcile" },
      { id: "ask_dream", label: "Ask about her dream", effects: { affection: 10, honesty: 8, courage: 4 }, ending_bias: "slow_burn" },
      { id: "stay_silent", label: "Stay silent", effects: { affection: -12, honesty: -10, courage: -6 }, ending_bias: "farewell" }
    ],
    endings: [
      { id: "reconcile", requirement: "affection >= 65 and honesty >= 55" },
      { id: "slow_burn", requirement: "affection >= 45" },
      { id: "farewell", requirement: "affection < 45" }
    ]
  });
  writeJson(path.join(paths.data, "EngineLabChecklist.json"), {
    purpose: "Use this project as a regression lab for MiniForge MCP-generated 2D story games.",
    checks: [
      "Open scene and inspect Dialogue components on Mara and Tomas",
      "Move Sol with WASD through CharacterController2D",
      "Press Space to exercise Ability plus VisualScript quest flow",
      "Press 1 or 2 to resolve a Dialogue choice through NarrativeSystem and Luau",
      "Inspect QuestLog and Saveable on Sol",
      "Inspect Trigger2D areas for station and bridge beats",
      "Inspect AudioSource and audio event JSON placeholders",
      "Inspect ParticleEmitter and ParallaxLayer on RainParticles",
      "Inspect Sequencer2D on StoryDirector",
      "Inspect tilemap_layers for Ground, Decoration, Collision and Overlay"
    ]
  });
  writeJson(path.join(paths.audio, "RainTheme.audio.json"), makeAudioEvent(info.engineVersion, "RainTheme", "Music", true, 0.72));
  writeJson(path.join(paths.audio, "HeartbeatChoice.audio.json"), makeAudioEvent(info.engineVersion, "HeartbeatChoice", "SFX", false, 0.9));
  writeJson(path.join(paths.assets, "sequences", "IntroStation.seq2d.json"), {
    version: info.engineVersion,
    kind: "MiniForgeSequencer2D",
    name: "IntroStation",
    duration: 18.0,
    tracks: [
      { type: "camera", target: "Sol", keys: [{ time: 0.0, zoom: 1.0 }, { time: 8.0, zoom: 1.12 }, { time: 18.0, zoom: 1.02 }] },
      { type: "dialogue", target: "HUD_Dialogue", keys: [{ time: 1.0, text: "Rain taps on the platform roof." }, { time: 6.0, text: "Mara waits where the old train map used to hang." }] },
      { type: "audio", target: "RainTheme", keys: [{ time: 0.0, action: "fade_in", seconds: 2.0 }] },
      { type: "event", target: "StoryDirector", keys: [{ time: 12.0, event: "EnablePlayerInput" }] }
    ]
  });
  writeJson(path.join(paths.prefabs, "Sol.prefab"), createStoryPrefab("Sol", [92, 174, 230], "Player", [
    component("CharacterController2D", { mode: "topdown", walk_speed: 4.2 }),
    component("QuestLog"),
    component("Saveable", { save_key: "sol_prefab" })
  ]));
  writeJson(path.join(paths.prefabs, "Mara.prefab"), createStoryPrefab("Mara", [238, 120, 168], "LoveInterest", [
    makeDialogueLines("Mara", ["Prefab line: edit me in the Details Inspector."]),
    component("Interaction", { prompt: "Talk to Mara", radius: 1.8 })
  ]));
  writeJson(path.join(paths.prefabs, "MemoryTrigger.prefab"), createStoryPrefab("MemoryTrigger", [130, 190, 255], "Trigger", [
    component("Trigger2D", { width: 3.0, height: 3.0, overlap_mask: ["Player"] })
  ]));
  writeText(path.join(projectPath, "README.md"), `# Letters Under Rain

2D narrative love story lab generated by the MiniForge MCP using MiniForge ${info.engineVersion} / stream ${info.engineStreamVersion}.

This project is a test bed for story-game features: movement, dialogue, quests, choices, visual scripting, UI, audio events, particles, triggers, checkpoints, saveable entities, sequencer data and tilemap layers.

## Run Editor

\`\`\`bash
cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher
\`\`\`

## Validate Headless

\`\`\`bash
cargo run --no-default-features --features runtime --bin miniforge_headless -- ${rel(projectPath)} 3
\`\`\`

## Controls

- WASD: move Sol.
- E: interact or probe dialogue.
- Space: remember.
- 1 / 2: choose a dialogue branch.

## What To Inspect

- Sol: CharacterController2D, CameraFollow, QuestLog, Ability, Checkpoint and Saveable.
- Mara/Tomas: Dialogue and Interaction.
- StoryDirector: GameMode2D, GameState2D, Blackboard, Sequencer2D and VisualScript.
- RainAmbience/RainParticles: AudioSource, ParticleEmitter and ParallaxLayer.
- StationMemoryTrigger/BridgeChoiceTrigger: Trigger2D and checkpoint flow.
- assets/data: story bible, dialogue beats, choice matrix and engine lab checklist.
`);

  return {
    projectName,
    projectPath,
    engineVersion: info.engineVersion,
    engineStreamVersion: info.engineStreamVersion,
    startScene,
    createdFiles: [
      "project.json",
      "engine_config.json",
      "manifest.json",
      "saves/scenes/main.scene",
      "scripts/StoryDirector.luau",
      "scripts/SolController.luau",
      "scripts/DialogueProbe.luau",
      "scripts/visual_graphs/LoveStoryActLoop.mfgraph",
      "assets/data/StoryBible.json",
      "assets/data/DialogueBeats.json",
      "assets/data/ChoiceMatrix.json",
      "assets/data/EngineLabChecklist.json",
      "assets/audio/RainTheme.audio.json",
      "assets/audio/HeartbeatChoice.audio.json",
      "assets/sequences/IntroStation.seq2d.json",
      "assets/sprites/LoveLabCharacters.png",
      "assets/sprites/LoveLabCharacters.sprite.json",
      "assets/animations/LoveLabCharacters.spriteframes",
      "assets/ui/hud.ui2d.json",
      "assets/prefabs/Sol.prefab",
      "assets/prefabs/Mara.prefab",
      "assets/prefabs/MemoryTrigger.prefab",
      "README.md"
    ].map((item) => path.join(projectPath, item)),
    openCommand: `cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher`,
    validateCommand: `cargo run --bin miniforge -- --project ${rel(projectPath)} --runtime --no-launcher --headless-once`
  };
}

function provinceEntity(id, key, displayName, x, y, ownerTag, resource, tint, population, terrain = "plains") {
  return entity({
    id,
    name: `Province_${key}`,
    x,
    y,
    width: 2.2,
    height: 1.4,
    radius: 1.0,
    tag: "Province",
    layer: "Ground",
    tint,
    script: "ProvincePulse.rhai",
    extraComponents: [
      component("Province2D", {
        province_id: key,
        display_name: displayName,
        owner_tag: ownerTag,
        controller_tag: ownerTag,
        terrain,
        population,
        resource,
        infrastructure: terrain === "mountains" ? 0.7 : 1.1,
        literacy: ownerTag === "ALB" ? 0.32 : ownerTag === "BOR" ? 0.24 : 0.18,
        factory_slots: resource === "coal" || resource === "iron" ? 2 : 1,
        supply_limit: terrain === "mountains" ? 6.0 : 12.0
      }),
      component("PopulationPops2D", {
        pops: [
          { type: "farmers", size: population * 0.58, militancy: 0.06, consciousness: 0.15, wealth: 0.32 },
          { type: "laborers", size: population * 0.22, militancy: 0.08, consciousness: 0.22, wealth: 0.28 },
          { type: "craftsmen", size: population * 0.12, militancy: 0.10, consciousness: 0.30, wealth: 0.42 },
          { type: "bureaucrats", size: population * 0.03, militancy: 0.02, consciousness: 0.36, wealth: 0.70 }
        ]
      }),
      component("Market2D", {
        market_id: `${key}_market`,
        goods: {
          grain: { stockpile: resource === "grain" ? 140.0 : 55.0, price: 1.0, demand: 80.0, supply: resource === "grain" ? 130.0 : 45.0 },
          coal: { stockpile: resource === "coal" ? 90.0 : 20.0, price: 2.2, demand: 65.0, supply: resource === "coal" ? 80.0 : 15.0 },
          iron: { stockpile: resource === "iron" ? 80.0 : 18.0, price: 2.6, demand: 70.0, supply: resource === "iron" ? 70.0 : 12.0 }
        }
      }),
      component("Factory2D", {
        factory_id: `${key}_workshop`,
        good: resource === "iron" || resource === "coal" ? "steel" : "canned_food",
        workers: population * 0.07,
        throughput: ownerTag === "ALB" ? 1.15 : 0.95,
        profit: 12.0
      }),
      component("ObjectiveMarker", { label: displayName, color: tint, pulse: true })
    ]
  });
}

function nationEntity(id, tag, displayName, x, y, color, capitalProvince, treasury, government) {
  return entity({
    id,
    name: `Nation_${tag}`,
    x,
    y,
    visible: false,
    locked: true,
    tag: "Nation",
    layer: "EditorOnly",
    extraComponents: [
      component("Nation2D", {
        nation_tag: tag,
        display_name: displayName,
        capital_province: capitalProvince,
        government,
        treasury,
        prestige: tag === "ALB" ? 14.0 : tag === "BOR" ? 9.0 : 5.0,
        primary_culture: tag === "ALB" ? "albian" : tag === "BOR" ? "boreal" : "cyran",
        accepted_cultures: tag === "CYR" ? ["cyran", "borderfolk"] : []
      }),
      component("EconomyWallet", { resources: { Gold: treasury, Grain: 180.0, Coal: 70.0, Iron: 55.0 }, capacity: 999999.0 }),
      component("Diplomacy2D", {
        relations: { ALB: tag === "ALB" ? 100 : 20, BOR: tag === "BOR" ? 100 : -10, CYR: tag === "CYR" ? 100 : 5 },
        rivals: tag === "ALB" ? ["BOR"] : tag === "BOR" ? ["ALB"] : [],
        alliances: tag === "CYR" ? ["ALB"] : []
      }),
      component("ResearchTree2D", {
        current_research: tag === "ALB" ? "organized_factories" : "professional_army",
        progress: tag === "ALB" ? 0.42 : 0.24,
        points_per_month: tag === "ALB" ? 2.0 : 1.4
      }),
      component("Team", { team_id: id, team_name: displayName, color })
    ]
  });
}

function armyEntity(id, name, x, y, ownerTag, provinceId, tint, regiments) {
  return unit({
    id,
    name,
    x,
    y,
    tag: ownerTag === "ALB" ? "Player" : "Enemy",
    layer: "Units",
    tint,
    extraComponents: [
      component("ArmyStack2D", {
        army_id: name.toLowerCase().replace(/[^a-z0-9]+/g, "_"),
        owner_tag: ownerTag,
        province_id: provinceId,
        regiments,
        supply: 0.92,
        dig_in: 0.15
      }),
      component("Health", { max_health: 100.0, health: 100.0 }),
      component("Commandable", { can_move: true, can_attack: true, command_tags: ["move", "attack_move", "hold"] }),
      component("Vision", { radius: 6.0 }),
      component("ThreatSource", { strength: regiments.length * 3.0, radius: 4.5 })
    ]
  });
}

function makeIronTreatyScene(engineVersion) {
  const entities = [
    entity({
      id: 1,
      name: "CampaignRules",
      x: 0,
      y: 0,
      visible: false,
      locked: true,
      layer: "EditorOnly",
      script: "CampaignController.rhai",
      extraComponents: [
        component("GameMode2D", { start_scene: "saves/scenes/campaign_1836.scene", rules: { tick_mode: "monthly", victory: "great_power_rank" } }),
        component("GameState2D", { phase: "campaign", score: { prestige: 14, industry: 21, military: 12 } }),
        component("WorldPartition2D", { cell_size: 8.0, streaming_enabled: false }),
        component("RuntimeBudget2D", { max_entities: 25000, max_visible_sprites: 6000, max_script_ms: 6.0 })
      ]
    }),
    nationEntity(10, "ALB", "Albian Union", 0, 0, [80, 150, 255], "albion", 1600.0, "constitutional_monarchy"),
    nationEntity(11, "BOR", "Boreal Empire", 0, 0, [255, 100, 90], "borgrad", 1300.0, "absolute_monarchy"),
    nationEntity(12, "CYR", "Cyran Republic", 0, 0, [120, 220, 150], "cyrhaven", 900.0, "republic"),
    provinceEntity(100, "albion", "Albion", 6, 7, "ALB", "grain", [80, 150, 255], 420000, "plains"),
    provinceEntity(101, "whitecliff", "Whitecliff", 9, 5, "ALB", "coal", [95, 165, 255], 260000, "hills"),
    provinceEntity(102, "ironvale", "Ironvale", 10, 9, "ALB", "iron", [75, 135, 230], 310000, "hills"),
    provinceEntity(103, "borgrad", "Borgrad", 17, 7, "BOR", "coal", [255, 100, 90], 520000, "plains"),
    provinceEntity(104, "frostmarch", "Frostmarch", 20, 5, "BOR", "iron", [245, 90, 85], 230000, "mountains"),
    provinceEntity(105, "redriver", "Red River", 19, 10, "BOR", "grain", [230, 85, 80], 360000, "plains"),
    provinceEntity(106, "cyrhaven", "Cyrhaven", 13, 13, "CYR", "grain", [120, 220, 150], 300000, "coast"),
    provinceEntity(107, "glassport", "Glassport", 15, 15, "CYR", "coal", [105, 205, 140], 180000, "coast"),
    armyEntity(200, "First Albian Army", 8, 8, "ALB", "albion", [120, 190, 255], [
      { type: "infantry", strength: 3000.0, organization: 0.9, morale: 0.8 },
      { type: "artillery", strength: 1000.0, organization: 0.7, morale: 0.75 }
    ]),
    armyEntity(201, "Boreal Guard", 17, 8, "BOR", "borgrad", [255, 125, 110], [
      { type: "infantry", strength: 4000.0, organization: 0.82, morale: 0.86 },
      { type: "cavalry", strength: 1000.0, organization: 0.76, morale: 0.8 }
    ]),
    armyEntity(202, "Cyran Volunteers", 14, 13, "CYR", "cyrhaven", [145, 235, 170], [
      { type: "infantry", strength: 2500.0, organization: 0.68, morale: 0.72 }
    ]),
    entity({
      id: 300,
      name: "NorthernCoalRoute",
      x: 12,
      y: 5,
      tag: "Trade",
      layer: "Ground",
      tint: [250, 220, 120],
      extraComponents: [
        component("TradeRoute2D", {
          route_id: "northern_coal",
          from_market: "whitecliff_market",
          to_market: "borgrad_market",
          good: "coal",
          volume: 42.0,
          capacity: 90.0,
          profit: 18.0,
          risk: 0.22
        }),
        component("ObjectiveMarker", { label: "Coal Route", color: [250, 220, 120] })
      ]
    }),
    uiLabel(400, "HUD_Campaign", "Iron Treaty 1836: administra poblacion, industria, ejercitos y diplomacia.", 24, 24, 680),
    uiLabel(401, "HUD_Tip", "Selecciona provincias/ejercitos. El contenido vive en componentes editables y datos JSON.", 24, 64, 690),
    uiLabel(402, "HUD_Date", "Enero 1836 - Pausado", 24, 104, 260)
  ];

  return {
    version: engineVersion,
    engine_version: engineVersion,
    scene_name: "campaign_1836",
    mode: "EDITOR",
    active_tool: "Select",
    tile_brush: 0,
    brush_size: 1,
    camera: { x: 390.0, y: 270.0, zoom: 0.92 },
    control_groups: {},
    grid: { width: 80, height: 48, tile_size: 32, chunk_size: 8 },
    tiles: [],
    tilemap_layers: [],
    settings: {
      genre: "grand_strategy_rts",
      start_year: 1836,
      tick_rate: "monthly",
      victory_conditions: ["great_power_rank", "industrial_score", "war_score"],
      generated_by: "miniforge-mcp"
    },
    entities,
    editor_view_settings: {},
    ui_canvases: []
  };
}

function makeIronTreatyManifest(engineVersion) {
  return {
    engine_version: engineVersion,
    runtime: "rust",
    scenes: ["saves/scenes/campaign_1836.scene"],
    scripts: [
      "scripts/CampaignController.rhai",
      "scripts/ProvincePulse.rhai",
      "scripts/visual_graphs/MonthlyCampaignTick.mfgraph"
    ],
    assets: [
      "assets/data/WorldMap.json",
      "assets/data/Nations.json",
      "assets/data/GoodsMarket.json",
      "assets/data/TechTree.json",
      "assets/data/Decisions.json"
    ],
    components: [
      "Province2D",
      "Nation2D",
      "PopulationPops2D",
      "Market2D",
      "Factory2D",
      "Diplomacy2D",
      "ResearchTree2D",
      "ArmyStack2D",
      "WarGoal2D",
      "TradeRoute2D"
    ],
    systems: []
  };
}

function createIronTreaty({ name = "MCP_IronTreaty_1836", overwrite = false, projectDirectory } = {}) {
  const projectName = safeProjectName(name);
  const projectPath = path.resolve(projectDirectory || path.join(PROJECTS_DIR, projectName));
  if (existsSync(projectPath)) {
    if (!overwrite) {
      throw new Error(`Project already exists: ${projectPath}. Pass overwrite=true to recreate it.`);
    }
    rmSync(projectPath, { recursive: true, force: true });
  }

  const info = readEngineInfo();
  const paths = projectPaths(projectPath);
  ensureProjectFolders(paths);
  const startScene = "campaign_1836.scene";

  writeJson(path.join(projectPath, "project.json"), {
    project_name: projectName,
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    start_scene: startScene,
    current_scene: startScene,
    author: "MiniForge MCP",
    description: "Grand strategy RTS sandbox inspired by nineteenth-century diplomacy, industry, population, and armies."
  });
  writeJson(path.join(projectPath, "engine_config.json"), {
    engine_name: "MiniForge",
    engine_alt_name: "MiniForge",
    engine_version: info.engineVersion,
    engine_stream_version: info.engineStreamVersion,
    project_name: projectName,
    start_scene: startScene,
    autosave: true,
    autosave_interval_seconds: 45,
    safe_mode: true,
    config_version: 2,
    editor: { script_hot_reload: true, open_created_assets: true, fallback_assets: true },
    rendering: {
      backend: "macroquad",
      pixel_perfect: true,
      sprite_batching: true,
      tilemap_chunk_batching: true,
      post_processing: true,
      enable_3d: false,
      vsync: true
    },
    logs: {
      level: "info",
      file: "logs/miniforge.log",
      engine: "logs/engine.log",
      error: "logs/error.log"
    }
  });
  writeJson(path.join(projectPath, "manifest.json"), makeIronTreatyManifest(info.engineVersion));
  writeJson(path.join(paths.settings, "runtime_config.json"), {
    game_name: "Iron Treaty 1836",
    start_scene: startScene,
    window_width: 1400,
    window_height: 820,
    fullscreen: false,
    target_fps: 60,
    debug: true,
    quality_preset: "high",
    performance_class: "auto",
    worker_threads: "auto",
    parallel_asset_scan: true,
    prefer_metal_on_macos: true
  });
  writeJson(path.join(paths.settings, "build_settings.json"), {
    game_name: "Iron Treaty 1836",
    start_scene: startScene,
    target_fps: 60,
    window_width: 1400,
    window_height: 820,
    fullscreen: false,
    debug_mode: true,
    export_folder: "builds"
  });
  writeJson(path.join(paths.settings, "build_profiles.json"), {
    active: "Development",
    profiles: {
      Development: { debug_mode: true, target_fps: 60, simulation_tick: "monthly" },
      Release: { debug_mode: false, target_fps: 60 },
      Shipping: { debug_mode: false, target_fps: 60, strip_debug: true }
    }
  });
  writeJson(path.join(paths.settings, "input_map.json"), makeInputMap());
  writeJson(path.join(paths.settings, "tags.json"), {
    items: ["Untagged", "Player", "Enemy", "Neutral", "Province", "Nation", "Trade", "Army"]
  });
  writeJson(path.join(paths.settings, "layers.json"), {
    items: ["Default", "Ground", "Units", "Buildings", "UI", "Effects", "IgnoreSelection", "EditorOnly"]
  });
  writeJson(path.join(paths.scenes, startScene), makeIronTreatyScene(info.engineVersion));
  writeText(path.join(paths.scripts, "CampaignController.rhai"), `fn on_start() {
    set_ui_text("HUD_Campaign", "Iron Treaty 1836 listo: economia, pops, diplomacia e industria cargadas.");
}

fn on_update(dt) {
    if input_pressed("Space") {
        set_ui_text("HUD_Date", "Simulacion mensual en pausa tactica");
    }
}
`);
  writeText(path.join(paths.scripts, "ProvincePulse.rhai"), `fn on_start() {
    ui_text("Provincia lista");
}
`);
  writeJson(path.join(paths.graphs, "MonthlyCampaignTick.mfgraph"), {
    version: info.engineVersion,
    kind: "MiniForgeVisualGraph",
    runtime: "rust_visual_graph",
    name: "MonthlyCampaignTick",
    variables: {
      month: 1,
      treasury_delta: 0.0,
      militancy_pressure: 0.0,
      industrial_score: 21
    },
    nodes: [
      { id: "start", type: "EventStart", next: "announce" },
      { id: "update", type: "EventUpdate", next: "economy" },
      { id: "announce", type: "Log", message: "Grand strategy campaign systems loaded", next: null },
      { id: "economy", type: "EconomyAdd", resource: "Gold", amount: 5, next: "research" },
      { id: "research", type: "Log", message: "Monthly research and market pass", next: null }
    ]
  });
  writeJson(path.join(paths.data, "WorldMap.json"), {
    map_name: "Aurelian Basin",
    projection: "boardgame_grid",
    provinces: ["albion", "whitecliff", "ironvale", "borgrad", "frostmarch", "redriver", "cyrhaven", "glassport"],
    adjacencies: {
      albion: ["whitecliff", "ironvale", "cyrhaven"],
      whitecliff: ["albion", "borgrad"],
      ironvale: ["albion", "redriver", "cyrhaven"],
      borgrad: ["whitecliff", "frostmarch", "redriver"],
      frostmarch: ["borgrad"],
      redriver: ["borgrad", "ironvale", "cyrhaven"],
      cyrhaven: ["albion", "ironvale", "redriver", "glassport"],
      glassport: ["cyrhaven"]
    }
  });
  writeJson(path.join(paths.data, "Nations.json"), {
    playable: "ALB",
    nations: {
      ALB: { name: "Albian Union", capital: "albion", ideology: "liberal", goals: ["industrialize", "contain_boreal"] },
      BOR: { name: "Boreal Empire", capital: "borgrad", ideology: "reactionary", goals: ["expand_west", "secure_coal"] },
      CYR: { name: "Cyran Republic", capital: "cyrhaven", ideology: "republican", goals: ["trade_power", "neutrality"] }
    }
  });
  writeJson(path.join(paths.data, "GoodsMarket.json"), {
    goods: {
      grain: { base_price: 1.0, category: "food" },
      coal: { base_price: 2.2, category: "industrial" },
      iron: { base_price: 2.6, category: "industrial" },
      steel: { base_price: 5.4, category: "manufactured" },
      canned_food: { base_price: 3.8, category: "military" }
    },
    update_order: ["local_supply", "pop_needs", "factory_inputs", "trade_routes", "price_adjust"]
  });
  writeJson(path.join(paths.data, "TechTree.json"), {
    schools: {
      industry: ["steam_power", "organized_factories", "railroad_logistics"],
      army: ["professional_army", "breech_loaded_rifles", "general_staff"],
      society: ["public_schools", "civil_service", "mass_politics"]
    }
  });
  writeJson(path.join(paths.data, "Decisions.json"), {
    decisions: [
      { id: "subsidize_steel", title: "Subsidize Steelworks", cost: { Gold: 120 }, effects: { factory_throughput: 0.15 } },
      { id: "mobilize_reserves", title: "Mobilize Reserves", cost: { Grain: 60 }, effects: { regiments: 2, militancy: 0.05 } },
      { id: "trade_mission", title: "Cyran Trade Mission", cost: { Gold: 80 }, effects: { relation_CYR: 25, tariff_income: 0.05 } }
    ]
  });
  writeText(path.join(projectPath, "README.md"), `# Iron Treaty 1836

Grand strategy RTS sandbox generado por el MCP de MiniForge usando MiniForge ${info.engineVersion} / stream ${info.engineStreamVersion}.

No copia Victoria 2; usa una idea parecida: provincias, pops, mercado, industrias, diplomacia, investigacion y ejercitos en un mapa 2D editable.

## Ejecutar editor

\`\`\`bash
cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher
\`\`\`

## Validar sin ventana

\`\`\`bash
cargo run --bin miniforge -- --project ${rel(projectPath)} --runtime --no-launcher --headless-once
\`\`\`

## Que mirar en el Inspector

- Provincias: \`Province2D\`, \`PopulationPops2D\`, \`Market2D\`, \`Factory2D\`.
- Naciones: \`Nation2D\`, \`Diplomacy2D\`, \`ResearchTree2D\`.
- Ejercitos: \`ArmyStack2D\`, \`ThreatSource\`, \`Commandable\`.
- Ruta comercial: \`TradeRoute2D\`.

## Archivos de diseno

- \`assets/data/WorldMap.json\`
- \`assets/data/Nations.json\`
- \`assets/data/GoodsMarket.json\`
- \`assets/data/TechTree.json\`
- \`assets/data/Decisions.json\`
`);

  return {
    projectName,
    projectPath,
    engineVersion: info.engineVersion,
    engineStreamVersion: info.engineStreamVersion,
    startScene,
    createdFiles: [
      "project.json",
      "engine_config.json",
      "manifest.json",
      `saves/scenes/${startScene}`,
      "scripts/CampaignController.rhai",
      "scripts/ProvincePulse.rhai",
      "scripts/visual_graphs/MonthlyCampaignTick.mfgraph",
      "assets/data/WorldMap.json",
      "assets/data/Nations.json",
      "assets/data/GoodsMarket.json",
      "assets/data/TechTree.json",
      "assets/data/Decisions.json",
      "README.md"
    ].map((item) => path.join(projectPath, item)),
    openCommand: `cargo run --bin miniforge_editor -- --project ${rel(projectPath)} --no-launcher`,
    validateCommand: `cargo run --bin miniforge -- --project ${rel(projectPath)} --runtime --no-launcher --headless-once`
  };
}

function listMiniForgeProjects() {
  mkdirSync(PROJECTS_DIR, { recursive: true });
  return readdirSync(PROJECTS_DIR)
    .map((name) => path.join(PROJECTS_DIR, name))
    .filter((projectPath) => {
      try {
        return statSync(projectPath).isDirectory() && existsSync(path.join(projectPath, "project.json"));
      } catch {
        return false;
      }
    })
    .map((projectPath) => {
      let data = {};
      try {
        data = JSON.parse(readText(path.join(projectPath, "project.json")));
      } catch {
        data = {};
      }
      return {
        name: data.project_name || path.basename(projectPath),
        path: projectPath,
        engine_version: data.engine_version || "unknown",
        start_scene: data.start_scene || "main.scene"
      };
    });
}

async function runHeadlessValidation(projectPath, timeoutSeconds = 120) {
  const absoluteProjectPath = path.resolve(projectPath || path.join(PROJECTS_DIR, "MCP_AstroHarvester"));
  const args = [
    "run",
    "--bin",
    "miniforge",
    "--",
    "--project",
    absoluteProjectPath,
    "--runtime",
    "--no-launcher",
    "--headless-once"
  ];
  try {
    const result = await execFileAsync("cargo", args, {
      cwd: ENGINE_ROOT,
      timeout: timeoutSeconds * 1000,
      maxBuffer: 1024 * 1024 * 8,
      env: { ...process.env, NO_COLOR: "1" }
    });
    return {
      ok: true,
      projectPath: absoluteProjectPath,
      command: `cargo ${args.join(" ")}`,
      stdout: result.stdout.trim(),
      stderr: result.stderr.trim()
    };
  } catch (error) {
    const report = {
      ok: false,
      projectPath: absoluteProjectPath,
      command: `cargo ${args.join(" ")}`,
      message: error.message,
      stdout: String(error.stdout || "").trim(),
      stderr: String(error.stderr || "").trim()
    };
    appendFeedback({
      severity: "problem",
      title: "Headless validation failed",
      details: [
        `Project: ${absoluteProjectPath}`,
        `Command: ${report.command}`,
        `Error: ${report.message}`,
        report.stderr ? `stderr: ${report.stderr.slice(0, 3000)}` : null
      ].filter(Boolean).join("\n")
    });
    return report;
  }
}

function ensureFeedbackFile() {
  if (!existsSync(FEEDBACK_PATH)) {
    writeText(FEEDBACK_PATH, `# MiniForge MCP Feedback

Este archivo lo usa el MCP de MiniForge para apuntar problemas, recomendaciones y observaciones que luego puedes pasar a Codex para mejorar el motor.

`);
  }
}

function appendFeedback({ severity = "note", title, details, source = "miniforge-mcp" }) {
  ensureFeedbackFile();
  const date = new Date().toISOString();
  const entry = `## ${date} - ${severity.toUpperCase()} - ${title || "Untitled"}

Fuente: ${source}

${details || "Sin detalles."}

`;
  writeFileSync(FEEDBACK_PATH, entry, { encoding: "utf8", flag: "a" });
  return { feedbackPath: FEEDBACK_PATH, entry };
}

const server = new McpServer({
  name: "miniforge-mcp",
  version: SERVER_VERSION
});

server.registerTool("engine_status", {
  title: "MiniForge Engine Status",
  description: "Read the current MiniForge engine version, repo paths, and recommended commands.",
  inputSchema: {}
}, async () => {
  const result = readEngineInfo();
  return textResult(result, result);
});

server.registerTool("list_projects", {
  title: "List MiniForge Projects",
  description: "List MiniForge projects under the engine projects folder.",
  inputSchema: {}
}, async () => {
  const result = { projectsDir: PROJECTS_DIR, projects: listMiniForgeProjects() };
  return textResult(result, result);
});

server.registerTool("create_game", {
  title: "Create MiniForge Game",
  description: "Create a playable MiniForge demo project using the current engine version.",
  inputSchema: {
    name: z.string().optional().describe("Project folder name. Defaults to MCP_AstroHarvester."),
    template: z.enum(["astro_harvester", "grand_strategy_rts", "love_story_lab"]).optional().describe("Game template to generate."),
    overwrite: z.boolean().optional().describe("Recreate the project if it already exists."),
    projectDirectory: z.string().optional().describe("Absolute output directory. Defaults to <engine>/projects/<name>.")
  }
}, async (args) => {
  const result = args.template === "grand_strategy_rts"
    ? createIronTreaty({ ...args, name: args.name || "MCP_IronTreaty_1836" })
    : args.template === "love_story_lab"
      ? createLoveStoryLab({ ...args, name: args.name || "MCP_LoveStoryLab" })
      : createAstroHarvester(args);
  return textResult(result, result);
});

server.registerTool("validate_game", {
  title: "Validate MiniForge Game",
  description: "Run the MiniForge headless runtime once for a generated project. Failures are also logged to the feedback file.",
  inputSchema: {
    projectPath: z.string().optional().describe("Project path. Defaults to projects/MCP_AstroHarvester."),
    timeoutSeconds: z.number().int().min(10).max(600).optional()
  }
}, async ({ projectPath, timeoutSeconds }) => {
  const result = await runHeadlessValidation(projectPath, timeoutSeconds || 120);
  return textResult(result, result);
});

server.registerTool("record_feedback", {
  title: "Record MiniForge Feedback",
  description: "Append a problem, recommendation, or note to docs/MINIFORGE_MCP_FEEDBACK.md.",
  inputSchema: {
    severity: z.enum(["problem", "recommendation", "note"]).optional(),
    title: z.string(),
    details: z.string(),
    source: z.string().optional()
  }
}, async (args) => {
  const result = appendFeedback(args);
  return textResult(result, result);
});

async function main() {
  const [first, second, third] = process.argv.slice(2);
  if (first === "--self-test") {
    const info = readEngineInfo();
    console.log(JSON.stringify({ ok: true, server: "miniforge-mcp", ...info }, null, 2));
    return;
  }
  if (first === "--create-demo-game") {
    const result = createAstroHarvester({ name: second || "MCP_AstroHarvester", overwrite: true });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  if (first === "--create-grand-strategy") {
    const result = createIronTreaty({ name: second || "MCP_IronTreaty_1836", overwrite: true });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  if (first === "--create-love-story-lab") {
    const result = createLoveStoryLab({
      name: second || "MCP_LoveStoryLab",
      overwrite: true,
      projectDirectory: third
    });
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  if (first === "--validate-game") {
    const result = await runHeadlessValidation(second || path.join(PROJECTS_DIR, "MCP_AstroHarvester"), 180);
    console.log(JSON.stringify(result, null, 2));
    return;
  }
  ensureFeedbackFile();
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error(`MiniForge MCP running on stdio for ${ENGINE_ROOT}`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
