use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SafeModeSettings {
    pub enabled: bool,
    pub disable_scripts: bool,
    pub disable_graphs: bool,
    pub disable_plugins: bool,
    pub disable_asset_importers: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SafeModeReport {
    pub active: bool,
    pub disabled_systems: Vec<String>,
    pub warnings: Vec<String>,
}

impl SafeModeSettings {
    pub fn for_recovery(reason: impl Into<String>) -> Self {
        Self {
            enabled: true,
            disable_scripts: true,
            disable_graphs: true,
            disable_plugins: true,
            disable_asset_importers: false,
            reason: reason.into(),
        }
    }

    pub fn report(&self) -> SafeModeReport {
        let mut disabled_systems = Vec::new();
        if self.disable_scripts {
            disabled_systems.push("scripts".to_string());
        }
        if self.disable_graphs {
            disabled_systems.push("visual_graphs".to_string());
        }
        if self.disable_plugins {
            disabled_systems.push("plugins".to_string());
        }
        if self.disable_asset_importers {
            disabled_systems.push("asset_importers".to_string());
        }
        SafeModeReport {
            active: self.enabled,
            disabled_systems,
            warnings: if self.enabled {
                vec![format!(
                    "Safe Mode activo: {}",
                    if self.reason.is_empty() {
                        "recuperacion manual"
                    } else {
                        &self.reason
                    }
                )]
            } else {
                Vec::new()
            },
        }
    }

    pub fn allows_scripts(&self) -> bool {
        !self.enabled || !self.disable_scripts
    }

    pub fn allows_graphs(&self) -> bool {
        !self.enabled || !self.disable_graphs
    }

    pub fn allows_plugins(&self) -> bool {
        !self.enabled || !self.disable_plugins
    }

    pub fn allows_asset_importers(&self) -> bool {
        !self.enabled || !self.disable_asset_importers
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde_json::json;

    use super::SafeModeSettings;
    use crate::core::game::Game;
    use crate::engine::asset_tools::AssetTools;
    use crate::engine::component::default_component;
    use crate::entities::game_object::GameObject;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestProject(std::path::PathBuf);

    impl TestProject {
        fn new() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "miniforge_safe_mode_{}_{}",
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
    fn recovery_mode_skips_luau_graphs_and_native_plugins() {
        let project = TestProject::new();
        fs::write(
            project.0.join("scripts/Unsafe.luau"),
            "function on_start()\n    set_position(99, 99)\nend\n",
        )
        .expect("script");
        let native = project.0.join("native/broken");
        fs::create_dir_all(&native).expect("native directory");
        fs::write(
            native.join("native.json"),
            serde_json::to_vec_pretty(&json!({
                "id": "broken",
                "library": "missing.dylib",
                "enabled": true,
                "required": true,
                "abi_version": 1,
                "category": "middleware",
                "platforms": [],
                "services": []
            }))
            .expect("manifest json"),
        )
        .expect("manifest");

        let safe_mode = SafeModeSettings::for_recovery("test recovery");
        let mut game = Game::from_project_with_safe_mode(&project.0, true, safe_mode)
            .expect("safe game starts despite broken native plugin");
        let mut entity = GameObject::new(0.0, 0.0, Some("SafePlayer".to_string()));
        entity.script = Some("Unsafe.luau".to_string());
        let mut graph = default_component("VisualScript").expect("visual script component");
        graph.set(
            "nodes",
            json!([
                {"id": "start", "type": "EventStart", "next": "move"},
                {"id": "move", "type": "Move", "x": 50.0, "y": 0.0, "next": null}
            ]),
        );
        entity.add_component(graph);
        game.units = vec![entity];

        game.run_headless_once(1.0 / 60.0);

        assert_eq!((game.units[0].x, game.units[0].y), (0.0, 0.0));
        assert_eq!(game.luau_script_runtime.last_frame_scripts, 0);
        assert_eq!(game.visual_script_runtime.last_frame_graphs, 0);
        assert!(game.native_libraries.diagnostics.is_empty());
    }
}
