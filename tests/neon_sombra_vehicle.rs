use std::path::PathBuf;

use serde_json::json;

use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/NewProject_5")
}

#[test]
fn neon_sombra_drivable_vehicle_accepts_player_control() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Neon Sombra runtime");
    runtime.run_headless_once(1.0 / 60.0);
    assert!(runtime.runtime_world.validate().is_valid());
    assert!(
        runtime.luau_script_runtime.last_errors.is_empty(),
        "{:?}",
        runtime.luau_script_runtime.last_errors
    );

    let (vehicle_id, start_x, start_y) = runtime
        .runtime_world
        .units
        .iter()
        .find_map(|entity| {
            let values = entity
                .get_component("Blackboard")?
                .get("values")?
                .as_object()?;
            values
                .get("drivable")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                .then_some((entity.id, entity.x, entity.y))
        })
        .expect("drivable vehicle");
    let player_id = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.tag == "Player")
        .map(|entity| entity.id)
        .expect("player");

    let report = runtime.luau_script_runtime.run_custom_event(
        &mut runtime.runtime_world.units,
        "vehicle_entered",
        json!({
            "vehicle_id": vehicle_id,
            "vehicle_tag": "Vehicle",
            "player_id": player_id,
        }),
    );
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    runtime.set_script_input_pressed("D", true);
    runtime.set_script_input_pressed("Shift", true);
    for _ in 0..8 {
        runtime.run_headless_once(1.0 / 60.0);
    }
    runtime.set_script_input_pressed("D", false);
    runtime.set_script_input_pressed("Shift", false);

    let vehicle = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.id == vehicle_id)
        .expect("vehicle still exists");
    assert!(
        vehicle.x > start_x + 0.05,
        "vehicle did not move right enough: start=({start_x},{start_y}) now=({},{})",
        vehicle.x,
        vehicle.y
    );
    assert!(runtime.luau_script_runtime.last_errors.is_empty());
}
