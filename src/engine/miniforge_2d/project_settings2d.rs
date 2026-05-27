use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::physics2d::Physics2DSettings;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSettings2D {
    pub general: GeneralSettings2D,
    pub rendering: RenderingSettings2D,
    pub physics2d: Physics2DSettings,
    pub input: InputSettings2D,
    pub audio: AudioSettings2D,
    pub ui: UiSettings2D,
    pub scripting: ScriptingSettings2D,
    pub visual_graphs: VisualGraphSettings2D,
    pub export: ExportSettings2D,
    pub editor: EditorSettings2D,
    pub plugins: PluginSettings2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralSettings2D {
    pub project_name: String,
    pub start_scene: String,
    pub base_width: u32,
    pub base_height: u32,
    pub target_fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderingSettings2D {
    pub background_color: [u8; 4],
    pub pixel_snap: bool,
    pub default_pixels_per_unit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputSettings2D {
    pub input_map: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioSettings2D {
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSettings2D {
    pub default_canvas: String,
    pub scale_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptingSettings2D {
    pub safe_mode: bool,
    pub scripts_path: String,
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VisualGraphSettings2D {
    pub graphs_path: String,
    pub debug_runtime: bool,
    pub validate_on_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportSettings2D {
    pub output_path: String,
    pub include_debug_symbols: bool,
    pub validate_before_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorSettings2D {
    pub theme: String,
    pub autosave: bool,
    pub autosave_seconds: u32,
    pub safe_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginSettings2D {
    pub active_plugins: Vec<String>,
    pub plugin_paths: Vec<String>,
}

impl Default for ProjectSettings2D {
    fn default() -> Self {
        Self {
            general: GeneralSettings2D {
                project_name: "MiniForge2DProject".to_string(),
                start_scene: "saves/scenes/main.scene".to_string(),
                base_width: 1280,
                base_height: 720,
                target_fps: 60,
            },
            rendering: RenderingSettings2D {
                background_color: [18, 20, 24, 255],
                pixel_snap: true,
                default_pixels_per_unit: 32.0,
            },
            physics2d: Physics2DSettings::default(),
            input: InputSettings2D {
                input_map: "settings/input_map.json".to_string(),
                actions: vec![
                    "Move".to_string(),
                    "Jump".to_string(),
                    "Interact".to_string(),
                ],
            },
            audio: AudioSettings2D {
                master_volume: 1.0,
                sfx_volume: 1.0,
                music_volume: 0.8,
            },
            ui: UiSettings2D {
                default_canvas: "assets/ui/hud.mfui".to_string(),
                scale_mode: "scale_with_screen".to_string(),
            },
            scripting: ScriptingSettings2D {
                safe_mode: true,
                scripts_path: "scripts".to_string(),
                hot_reload: true,
            },
            visual_graphs: VisualGraphSettings2D {
                graphs_path: "scripts/visual_graphs".to_string(),
                debug_runtime: true,
                validate_on_save: true,
            },
            export: ExportSettings2D {
                output_path: "build".to_string(),
                include_debug_symbols: true,
                validate_before_export: true,
            },
            editor: EditorSettings2D {
                theme: "dark".to_string(),
                autosave: true,
                autosave_seconds: 120,
                safe_mode: true,
            },
            plugins: PluginSettings2D {
                active_plugins: Vec::new(),
                plugin_paths: vec!["plugins".to_string()],
            },
        }
    }
}

impl ProjectSettings2D {
    pub fn load_or_default(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let settings = Self::default();
            settings.save_with_backup(path)?;
            return Ok(settings);
        }
        let value = AssetTools::read_json(path)?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }

    pub fn save_with_backup(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if path.exists() {
            std::fs::copy(path, path.with_extension("json.bak"))?;
        }
        AssetTools::write_json(path, &serde_json::to_value(self).unwrap_or_default())
    }

    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        if self.general.project_name.trim().is_empty() {
            report.error(
                "project_name",
                "general.project_name",
                "Nombre de proyecto vacio.",
            );
        }
        if self.general.start_scene.trim().is_empty() {
            report.error("start_scene", "general.start_scene", "Start scene vacio.");
        }
        if self.general.target_fps == 0 {
            report.error(
                "target_fps",
                "general.target_fps",
                "FPS objetivo debe ser mayor a cero.",
            );
        }
        if !(0.0..=1.0).contains(&self.audio.master_volume) {
            report.warning(
                "master_volume",
                "audio.master_volume",
                "Volumen maestro fuera de 0..1.",
            );
        }
        report
    }
}
