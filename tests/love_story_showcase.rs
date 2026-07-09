use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::runtime_exporter::{ExportProfile, RuntimeExporter};
use miniforge::engine::runtime_manifest_loader::RuntimeManifestLoader;
use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/MCP_LoveStoryLab")
}

fn temp_export_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "miniforge-love-story-export-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn love_story_lab_boots_on_isolated_runtime_with_connected_systems() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Love Story runtime");
    for _ in 0..3 {
        runtime.run_headless_once(1.0 / 60.0);
    }

    assert!(runtime.runtime_world.validate().is_valid());
    assert_eq!(runtime.grid.width, 54);
    assert_eq!(runtime.grid.height, 32);
    assert_eq!(runtime.visual_script_runtime.last_frame_graphs, 1);
    assert_eq!(
        runtime
            .sprite_animation_system
            .last_report
            .animated_entities,
        3
    );
    assert!(
        runtime
            .sprite_animation_system
            .last_report
            .errors
            .is_empty()
    );
    assert!(!runtime.audio_system.voices.is_empty());
    assert!(runtime.luau_script_runtime.last_errors.is_empty());
}

#[test]
fn love_story_dialogue_choice_updates_ui_quest_and_affection() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Love Story runtime");
    runtime.run_headless_once(1.0 / 60.0);
    let mara_position = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "Mara")
        .map(|entity| (entity.x, entity.y))
        .expect("Mara");
    let sol = runtime
        .runtime_world
        .units
        .iter_mut()
        .find(|entity| entity.name == "Sol")
        .expect("Sol");
    sol.x = mara_position.0 - 1.0;
    sol.y = mara_position.1;
    sol.sync_to_components();

    let dialogue = runtime.interact().expect("dialogue interaction");
    assert_eq!(dialogue.speaker, "Mara");
    assert!(dialogue.quest_updated);
    let choice = runtime.choose_dialogue(0).expect("first dialogue choice");
    assert_eq!(choice.text, "Tell her the truth");

    let director = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "StoryDirector")
        .expect("StoryDirector");
    let affection = director
        .get_component("Blackboard")
        .and_then(|blackboard| blackboard.get("values"))
        .and_then(serde_json::Value::as_object)
        .and_then(|values| values.get("affection"))
        .and_then(serde_json::Value::as_f64)
        .expect("affection score");
    assert_eq!(affection, 50.0);

    let hud = runtime
        .runtime_world
        .units
        .iter()
        .find(|entity| entity.name == "HUD_Dialogue")
        .and_then(|entity| entity.get_component("UIElement"))
        .expect("dialogue HUD");
    let dialogue_text = hud.get_string("text", "");
    assert!(
        dialogue_text.contains("Tell her the truth"),
        "unexpected dialogue HUD text: {dialogue_text:?}"
    );
}

#[test]
fn love_story_release_export_is_self_contained() {
    let output_root = temp_export_root();
    let report =
        RuntimeExporter::export_with_profile(project_path(), &output_root, ExportProfile::Release)
            .expect("Love Story release export");

    assert!(report.validation_errors.is_empty());
    assert!(report.missing_assets.is_empty());
    assert!(report.readiness_score >= 90);
    assert!(!report.output_path.join(".miniforge").exists());
    assert!(!report.output_path.join("README.md").exists());
    assert!(
        !report
            .output_path
            .join("settings/editor_layout.json")
            .exists()
    );
    assert!(
        !report
            .output_path
            .join("settings/build_profiles.json")
            .exists()
    );
    assert!(
        !report
            .output_path
            .join("project/project_state.json")
            .exists()
    );
    assert!(
        !report
            .used_assets
            .iter()
            .any(|asset| asset.ends_with(".bak"))
    );
    for asset in [
        "assets/sprites/RainyStationBackdrop-v2.png",
        "assets/sprites/LoveLabCharacters.png",
        "assets/animations/LoveLabCharacters.spriteframes",
        "assets/ui/hud.ui2d.json",
    ] {
        assert!(
            report.output_path.join(asset).is_file(),
            "missing exported asset: {asset}"
        );
        assert!(
            report.used_assets.iter().any(|used| used == asset),
            "asset absent from runtime manifest: {asset}"
        );
    }

    let missing = RuntimeManifestLoader::validate_tree(&report.output_path)
        .expect("validate exported runtime tree");
    assert!(
        missing.is_empty(),
        "exported runtime is incomplete: {missing:?}"
    );
    let mut exported_runtime =
        EngineRuntime::new(&report.output_path).expect("boot exported Love Story runtime");
    exported_runtime.run_headless_once(1.0 / 60.0);
    assert!(exported_runtime.runtime_world.validate().is_valid());
    assert!(exported_runtime.luau_script_runtime.last_errors.is_empty());
    assert!(!report.output_path.join("templates").exists());
    assert!(!report.output_path.join("builds").exists());
    assert!(
        !report
            .output_path
            .join("project/project_state.json")
            .exists()
    );
    std::fs::remove_dir_all(output_root).expect("remove test export");
}
