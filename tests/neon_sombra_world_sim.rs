use std::path::PathBuf;

use serde_json::Value;

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
fn neon_sombra_world_simulation_publishes_weather_and_beach_state() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Neon Sombra runtime");
    runtime.run_headless_once(1.0 / 60.0);

    assert!(runtime.runtime_world.validate().is_valid());
    assert!(
        runtime.luau_script_runtime.last_errors.is_empty(),
        "{:?}",
        runtime.luau_script_runtime.last_errors
    );

    let backdrop = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "City_Backdrop")
        .expect("city backdrop");
    assert_eq!(backdrop.width.round() as i32, 128);
    assert_eq!(backdrop.height.round() as i32, 80);
    assert!(
        runtime
            .runtime_world
            .units
            .iter()
            .any(|entity| entity.name == "Luz_Contact")
    );
    assert!(
        runtime
            .runtime_world
            .units
            .iter()
            .filter(|entity| entity.name.starts_with("Yacht_"))
            .count()
            >= 3
    );

    assert!(
        blackboard_value(&runtime, "CityDirector", "weather")
            .and_then(Value::as_str)
            .is_some()
    );
    assert!(
        blackboard_value(&runtime, "CityDirector", "phase")
            .and_then(Value::as_str)
            .is_some()
    );
    assert_eq!(
        blackboard_value(&runtime, "CityDirector", "headlights").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        ui_text(&runtime, "HUD_Weather")
            .as_deref()
            .unwrap_or_default()
            .contains('/')
    );

    let car_light = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "TrafficCar_01")
        .and_then(|entity| entity.get_component("Light2D"))
        .and_then(|light| light.get("intensity"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    assert!(
        car_light > 0.5,
        "expected night headlights, got {car_light}"
    );
}
