use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::engine::miniforge_2d::blueprint::minimal_blueprint_graph;
use miniforge::engine::native_library::{
    MINIFORGE_NATIVE_ABI_VERSION, NativeLibraryCategory, NativeLibraryManager,
    NativeLibraryManifest, dynamic_library_extension,
};
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
fn luau_template_compiles_and_runtime_applies_commands() {
    let root = temp_dir("luau-runtime");
    let script = AssetTools::create_luau_script(&root, "Mover").unwrap();
    let source = r#"
function on_start()
    set_tag("Player")
end

function on_update(dt: number)
    move(8 * dt, 0)
end
"#;
    fs::write(&script, source).unwrap();
    LuauScriptRuntime::validate_source(&AssetTools::template_luau_script("Valid"), "Valid.luau")
        .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("Player".to_string()));
    entity.script = Some("Mover.luau".to_string());
    let mut entities = vec![entity];
    let mut runtime = LuauScriptRuntime::new(&root);
    let report = runtime.update_entities(&mut entities, 0.25, "PLAY");

    assert_eq!(report.scripts_run, 2);
    assert!(report.commands_applied >= 2);
    assert_eq!(entities[0].tag, "Player");
    assert!((entities[0].x - 2.0).abs() < f64::EPSILON);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn blueprint_compiler_builds_schedule_and_rejects_exec_cycles() {
    let mut graph = minimal_blueprint_graph();
    let log_a = graph
        .add_node("PrintString", "A", 400.0, 0.0, json!({"message": "A"}))
        .unwrap();
    let log_b = graph
        .add_node("PrintString", "B", 700.0, 0.0, json!({"message": "B"}))
        .unwrap();
    assert!(
        graph
            .connect_nodes_checked("print_ready", "then", &log_a, "exec")
            .unwrap()
    );
    assert!(
        graph
            .connect_nodes_checked(&log_a, "then", &log_b, "exec")
            .unwrap()
    );
    assert!(
        graph
            .connect_nodes_checked(&log_b, "then", &log_a, "exec")
            .is_err()
    );

    let compiled = graph.compile();
    assert!(compiled.valid, "{:?}", compiled.diagnostics);
    assert_eq!(compiled.compiler_version, 2);
    assert!(compiled.execution_order.contains(&log_b));
    assert_eq!(compiled.entry_points["EventBeginPlay"], "begin_play");
}

#[test]
fn native_cpp_plugin_loads_through_the_c_abi_when_a_compiler_is_available() {
    let compiler = ["c++", "clang++", "g++"]
        .into_iter()
        .find(|candidate| Command::new(candidate).arg("--version").output().is_ok());
    let Some(compiler) = compiler else { return };

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = temp_dir("native-cpp");
    let library = root.join(format!(
        "libminiforge_example.{}",
        dynamic_library_extension()
    ));
    let mut command = Command::new(compiler);
    command.args(["-std=c++17", "-fPIC"]);
    if cfg!(target_os = "macos") {
        command.arg("-dynamiclib");
    } else {
        command.arg("-shared");
    }
    let status = command
        .arg("-I")
        .arg(workspace.join("include"))
        .arg(workspace.join("examples/native_plugin_cpp/example_plugin.cpp"))
        .arg("-o")
        .arg(&library)
        .status()
        .unwrap();
    assert!(status.success());

    let manifest = NativeLibraryManifest {
        id: "example_cpp".to_string(),
        library: library.clone(),
        enabled: true,
        required: true,
        abi_version: MINIFORGE_NATIVE_ABI_VERSION,
        category: NativeLibraryCategory::Middleware,
        platforms: Vec::new(),
        services: vec!["example.echo".to_string()],
    };
    let mut manager = NativeLibraryManager::new(&root);
    let info = manager.load(&manifest, &library).unwrap();
    assert_eq!(info.name, "MiniForge C++ Example");
    let result = manager
        .invoke("example_cpp", "echo", &json!({"value": 42}))
        .unwrap();
    assert_eq!(result.status, 0);
    assert_eq!(result.value["operation"], "echo");
    assert!(manager.unload("example_cpp"));
    fs::remove_dir_all(root).unwrap();
}
