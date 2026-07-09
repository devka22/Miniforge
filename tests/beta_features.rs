//! Beta technical tests: runtime manifest, incremental scene save, UI canvas, importers, packaging.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_importers::{SpriteSheetImporter, WaveformCache};
use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::camera::Camera;
use miniforge::engine::packaging_manager::PackagingManager;
use miniforge::engine::profiler::Profiler;
use miniforge::engine::runtime_exporter::ExportProfile;
use miniforge::engine::runtime_manifest_loader::RuntimeManifestLoader;
use miniforge::engine::scene_manager::SceneManager;
use miniforge::engine::scene_save_manager::SceneSaveManager;
use miniforge::engine::tilemap_layers::TilemapLayers;
use miniforge::engine::ui_canvas::{UiCanvasRoot, ui_canvases_from_value};
use miniforge::entities::game_object::GameObject;
use miniforge::map::grid::Grid;
use serde_json::json;

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-beta-{name}-{stamp}"));
    fs::create_dir_all(&path).unwrap();
    path
}

/// Minimal valid 1×1 PNG (IHDR + IDAT + IEND).
const MIN_PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

#[test]
fn runtime_manifest_loader_reads_export() {
    let root = temp_dir("manifest");
    AssetTools::write_json(
        root.join("runtime_manifest.json"),
        &json!({
            "engine_version": "0.7.0",
            "profile": "debug",
            "runtime": "rust",
            "used_assets": ["assets/missing.png"],
            "missing_assets": ["assets/missing.png"],
            "source_manifest": {"assets": []}
        }),
    )
    .unwrap();
    AssetTools::write_json(
        root.join("build_info.json"),
        &json!({
            "engine_version": "0.7.0",
            "runtime": "rust",
            "profile": "debug",
            "copied_files": 1,
            "missing_assets": 1
        }),
    )
    .unwrap();
    let loaded = RuntimeManifestLoader::load(&root).unwrap();
    assert_eq!(loaded.profile, "debug");
    assert!(!loaded.validated_missing.is_empty());
}

#[test]
fn scene_save_manager_incremental_merge() {
    let proj = temp_dir("scene_save");
    let scenes = proj.join("scenes");
    fs::create_dir_all(&scenes).unwrap();
    fs::create_dir_all(proj.join("project")).unwrap();
    AssetTools::write_json(proj.join("project").join("project.json"), &json!({})).unwrap();

    let mut sm = SceneManager::new(&proj);
    sm.current_scene = "test.scene".to_string();

    let mut a = GameObject::new(1.0, 1.0, Some("A".to_string()));
    let id_a = a.id;
    let mut b = GameObject::new(2.0, 2.0, Some("B".to_string()));
    let id_b = b.id;
    a.sync_to_components();
    b.sync_to_components();
    let mut units = vec![a, b];
    let tilemap = TilemapLayers::new(4, 4);
    let camera = Camera::default();
    let grid = Grid::new(4, 4, 32, 8);
    let ui = json!([]);

    let mut mgr = SceneSaveManager::new();
    mgr.bootstrap_from_scene(&mut units, &tilemap);
    mgr.save_scene(
        &sm, &mut units, &tilemap, &camera, "EDITOR", "Select", 0, 1, &grid, &ui,
    )
    .unwrap();

    if let Some(e) = units.iter_mut().find(|u| u.id == id_b) {
        e.x = 9.0;
        e.sync_to_components();
    }
    mgr.note_entity_dirty(id_b);
    mgr.save_scene(
        &sm, &mut units, &tilemap, &camera, "EDITOR", "Select", 0, 1, &grid, &ui,
    )
    .unwrap();

    let data = AssetTools::read_json(sm.scene_path()).unwrap();
    let ids: Vec<u64> = data
        .get("entities")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .map(|e| e.get("id").and_then(|v| v.as_u64()).unwrap())
        .collect();
    assert!(ids.contains(&id_a));
    assert!(ids.contains(&id_b));
    if let Some(e) = data
        .get("entities")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .find(|e| e.get("id").and_then(|v| v.as_u64()) == Some(id_b))
    {
        assert_eq!(e.get("x").and_then(|v| v.as_f64()), Some(9.0));
    }
}

#[test]
fn ui_canvas_roundtrip_value() {
    let c = UiCanvasRoot::default_hud();
    let v = c.to_value();
    let roots = ui_canvases_from_value(&json!([v]));
    assert_eq!(roots.len(), 1);
    assert!(!roots[0].elements.is_empty());
}

#[test]
fn sprite_sheet_importer_png_grid() {
    let tmp = temp_dir("spritesheet");
    let png = tmp.join("sheet.png");
    let mut f = fs::File::create(&png).unwrap();
    f.write_all(MIN_PNG_1X1).unwrap();
    let meta = SpriteSheetImporter::build_metadata(&png, 1, 1, 0, 0).unwrap();
    assert!(!meta.slices.is_empty());
}

#[test]
fn waveform_cache_wav() {
    let tmp = temp_dir("wave");
    let wav = tmp.join("tone.wav");
    let wav_bytes: Vec<u8> = vec![
        b'R', b'I', b'F', b'F', 40, 0, 0, 0, b'W', b'A', b'V', b'E', b'f', b'm', b't', b' ', 16, 0,
        0, 0, 1, 0, 1, 0, 0x44, 0xAC, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00, 2, 0, 16, 0, b'd', b'a',
        b't', b'a', 4, 0, 0, 0, 0x00, 0x10, 0x00, 0x80,
    ];
    fs::write(&wav, wav_bytes).unwrap();
    let cache = WaveformCache::new(&tmp);
    let peaks = cache.peaks_for_wav(&wav, 4).unwrap();
    assert_eq!(peaks.len(), 4);
}

#[test]
fn packaging_manager_creates_destination() {
    let proj = temp_dir("pkg_proj");
    AssetTools::ensure_project_folders(&proj).unwrap();
    let dest = proj.join("out_pkg");
    let report = PackagingManager::package_project(&proj, &dest, ExportProfile::Debug).unwrap();
    assert!(report.destination.exists());
    assert!(dest.join("runtime_manifest.json").exists());
    assert!(dest.join("standalone_manifest.json").exists());
}

#[test]
fn profiler_reports_slowest_and_total() {
    let mut p = Profiler::new();
    p.record_system("Animation", 2.5);
    p.record_system("RTS", 0.4);
    let (name, ms) = p.slowest_system().unwrap();
    assert_eq!(name, "Animation");
    assert!((ms - 2.5).abs() < 0.001);
    assert!((p.systems_time_total_ms() - 2.9).abs() < 0.001);
}
