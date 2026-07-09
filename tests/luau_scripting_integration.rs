use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::component::default_component;
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::entities::game_object::GameObject;
use serde_json::json;

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-{label}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn luau_returned_table_require_exports_and_transform_proxy_work() {
    let root = temp_dir("luau-table-api");
    AssetTools::ensure_project_folders(&root).unwrap();
    fs::write(
        root.join("scripts").join("Scale.luau"),
        r#"
local Scale = {}
function Scale.by(axis, speed)
    return axis * speed
end
return Scale
"#,
    )
    .unwrap();
    fs::write(
        root.join("scripts").join("Player.luau"),
        r#"
local Scale = require("./Scale")
local Player = {}

function Player:on_create()
    self.speed = self.speed or 220
    self.created = true
end

function Player:on_ready()
    self.ready = true
end

function Player:on_fixed_update(dt)
    self.fixed = (self.fixed or 0) + 1
end

function Player:on_update(dt)
    local direction = Input.get_axis("move_left", "move_right")
    self.entity.transform.position.x += Scale.by(direction, self.speed) * dt
    Debug.log("player moved")
end

return Player
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    let mut script = default_component("ScriptComponent").unwrap();
    script.set("path", json!("Player.luau"));
    entity.add_component(script);
    let mut entities = vec![entity];

    let mut runtime = LuauScriptRuntime::new(&root);
    runtime.set_input_pressed("move_right", true);
    let report =
        runtime.update_entities_with_fixed_steps(&mut entities, 0.1, 1.0 / 60.0, 2, "PLAY");

    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert!(report.scripts_run >= 5);
    assert!(
        report
            .debug_messages
            .iter()
            .any(|msg| msg.contains("player moved"))
    );
    assert!((entities[0].x - 22.0).abs() < 0.0001);

    let public_variables = entities[0]
        .get_component("ScriptComponent")
        .unwrap()
        .get("public_variables")
        .unwrap();
    assert_eq!(public_variables["speed"].as_f64(), Some(220.0));
    assert_eq!(public_variables["created"].as_bool(), Some(true));
    assert_eq!(public_variables["ready"].as_bool(), Some(true));
    assert_eq!(public_variables["fixed"].as_i64(), Some(2));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn luau_multiple_scripts_on_one_entity_run_independently() {
    let root = temp_dir("luau-multiple-scripts");
    AssetTools::ensure_project_folders(&root).unwrap();
    fs::write(
        root.join("scripts").join("MoveX.luau"),
        r#"
local MoveX = {}
function MoveX:on_update(dt)
    self.entity.transform.position.x += 3
end
return MoveX
"#,
    )
    .unwrap();
    fs::write(
        root.join("scripts").join("MoveY.luau"),
        r#"
local MoveY = {}
function MoveY:on_update(dt)
    self.entity.transform.position.y += 4
end
return MoveY
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("Actor".to_string()));
    entity.scripts = vec![
        json!({"runtime": "luau", "path": "MoveX.luau"}),
        json!({"runtime": "luau", "path": "MoveY.luau"}),
    ];
    let mut entities = vec![entity];

    let mut runtime = LuauScriptRuntime::new(&root);
    let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");

    assert!(report.errors.is_empty(), "{:?}", report.errors);
    assert_eq!(entities[0].x, 3.0);
    assert_eq!(entities[0].y, 4.0);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn luau_blocked_script_is_interrupted_and_reported() {
    let root = temp_dir("luau-blocked-script");
    AssetTools::ensure_project_folders(&root).unwrap();
    fs::write(
        root.join("scripts").join("Hang.luau"),
        r#"
local Hang = {}
function Hang:on_update(dt)
    while true do
    end
end
return Hang
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("Hang".to_string()));
    entity.scripts = vec![json!({"runtime": "luau", "path": "Hang.luau"})];
    let mut entities = vec![entity];

    let mut runtime = LuauScriptRuntime::new(&root);
    let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");

    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("script execution budget exceeded")),
        "{:?}",
        report.errors
    );

    fs::remove_dir_all(root).unwrap();
}
