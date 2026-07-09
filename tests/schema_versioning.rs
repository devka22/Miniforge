use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::prefab_manager::PrefabManager;
use miniforge::engine::prefab_serializer::{PREFAB_FORMAT, PREFAB_SCHEMA_VERSION};
use miniforge::engine::scene_manager::SceneManager;
use miniforge::engine::scene_serializer::{SCENE_FORMAT, SCENE_SCHEMA_VERSION};
use miniforge::entities::game_object::GameObject;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct TestProject(PathBuf);

impl TestProject {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "miniforge_schema_{name}_{}_{}",
            std::process::id(),
            sequence
        ));
        AssetTools::ensure_project_folders(&path).expect("test project");
        Self(path)
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn scene_and_prefab_savers_emit_current_schema_headers() {
    let project = TestProject::new("save");
    let scene_manager = SceneManager::new(&project.0);
    let mut scene_entities = vec![GameObject::new(1.0, 2.0, Some("Hero".to_string()))];
    scene_manager
        .save_current_scene(&mut scene_entities)
        .expect("save scene");
    let scene = AssetTools::read_json(scene_manager.scene_path()).expect("scene JSON");
    assert_eq!(scene["format"], SCENE_FORMAT);
    assert_eq!(scene["schema_version"], SCENE_SCHEMA_VERSION);

    let prefab_manager = PrefabManager::new(&project.0);
    let mut prefab_entity = GameObject::new(0.0, 0.0, Some("HeroPrefab".to_string()));
    let prefab_path = prefab_manager
        .save_prefab(&mut prefab_entity, None)
        .expect("save prefab");
    let prefab = AssetTools::read_json(prefab_path).expect("prefab JSON");
    assert_eq!(prefab["format"], PREFAB_FORMAT);
    assert_eq!(prefab["schema_version"], PREFAB_SCHEMA_VERSION);
}

#[test]
fn loaders_reject_future_schema_instead_of_hiding_it_with_old_backup() {
    let project = TestProject::new("future");
    let paths = AssetTools::get_project_paths(&project.0);
    let scene_path = paths.scenes.join("main.scene");
    fs::write(
        &scene_path,
        include_str!("fixtures/formats/scene_future.scene"),
    )
    .expect("future scene");
    fs::write(
        scene_path.with_extension("scene.bak"),
        include_str!("fixtures/formats/scene_v1.scene"),
    )
    .expect("scene backup");
    let scene_error = SceneManager::new(&project.0)
        .load_current_scene_data()
        .expect_err("future scene must not fall back");
    assert!(scene_error.to_string().contains("newer than supported"));

    let prefab_path = paths.prefabs.join("future.prefab");
    fs::write(
        &prefab_path,
        include_str!("fixtures/formats/prefab_future.prefab"),
    )
    .expect("future prefab");
    fs::write(
        prefab_path.with_extension("prefab.bak"),
        include_str!("fixtures/formats/prefab_v1.prefab"),
    )
    .expect("prefab backup");
    let prefab_error = PrefabManager::new(&project.0)
        .load_prefab(&prefab_path)
        .expect_err("future prefab must not fall back");
    assert!(prefab_error.to_string().contains("newer than supported"));
}

#[test]
fn loaders_accept_legacy_documents_without_rewriting_them() {
    let project = TestProject::new("legacy");
    let paths = AssetTools::get_project_paths(&project.0);
    let scene_path = paths.scenes.join("main.scene");
    let legacy_scene = include_str!("fixtures/formats/scene_v0.scene");
    fs::write(&scene_path, legacy_scene).expect("legacy scene");
    let scene = SceneManager::new(&project.0)
        .load_current_scene_data()
        .expect("load legacy scene");
    assert_eq!(scene["schema_version"], SCENE_SCHEMA_VERSION);
    assert_eq!(
        fs::read_to_string(&scene_path).expect("unchanged scene"),
        legacy_scene
    );

    let prefab_path = paths.prefabs.join("legacy.prefab");
    let legacy_prefab = include_str!("fixtures/formats/prefab_v0.prefab");
    fs::write(&prefab_path, legacy_prefab).expect("legacy prefab");
    let loaded = PrefabManager::new(&project.0)
        .load_prefab(&prefab_path)
        .expect("load legacy prefab")
        .expect("prefab entity");
    assert_eq!(loaded.name, "LegacyHero_Instance");
    assert_eq!(
        fs::read_to_string(&prefab_path).expect("unchanged prefab"),
        legacy_prefab
    );
}
