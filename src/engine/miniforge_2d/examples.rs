use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::engine::miniforge_2d::ai::minimal_behavior_tree;
use crate::engine::miniforge_2d::animation_blueprint::minimal_animation_blueprint;
use crate::engine::miniforge_2d::blueprint::minimal_blueprint_graph;
use crate::engine::miniforge_2d::blueprint_library::BlueprintLibrary2D;
use crate::engine::miniforge_2d::editor_layout::EditorLayout2D;
use crate::engine::miniforge_2d::gameplay::GameFramework2D;
use crate::engine::miniforge_2d::gameplay_ability::minimal_ability_system;
use crate::engine::miniforge_2d::massive_world2d::{
    RuntimeBudgetStats2D, SaveSharding2D, SpawnDirector2D, SpawnRule2D, minimal_massive_world2d,
};
use crate::engine::miniforge_2d::packaging2d::minimal_package_manifest;
use crate::engine::miniforge_2d::paper2d::minimal_paper2d_assets;
use crate::engine::miniforge_2d::particles2d::minimal_particle_system;
use crate::engine::miniforge_2d::physics2d::minimal_physics_config;
use crate::engine::miniforge_2d::project_settings2d::ProjectSettings2D;
use crate::engine::miniforge_2d::rts_tools::RtsTools2D;
use crate::engine::miniforge_2d::scene_view::SceneView2D;
use crate::engine::miniforge_2d::sequencer2d::minimal_sequencer;
use crate::engine::miniforge_2d::tilemap_editor2d::minimal_tilemap_editor;
use crate::engine::miniforge_2d::toolbar::{Toolbar2D, ToolbarStatus2D};
use crate::engine::miniforge_2d::ui_designer::UiDesigner2D;
use crate::engine::miniforge_2d::ui_framework::minimal_ui_canvas;
use crate::engine::render_3d::{HybridScene3DStarter, Render3DCompatibilityPlan};
use crate::render::backend::RenderBackendConfig;

pub fn minimal_examples() -> BTreeMap<String, Value> {
    BTreeMap::from([
        (
            "editor_layout".to_string(),
            json!(EditorLayout2D::default()),
        ),
        (
            "toolbar".to_string(),
            json!(Toolbar2D::new(ToolbarStatus2D::default())),
        ),
        ("actor_component".to_string(), actor_component_example()),
        (
            "game_framework".to_string(),
            json!(GameFramework2D::default()),
        ),
        ("ability_system".to_string(), minimal_ability_system()),
        (
            "blueprint_graph".to_string(),
            json!(minimal_blueprint_graph()),
        ),
        ("content_browser".to_string(), content_browser_example()),
        ("details_inspector".to_string(), details_inspector_example()),
        ("world_outliner".to_string(), world_outliner_example()),
        ("scene_view_2d".to_string(), json!(SceneView2D::default())),
        ("hybrid_3d_rendering".to_string(), hybrid_3d_example()),
        ("paper2d".to_string(), minimal_paper2d_assets()),
        ("tilemap_editor2d".to_string(), minimal_tilemap_editor()),
        ("particles2d".to_string(), minimal_particle_system()),
        (
            "animation_blueprint".to_string(),
            json!(minimal_animation_blueprint()),
        ),
        ("umg_like_ui".to_string(), json!(minimal_ui_canvas())),
        ("ui_designer".to_string(), json!(UiDesigner2D::default())),
        (
            "blueprint_library".to_string(),
            json!(BlueprintLibrary2D::default_library()),
        ),
        ("sequencer2d".to_string(), json!(minimal_sequencer())),
        ("physics2d".to_string(), minimal_physics_config()),
        (
            "ai_behavior_tree".to_string(),
            json!(minimal_behavior_tree()),
        ),
        (
            "rts_tools".to_string(),
            json!(RtsTools2D::default().minimal_demo_scene()),
        ),
        ("massive_world2d".to_string(), massive_world2d_example()),
        ("asset_registry".to_string(), asset_registry_example()),
        ("plugin_system".to_string(), plugin_system_example()),
        (
            "project_settings".to_string(),
            json!(ProjectSettings2D::default()),
        ),
        ("packaging".to_string(), json!(minimal_package_manifest())),
        ("play_in_editor".to_string(), pie_example()),
    ])
}

pub fn examples_document() -> Value {
    json!({
        "format": "miniforge_2d_examples",
        "version": 1,
        "description": "Minimal JSON examples for MiniForge2D systems inspired by high-level editor architecture.",
        "examples": minimal_examples()
    })
}

fn actor_component_example() -> Value {
    json!({
        "type": "Actor2D",
        "name": "PickupActor",
        "position": [2.0, 3.0],
        "components": [
            {"component_type": "Transform", "x": 2.0, "y": 3.0, "rotation": 0.0, "scale_x": 1.0, "scale_y": 1.0},
            {"component_type": "SpriteRenderer", "sprite_name": "Coin", "sorting_order": 10},
            {"component_type": "Collider2D", "shape": "circle", "radius": 0.35, "is_trigger": true},
            {"component_type": "VisualScript", "graph_name": "BP_CoinPickup"}
        ]
    })
}

fn content_browser_example() -> Value {
    json!({
        "runtime": "miniforge_content_browser_2d",
        "assets": {
            "assets/sprites/player.png": {
                "guid": "asset-player-sprite",
                "path": "assets/sprites/player.png",
                "name": "player",
                "asset_type": "Sprite2D",
                "labels": ["player", "runtime"],
                "preview": {"preview_type": "sprite", "thumbnail_path": "assets/sprites/player.png", "summary": "32x32 player sprite"},
                "dependencies": [],
                "valid": true,
                "metadata": {"pixels_per_unit": 32}
            }
        }
    })
}

fn details_inspector_example() -> Value {
    json!({
        "target_name": "PlayerPawn",
        "sections": [
            {"title": "Transform", "fields": [{"path": "x", "label": "X", "value_type": "number", "editable": true, "value_preview": "4.000"}]},
            {"title": "Pawn2D", "fields": [{"path": "components.Pawn2D.auto_possess", "label": "auto_possess", "value_type": "bool", "editable": true, "value_preview": "true"}]}
        ]
    })
}

fn world_outliner_example() -> Value {
    json!({
        "items": [
            {"id": 1, "name": "WorldRoot", "entity_type": "Actor2D", "parent_id": null, "depth": 0, "enabled": true, "visible": true, "locked": false, "children": [2]},
            {"id": 2, "name": "PlayerPawn", "entity_type": "Pawn2D", "parent_id": 1, "depth": 1, "enabled": true, "visible": true, "locked": false, "children": []}
        ]
    })
}

fn hybrid_3d_example() -> Value {
    let config = RenderBackendConfig {
        enable_3d: true,
        hybrid_2d_3d: true,
        depth_buffer: true,
        mesh_batching: true,
        ..RenderBackendConfig::default()
    };
    json!({
        "runtime": "miniforge_hybrid_2d_3d",
        "compatibility_plan": Render3DCompatibilityPlan::from_config(&config),
        "starter_scene": HybridScene3DStarter::minimal(),
        "components": [
            {"component_type": "Transform3D"},
            {"component_type": "MeshRenderer3D", "mesh": "builtin:cube", "material": "Default3D"},
            {"component_type": "Billboard3D", "sprite": "assets/sprites/player.png"},
            {"component_type": "Camera3D", "renders_2d_overlay": true},
            {"component_type": "HybridScene3D", "physics_mode": "2d_gameplay"}
        ]
    })
}

fn massive_world2d_example() -> Value {
    let (partition, budget, pool) = minimal_massive_world2d();
    let mut saves = SaveSharding2D::new(4, "saves/profile/global.json");
    saves.mark_dirty(0, 0);
    saves.mark_dirty(4, -1);
    json!({
        "world_partition": partition,
        "runtime_budget": budget,
        "current_stats": RuntimeBudgetStats2D {
            entities: 12000,
            visible_sprites: 3500,
            particles: 9000,
            draw_calls: 240,
            loaded_chunks: 25,
            script_ms: 2.4,
            physics_ms: 2.1,
            ui_ms: 0.8,
            memory_mb: 640.0,
        },
        "object_pool": pool,
        "spawn_director": SpawnDirector2D {
            max_spawn_per_tick: 4,
            rules: vec![SpawnRule2D {
                prefab: "assets/prefabs/enemy.prefab".to_string(),
                tag: "Enemy".to_string(),
                min_distance_from_camera: 12.0,
                max_distance_from_camera: 24.0,
                max_alive: 80,
                weight: 1.0,
                cooldown_frames: 30,
                last_spawn_frame: 0,
            }],
        },
        "save_shards": saves.flush_plan(),
    })
}

fn asset_registry_example() -> Value {
    json!({
        "assets": [
            {"guid": "asset-player-sprite", "path": "assets/sprites/player.png", "type": "Sprite2D"},
            {"guid": "graph-player", "path": "scripts/visual_graphs/BP_PlayerPawn2D.mfgraph", "type": "BlueprintGraph2D"}
        ],
        "lookup_keys": ["guid", "path", "label", "type"]
    })
}

fn plugin_system_example() -> Value {
    json!({
        "plugins": [
            {"name": "Paper2DExtras", "enabled": true, "version": "0.1.0", "modules": ["tileset_importer", "flipbook_tools"]}
        ]
    })
}

fn pie_example() -> Value {
    json!({
        "mode": "PlayInEditor",
        "snapshot_scene_before_play": true,
        "restore_on_stop": true,
        "start_paused": false,
        "simulate_controllers": true,
        "runtime": "macroquad+luau"
    })
}
