use std::path::PathBuf;

use serde_json::json;

use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/NewProject_5")
}

#[test]
fn neon_sombra_dispatch_activates_roadblocks_and_vehicle_pursuit() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Neon Sombra runtime");
    runtime.run_headless_once(1.0 / 60.0);
    assert!(runtime.runtime_world.validate().is_valid());
    assert!(
        runtime.luau_script_runtime.last_errors.is_empty(),
        "{:?}",
        runtime.luau_script_runtime.last_errors
    );

    for _ in 0..3 {
        let report = runtime.luau_script_runtime.run_custom_event(
            &mut runtime.runtime_world.units,
            "crime_committed",
            json!({
                "kind": "test_dispatch_escalation",
                "wanted": 5,
                "x": 80.0,
                "y": 8.0,
            }),
        );
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    for _ in 0..10 {
        runtime.run_headless_once(1.0 / 60.0);
    }

    let active_roadblocks = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.name.starts_with("Roadblock_") && entity.enabled && entity.visible)
        .count();
    assert!(
        active_roadblocks >= 3,
        "expected tier-3 roadblocks, got {active_roadblocks}"
    );

    let police_vehicle_states = runtime
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.name.starts_with("PoliceCar_"))
        .filter_map(|entity| {
            entity
                .get_component("Blackboard")?
                .get("values")?
                .get("state")?
                .as_str()
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();
    assert!(
        police_vehicle_states
            .iter()
            .any(|state| state == "search" || state == "pursuit"),
        "expected a police vehicle to search or pursue, got {police_vehicle_states:?}"
    );
    assert!(runtime.luau_script_runtime.last_errors.is_empty());
}
