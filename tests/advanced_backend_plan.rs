use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::engine_backend::EngineBackend;
use miniforge::engine::resource_manager::{ResourceKind, ResourceManager};
use miniforge::engine::runtime_exporter::{ExportProfile, RuntimeExporter};
use miniforge::engine::system_audit::SystemReadinessReport;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-backend-plan-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn write_complex_project_fixture(tmp: &PathBuf) {
    let paths = AssetTools::ensure_project_folders(tmp).unwrap();
    fs::write(paths.sprites.join("hero.png"), b"png").unwrap();
    fs::write(paths.audio.join("hit.wav"), b"wav").unwrap();
    fs::write(paths.data.join("items.json"), r#"{"items":[]}"#).unwrap();
    fs::write(
        paths.prefabs.join("player.prefab"),
        r#"{"entity":{"name":"Player","components":[]}}"#,
    )
    .unwrap();
    fs::write(paths.scripts.join("Player.luau"), "function on_start() end").unwrap();
    fs::write(
        paths.visual_graphs.join("Combat.mfgraph"),
        r#"{"runtime":"rust_visual_graph","nodes":[{"id":"start","type":"EventStart"}]}"#,
    )
    .unwrap();
    fs::write(
        paths.scenes.join("main.scene"),
        r#"{"version":"0.9.2","entities":[]}"#,
    )
    .unwrap();
    AssetTools::write_json(
        paths.settings.join("runtime_config.json"),
        &json!({
            "target_fps": 120,
            "fixed_timestep": 0.008333333,
            "max_frame_steps": 6,
            "max_entities": 20000,
            "max_particles": 100000,
            "streaming_enabled": true,
            "asset_hot_reload": true
        }),
    )
    .unwrap();

    let core = tmp.join("plugins").join("CoreTools");
    let combat = tmp.join("plugins").join("CombatTools");
    let disabled = tmp.join("plugins").join("DisabledBroken");
    fs::create_dir_all(&core).unwrap();
    fs::create_dir_all(&combat).unwrap();
    fs::create_dir_all(&disabled).unwrap();
    AssetTools::write_json(
        core.join("plugin.json"),
        &json!({
            "name": "CoreTools",
            "version": "1.0.0",
            "author": "MiniForge",
            "enabled": true,
            "description": "Core backend extension",
            "min_engine_version": "0.9.2",
            "hooks": ["on_editor_start"],
            "services": ["CoreToolService"],
            "systems": ["SaveGame"]
        }),
    )
    .unwrap();
    AssetTools::write_json(
        combat.join("plugin.json"),
        &json!({
            "name": "CombatTools",
            "version": "1.0.0",
            "author": "MiniForge",
            "enabled": true,
            "description": "Combat backend extension",
            "min_engine_version": "0.9.2",
            "dependencies": ["CoreTools"],
            "hooks": {"on_play_start": "scripts/Combat.luau"},
            "components": ["DamageVolume2D"],
            "systems": ["Combat"],
            "runtime_features": ["AbilityCombos"]
        }),
    )
    .unwrap();
    AssetTools::write_json(
        disabled.join("plugin.json"),
        &json!({
            "name": "DisabledBroken",
            "version": "1.0.0",
            "author": "MiniForge",
            "enabled": false,
            "description": "Disabled broken plugin",
            "min_engine_version": "0.9.2",
            "dependencies": ["MissingPlugin"]
        }),
    )
    .unwrap();
}

#[test]
fn backend_plan_connects_services_plugins_resources_and_runtime_tuning() {
    let tmp = temp_dir("complex");
    write_complex_project_fixture(&tmp);

    let plan = EngineBackend::plan_project(&tmp).unwrap();
    assert!(plan.editor_ready);
    assert!(plan.runtime_ready);
    assert!(plan.export_ready);
    assert!(plan.complex_game_ready());
    assert_eq!(
        plan.service_startup_order.first().unwrap(),
        "DiagnosticsService"
    );
    assert_eq!(plan.plugins.load_order, vec!["CoreTools", "CombatTools"]);
    assert!(
        plan.plugins
            .hooks
            .get("on_play_start")
            .unwrap()
            .contains(&"CombatTools".to_string())
    );
    assert!(
        plan.plugins
            .capabilities
            .components
            .contains_key("DamageVolume2D")
    );
    assert_eq!(plan.resources.counts["image"], 1);
    assert_eq!(plan.resources.counts["script"], 1);
    assert_eq!(plan.resources.counts["visual_graph"], 1);
    assert_eq!(plan.runtime_tuning.max_entities, 20000);
    assert!(
        plan.feature_modules
            .iter()
            .any(|module| module.name == "Massive World 2D")
    );
    assert!(plan.system_audit.total_score >= 60);
    assert!(plan.system_audit.areas.contains_key("Scripting"));
    assert!(plan.system_audit.areas.contains_key("Packaging"));
    assert!(
        plan.recommendations
            .iter()
            .any(|item| item.starts_with("System audit:"))
    );

    let resources = ResourceManager::scan_project_resources(&tmp).unwrap();
    assert!(resources.find(ResourceKind::Image, "hero").is_some());
    assert!(
        resources
            .find(ResourceKind::VisualGraph, "Combat")
            .is_some()
    );
}

#[test]
fn runtime_export_manifest_embeds_backend_plan_for_player_startup() {
    let tmp = temp_dir("export");
    write_complex_project_fixture(&tmp);

    let report =
        RuntimeExporter::export_with_profile(&tmp, tmp.join("exports"), ExportProfile::Release)
            .unwrap();
    let manifest = AssetTools::read_json(&report.manifest_path).unwrap();
    let build_info = AssetTools::read_json(report.output_path.join("build_info.json")).unwrap();
    assert_eq!(
        manifest["backend_plan"]["plugins"]["load_order"][1],
        "CombatTools"
    );
    assert_eq!(
        manifest["backend_plan"]["resources"]["counts"]["visual_graph"],
        1
    );
    assert_eq!(
        manifest["backend_plan"]["runtime_tuning"]["streaming_enabled"],
        true
    );
    assert_eq!(
        manifest["backend_plan"]["system_audit"]["areas"]["Scripting"]["system"],
        "Scripting"
    );
    assert!(report.readiness_score >= 60);
    assert!(build_info["readiness_score"].as_u64().unwrap() >= 60);
    assert!(build_info["readiness_actions"].as_array().is_some());
}

#[test]
fn system_audit_turns_loose_project_gaps_into_next_pass_actions() {
    let tmp = temp_dir("audit-gaps");
    AssetTools::ensure_project_folders(&tmp).unwrap();
    fs::write(tmp.join("project.json"), "{}").unwrap();

    let audit = SystemReadinessReport::audit_project(&tmp).unwrap();
    assert!(audit.areas.contains_key("Assets"));
    assert!(audit.areas.contains_key("UI"));
    assert!(audit.total_score < 100);
    assert!(
        audit
            .next_pass_backlog
            .iter()
            .any(|item| item.contains("crear assets minimos") || item.contains("crear HUDScreen"))
    );
    assert!(audit.concise_summary().contains("Readiness"));
}
