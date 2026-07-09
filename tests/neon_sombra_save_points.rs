use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/NewProject_5")
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

fn blackboard_values<'a>(runtime: &'a EngineRuntime, name: &str) -> Option<&'a Value> {
    runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == name)?
        .get_component("Blackboard")?
        .get("values")
}

#[test]
fn neon_sombra_save_point_writes_autosave_slot() {
    let project = project_path();
    let save_path = project.join("saves/profile/autosave.json");
    let _ = fs::remove_file(&save_path);

    let mut runtime = EngineRuntime::new(&project).expect("Neon Sombra runtime");
    {
        let player = runtime
            .runtime_world
            .units
            .iter_mut()
            .find(|entity| entity.name == "Player")
            .expect("player");
        player.x = 13.0;
        player.y = 49.0;
        player.sync_to_components();
    }
    runtime.set_script_input_pressed("E", true);
    runtime.set_script_input_pressed("interact", true);
    for _ in 0..4 {
        runtime.run_headless_once(1.0 / 60.0);
    }
    runtime.set_script_input_pressed("E", false);
    runtime.set_script_input_pressed("interact", false);

    assert!(
        save_path.exists(),
        "expected autosave at {}, prompt={:?}, save_point={:?}, errors={:?}",
        save_path.display(),
        ui_text(&runtime, "HUD_Prompt"),
        blackboard_values(&runtime, "SavePoint_Muelle"),
        runtime.luau_script_runtime.last_errors
    );
    let save = fs::read_to_string(&save_path).expect("read autosave");
    assert!(save.contains("\"kind\": \"MiniForgeSaveGame\""));
    assert!(save.contains("\"save_key\": \"player\""));
    assert!(save.contains("\"save_key\": \"city_director\""));
    assert!(runtime.luau_script_runtime.last_errors.is_empty());

    let _ = fs::remove_file(save_path);
}
