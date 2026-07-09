use std::path::PathBuf;

use serde_json::{Value, json};

use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/NewProject_5")
}

fn blackboard_value<'a>(runtime: &'a EngineRuntime, name: &str, key: &str) -> Option<&'a Value> {
    runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == name)?
        .get_component("Blackboard")?
        .get("values")?
        .get(key)
}

fn ui_text(runtime: &EngineRuntime, name: &str) -> Option<String> {
    runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == name)?
        .get_component("UIElement")?
        .get("text")?
        .as_str()
        .map(ToString::to_string)
}

#[test]
fn neon_sombra_city_life_tracks_story_reputation_and_npc_panic() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Neon Sombra runtime");
    runtime.run_headless_once(1.0 / 60.0);
    assert!(runtime.runtime_world.validate().is_valid());
    assert!(
        runtime.luau_script_runtime.last_errors.is_empty(),
        "{:?}",
        runtime.luau_script_runtime.last_errors
    );
    assert_eq!(
        ui_text(&runtime, "HUD_District").as_deref(),
        Some("Bajo Muelle")
    );
    assert_eq!(
        ui_text(&runtime, "HUD_Reputation").as_deref(),
        Some("Rep 0")
    );

    let report = runtime.luau_script_runtime.run_custom_event(
        &mut runtime.runtime_world.units,
        "contact_interact",
        json!({
            "contact_name": "Mara_Contact",
            "distance": 0.5,
        }),
    );
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for _ in 0..4 {
        runtime.run_headless_once(1.0 / 60.0);
    }

    assert_eq!(
        blackboard_value(&runtime, "CityDirector", "mission_index").and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        blackboard_value(&runtime, "CityDirector", "reputation").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        ui_text(&runtime, "HUD_Reputation").as_deref(),
        Some("Rep 1")
    );

    let report = runtime.luau_script_runtime.run_custom_event(
        &mut runtime.runtime_world.units,
        "crime_committed",
        json!({
            "kind": "city_life_test",
            "wanted": 4,
            "x": 13.0,
            "y": 47.0,
        }),
    );
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    for _ in 0..20 {
        runtime.run_headless_once(1.0 / 60.0);
    }

    let reactive_npcs = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.name.starts_with("NPC_"))
        .filter_map(|entity| {
            entity
                .get_component("Blackboard")?
                .get("values")?
                .get("mood")?
                .as_str()
        })
        .filter(|mood| matches!(*mood, "panic" | "witness" | "uneasy"))
        .count();
    assert!(
        reactive_npcs > 0,
        "expected nearby NPCs to react to the crime event"
    );
    assert!(runtime.luau_script_runtime.last_errors.is_empty());
}
