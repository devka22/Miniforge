use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use miniforge::engine::component::default_component;
use miniforge::engine::component_registry::ComponentRegistry;
use miniforge::engine::game_api::GameAPI;
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::entities::game_object::GameObject;
use miniforge::systems::physics_system::{BoxCastQuery, PhysicsQueryFilter, PhysicsSystem};

fn temp_project(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-{name}-{stamp}"));
    fs::create_dir_all(path.join("scripts")).unwrap();
    path
}

#[test]
fn new_2d_components_are_editor_creatable() {
    let registry = ComponentRegistry::new();
    for component in [
        "CharacterBody2D",
        "Area2D",
        "OneWayPlatform2D",
        "AnimatedSprite",
        "AnimationPlayer",
        "Tilemap2D",
        "TilemapChunk2D",
        "InputActions2D",
        "EventBus2D",
    ] {
        let descriptor = registry.descriptor(component).expect(component);
        assert!(descriptor.creatable, "{component} should be creatable");
        assert!(default_component(component).is_some());
    }
}

#[test]
fn physics_supports_shape_cast_overlap_area_and_area2d() {
    let mut enemy = GameObject::new(3.0, 0.0, Some("Enemy".to_string()));
    enemy.layer = "Enemy".to_string();
    if let Some(collider) = enemy.get_component_mut("Collider2D") {
        collider.set("collision_layer", json!("Enemy"));
    }

    let mut trigger = GameObject::new(0.0, 2.0, Some("Trigger".to_string()));
    trigger.remove_component("Collider2D");
    trigger.add_component(default_component("Area2D").unwrap());
    if let Some(area) = trigger.get_component_mut("Area2D") {
        area.set("collision_layer", json!("Trigger"));
        area.set_f64("width", 2.0);
        area.set_f64("height", 2.0);
    }

    let physics = PhysicsSystem::new();
    let layers = vec!["Enemy".to_string()];
    let hit = physics
        .box_cast_filtered(
            &[enemy.clone()],
            BoxCastQuery {
                origin: (0.0, 0.0),
                half_extents: (0.25, 0.25),
                direction: (1.0, 0.0),
                max_distance: 8.0,
                filter: PhysicsQueryFilter {
                    include_triggers: false,
                    layers: Some(&layers),
                },
            },
        )
        .expect("box cast should hit enemy");
    assert_eq!(hit.entity_name, "Enemy");

    let trigger_layers = vec!["Trigger".to_string()];
    let overlaps = physics.overlap_area_filtered(
        &[trigger],
        (0.0, 2.0),
        (1.0, 1.0),
        true,
        Some(&trigger_layers),
    );
    assert_eq!(overlaps.len(), 1);
    assert!(overlaps[0].is_trigger);
}

#[test]
fn luau_public_2d_api_drives_runtime_components() {
    let project = temp_project("luau-2d-api");
    fs::write(
        project.join("scripts").join("Player.luau"),
        r#"
local Player = {}

function Player:on_start()
    Camera.main():follow(self.entity)
    Camera.main():shake(0.3, 6)
    Component.add(self.entity, "Tilemap2D", {
        component_type = "Tilemap2D",
        width = 4,
        height = 4,
        layers = {{ name = "Collision", tiles = {} }}
    })
    Tilemap.set_tile(self.entity, "Collision", 1, 1, 7)
    AnimationPlayer.play(self.entity, "Run")
    AnimationPlayer.set_parameter(self.entity, "moving", true)
    Entity.spawn("Projectile", 2.0, 0.0, {
        tag = "Projectile",
        script = "Projectile.luau",
        components = {
            { component_type = "Rigidbody2D", body_type = "kinematic", velocity_x = 9.0, use_gravity = false }
        }
    })
    Task.delay(0.1, function()
        Events.emit("delayed", { ok = true })
    end)
end

function Player:on_update(dt)
    local hit = Physics2D.raycast(Vector2.new(0, 0), Vector2.new(5, 0), {
        mask = Layers.ENEMY,
        include_triggers = true
    })
    if hit then
        set_blackboard("seen", hit.name)
    end
    local enemy = Entity.find("Enemy")
    if enemy then
        set_blackboard("enemy_id", enemy.id)
    end
    set_blackboard("enemy_health", Component.get("Enemy", "Health", "health", 0))
    local nearby = Entity.nearby(self.entity, 6.0, { tag = "Enemy" })
    if #nearby > 0 then
        set_blackboard("nearby_enemy", nearby[1].name)
    end
end

function Player:on_event(name, payload)
    if name == "delayed" then
        set_blackboard("delayed", "yes")
    end
end

return Player
"#,
    )
    .unwrap();
    fs::write(
        project.join("scripts").join("Projectile.luau"),
        "function on_start() end",
    )
    .unwrap();

    let mut player = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    player.script = Some("Player.luau".to_string());
    let mut enemy = GameObject::new(3.0, 0.0, Some("Enemy".to_string()));
    enemy.tag = "Enemy".to_string();
    enemy.layer = "Enemy".to_string();
    enemy.add_component(default_component("Health").unwrap());
    if let Some(health) = enemy.get_component_mut("Health") {
        health.set_f64("health", 35.0);
    }
    if let Some(collider) = enemy.get_component_mut("Collider2D") {
        collider.set("collision_layer", json!("Enemy"));
    }
    let enemy_id = enemy.id;

    let mut entities = vec![player, enemy];
    let mut runtime = LuauScriptRuntime::new(&project);
    let first = runtime.update_entities(&mut entities, 0.05, "PLAY");
    assert!(first.errors.is_empty(), "{:?}", first.errors);
    let second = runtime.update_entities(&mut entities, 0.1, "PLAY");
    assert!(second.errors.is_empty(), "{:?}", second.errors);

    let player = entities
        .iter()
        .find(|entity| entity.name == "Player")
        .unwrap();
    assert!(player.get_component("CameraFollow").is_some());
    assert!(player.get_component("CameraShake").is_some());
    assert_eq!(
        GameAPI::get_blackboard(player, "seen", json!("")),
        json!("Enemy")
    );
    assert_eq!(
        GameAPI::get_blackboard(player, "enemy_id", json!(0)),
        json!(enemy_id)
    );
    assert_eq!(
        GameAPI::get_blackboard(player, "enemy_health", json!(0)),
        json!(35)
    );
    assert_eq!(
        GameAPI::get_blackboard(player, "nearby_enemy", json!("")),
        json!("Enemy")
    );
    assert_eq!(
        GameAPI::get_blackboard(player, "delayed", json!("")),
        json!("yes")
    );
    assert_eq!(
        player
            .get_component("AnimationPlayer")
            .unwrap()
            .get("current")
            .and_then(|value| value.as_str()),
        Some("Run")
    );
    assert_eq!(tile_at(player, "Collision", 1, 1), Some(7));

    let projectile = entities
        .iter()
        .find(|entity| entity.name == "Projectile")
        .expect("configured spawn");
    assert_eq!(projectile.tag, "Projectile");
    assert_eq!(projectile.script.as_deref(), Some("Projectile.luau"));
    assert!(projectile.get_component("Rigidbody2D").is_some());
}

#[test]
fn luau_utility_queries_spawn_handles_and_component_removal_work_together() {
    let project = temp_project("luau-productivity-api");
    fs::write(
        project.join("scripts").join("Utility.luau"),
        r#"
local Utility = {}

function Utility:on_start()
    local next_position = Vector2.move_towards(
        Vector2.new(0, 0),
        Vector2.new(10, 0),
        2.5
    )
    set_blackboard("next_x", next_position.x)
    set_blackboard("distance", Vector2.distance(next_position, Vector2.new(10, 0)))

    local nearest = Entity.nearest(self.entity, 20.0, { tag = "Enemy" })
    set_blackboard("enemy_exists", Entity.exists("EnemyNear"))
    set_blackboard("enemy_count", Entity.count_with_tag("Enemy"))
    if nearest then
        set_blackboard("nearest", nearest.name)
    end

    Component.remove(self.entity, "Collider2D")
    local projectile = Spawner.spawn("UtilityProjectile", 1.0, 2.0)
    Rigidbody2D.set_velocity(projectile, 12.0, -3.0)
end

return Utility
"#,
    )
    .unwrap();

    let mut actor = GameObject::new(0.0, 0.0, Some("UtilityActor".to_string()));
    actor.script = Some("Utility.luau".to_string());
    let mut near = GameObject::new(3.0, 0.0, Some("EnemyNear".to_string()));
    near.tag = "Enemy".to_string();
    let mut far = GameObject::new(9.0, 0.0, Some("EnemyFar".to_string()));
    far.tag = "Enemy".to_string();
    let existing_projectile = GameObject::new(-2.0, 0.0, Some("UtilityProjectile".to_string()));
    let existing_projectile_id = existing_projectile.id;
    let mut entities = vec![actor, near, far, existing_projectile];

    let mut runtime = LuauScriptRuntime::new(&project);
    let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let actor = entities
        .iter()
        .find(|entity| entity.name == "UtilityActor")
        .unwrap();
    assert_eq!(
        GameAPI::get_blackboard(actor, "next_x", json!(0)),
        json!(2.5)
    );
    assert_eq!(
        GameAPI::get_blackboard(actor, "distance", json!(0)),
        json!(7.5)
    );
    assert_eq!(
        GameAPI::get_blackboard(actor, "nearest", json!("")),
        json!("EnemyNear")
    );
    assert_eq!(
        GameAPI::get_blackboard(actor, "enemy_exists", json!(false)),
        json!(true)
    );
    assert_eq!(
        GameAPI::get_blackboard(actor, "enemy_count", json!(0)),
        json!(2)
    );
    assert!(actor.get_component("Collider2D").is_none());

    let spawned_id = *report.spawned.last().expect("reserved spawn id");
    let projectile = entities
        .iter()
        .find(|entity| entity.id == spawned_id)
        .expect("spawn handle should target the queued entity");
    assert_eq!(projectile.name, "UtilityProjectile");
    assert_ne!(projectile.id, existing_projectile_id);
    let body = projectile.get_component("Rigidbody2D").unwrap();
    assert_eq!(body.get_f64("velocity_x", 0.0), 12.0);
    assert_eq!(body.get_f64("velocity_y", 0.0), -3.0);

    fs::remove_dir_all(project).unwrap();
}

fn tile_at(entity: &GameObject, layer: &str, x: usize, y: usize) -> Option<i64> {
    let tilemap = entity.get_component("Tilemap2D")?;
    let width = tilemap.get_usize("width", 0);
    let index = y * width + x;
    tilemap
        .get("layers")
        .and_then(|value| value.as_array())?
        .iter()
        .find(|entry| entry.get("name").and_then(|value| value.as_str()) == Some(layer))?
        .get("tiles")
        .and_then(|value| value.as_array())?
        .get(index)?
        .as_i64()
}
