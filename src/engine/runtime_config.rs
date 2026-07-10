use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sysinfo::System;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub path: PathBuf,
    pub data: Value,
}

impl RuntimeConfig {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let defaults = Self::default_data();
        let mut data = if path.exists() {
            AssetTools::read_json(&path)?
        } else {
            defaults.clone()
        };
        merge_missing_defaults(&mut data, &defaults);
        AssetTools::write_json(&path, &data)?;
        Ok(Self { path, data })
    }

    pub fn default_data() -> Value {
        json!({
            "game_name": "MiniForgeGame",
            "start_scene": "main.scene",
            "window_width": 1100,
            "window_height": 740,
            "fullscreen": false,
            "target_fps": 60,
            "debug": true,
            "fixed_timestep": 0.016666667,
            "max_frame_steps": 5,
            "max_entities": 5000,
            "max_particles": 20000,
            "streaming_enabled": false,
            "asset_hot_reload": true,
            "quality_preset": "balanced",
            "performance_class": "desktop",
            "world_simulation": {
                "day_night_enabled": true,
                "weather_enabled": true,
                "vehicle_headlights": true,
                "day_length_seconds": 1333,
                "weather_min_seconds": 28,
                "weather_max_seconds": 50
            },
            "script_scheduler": {
                "enabled": true,
                "max_update_scripts_per_frame": 100000,
                "default_update_interval": 0.0,
                "distant_update_interval": 0.75,
                "budget_bypass_priority": 100,
                "prioritize_by_distance": true,
                "open_world_auto_policy": true
            },
            "graphics": {
                "quality": "medium",
                "view_frustum_culling": true,
                "occlusion_culling": true,
                "lod_enabled": true,
                "backface_culling_3d": true,
                "lod_near_pixels": 48,
                "lod_far_pixels": 18,
                "lod_cull_pixels": 3,
                "occlusion_padding": 0.0,
                "lighting_enabled": true,
                "shadow_lights_enabled": true,
                "light_sample_budget": 28,
                "max_shadow_lights": 8,
                "max_source_cores": 42,
                "max_drawn_entities": 520,
                "minimap_entity_budget": 58,
                "profiles": {
                    "low": {
                        "light_sample_budget": 18,
                        "max_shadow_lights": 0,
                        "max_source_cores": 18,
                        "max_drawn_entities": 260,
                        "minimap_entity_budget": 32,
                        "lod_far_pixels": 24
                    },
                    "medium": {
                        "light_sample_budget": 28,
                        "max_shadow_lights": 8,
                        "max_source_cores": 42,
                        "max_drawn_entities": 520,
                        "minimap_entity_budget": 58,
                        "lod_far_pixels": 18
                    },
                    "high": {
                        "light_sample_budget": 44,
                        "max_shadow_lights": 14,
                        "max_source_cores": 72,
                        "max_drawn_entities": 900,
                        "minimap_entity_budget": 84,
                        "lod_far_pixels": 14
                    },
                    "ultra": {
                        "light_sample_budget": 72,
                        "max_shadow_lights": 32,
                        "max_source_cores": 128,
                        "max_drawn_entities": 1600,
                        "minimap_entity_budget": 140,
                        "lod_far_pixels": 10
                    }
                }
            },
            "worker_threads": "auto",
            "parallel_asset_scan": true,
            "prefer_metal_on_macos": true,
        })
    }

    pub fn get(&self, key: &str, default: Value) -> Value {
        self.data.get(key).cloned().unwrap_or(default)
    }

    pub fn tuning(&self) -> RuntimeTuning {
        RuntimeTuning::from_value(&self.data)
    }

    pub fn hardware_profile(&self) -> HardwareProfile {
        HardwareProfile::detect()
    }

    pub fn optimized_tuning(&self) -> RuntimeTuning {
        self.tuning()
            .optimized_for_hardware(&self.hardware_profile())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RuntimeTuning {
    pub target_fps: u32,
    pub fixed_timestep: f64,
    pub max_frame_steps: u32,
    pub max_entities: usize,
    pub max_particles: usize,
    pub streaming_enabled: bool,
    pub asset_hot_reload: bool,
    pub quality_preset: String,
    pub performance_class: String,
    pub worker_threads: Option<usize>,
    pub parallel_asset_scan: bool,
    pub prefer_metal_on_macos: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardwareProfile {
    pub logical_cpus: usize,
    pub memory_mb: u64,
    pub os_name: String,
    pub arch: String,
    pub apple_silicon: bool,
    pub performance_tier: String,
}

fn merge_missing_defaults(data: &mut Value, defaults: &Value) {
    let (Some(data_map), Some(default_map)) = (data.as_object_mut(), defaults.as_object()) else {
        *data = defaults.clone();
        return;
    };
    for (key, value) in default_map {
        match data_map.get_mut(key) {
            Some(existing) if existing.is_object() && value.is_object() => {
                merge_missing_defaults(existing, value);
            }
            Some(_) => {}
            None => {
                data_map.insert(key.clone(), value.clone());
            }
        }
    }
}

impl Default for RuntimeTuning {
    fn default() -> Self {
        Self {
            target_fps: 60,
            fixed_timestep: 1.0 / 60.0,
            max_frame_steps: 5,
            max_entities: 5000,
            max_particles: 20000,
            streaming_enabled: false,
            asset_hot_reload: true,
            quality_preset: "balanced".to_string(),
            performance_class: "desktop".to_string(),
            worker_threads: None,
            parallel_asset_scan: true,
            prefer_metal_on_macos: true,
        }
    }
}

impl RuntimeTuning {
    pub fn from_value(value: &Value) -> Self {
        let defaults = Self::default();
        Self {
            target_fps: value
                .get("target_fps")
                .and_then(Value::as_u64)
                .map(|fps| fps.clamp(15, 360) as u32)
                .unwrap_or(defaults.target_fps),
            fixed_timestep: value
                .get("fixed_timestep")
                .and_then(Value::as_f64)
                .filter(|step| *step > 0.0)
                .unwrap_or(defaults.fixed_timestep),
            max_frame_steps: value
                .get("max_frame_steps")
                .and_then(Value::as_u64)
                .map(|steps| steps.clamp(1, 16) as u32)
                .unwrap_or(defaults.max_frame_steps),
            max_entities: value
                .get("max_entities")
                .and_then(Value::as_u64)
                .map(|count| count.min(1_000_000) as usize)
                .unwrap_or(defaults.max_entities),
            max_particles: value
                .get("max_particles")
                .and_then(Value::as_u64)
                .map(|count| count.min(2_000_000) as usize)
                .unwrap_or(defaults.max_particles),
            streaming_enabled: value
                .get("streaming_enabled")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.streaming_enabled),
            asset_hot_reload: value
                .get("asset_hot_reload")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.asset_hot_reload),
            quality_preset: value
                .get("quality_preset")
                .and_then(Value::as_str)
                .unwrap_or(&defaults.quality_preset)
                .to_string(),
            performance_class: value
                .get("performance_class")
                .and_then(Value::as_str)
                .unwrap_or(&defaults.performance_class)
                .to_string(),
            worker_threads: parse_worker_threads(value.get("worker_threads")),
            parallel_asset_scan: value
                .get("parallel_asset_scan")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.parallel_asset_scan),
            prefer_metal_on_macos: value
                .get("prefer_metal_on_macos")
                .and_then(Value::as_bool)
                .unwrap_or(defaults.prefer_metal_on_macos),
        }
    }

    pub fn complex_game_ready(&self) -> bool {
        self.streaming_enabled && self.max_entities >= 10_000 && self.max_particles >= 50_000
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.target_fps > 144 && self.fixed_timestep < 1.0 / 120.0 {
            warnings.push(
                "Runtime target_fps alto con fixed_timestep muy fino; revisa presupuesto CPU"
                    .to_string(),
            );
        }
        if self.max_entities >= 10_000 && !self.streaming_enabled {
            warnings.push(
                "max_entities alto sin streaming_enabled: activa particion/streaming para mundos grandes"
                    .to_string(),
            );
        }
        if self.max_particles >= 50_000 && self.target_fps > 120 {
            warnings.push(
                "max_particles alto con FPS alto: usa GPU particles o baja presupuesto por frame"
                    .to_string(),
            );
        }
        warnings
    }

    pub fn frame_budget_ms(&self) -> f64 {
        1000.0 / self.target_fps.max(1) as f64
    }

    pub fn recommended_worker_threads(&self) -> usize {
        if let Some(worker_threads) = self.worker_threads {
            return worker_threads;
        }
        match self.performance_class.as_str() {
            "mobile" => 2,
            "low_end" => 2,
            "workstation" => 8,
            _ => 4,
        }
    }

    pub fn recommended_worker_threads_for(&self, hardware: &HardwareProfile) -> usize {
        if let Some(worker_threads) = self.worker_threads {
            return worker_threads;
        }
        match self.performance_class.as_str() {
            "auto" | "apple_silicon" | "apple_silicon_pro" => hardware.recommended_worker_threads(),
            "workstation" if hardware.logical_cpus >= 10 => {
                hardware.recommended_worker_threads().max(8)
            }
            _ => self
                .recommended_worker_threads()
                .min(hardware.logical_cpus.max(1)),
        }
    }

    pub fn optimized_for_hardware(&self, hardware: &HardwareProfile) -> Self {
        let mut optimized = self.clone();
        if optimized.performance_class == "auto" {
            optimized.performance_class = hardware.performance_tier.clone();
        }
        if optimized.worker_threads.is_none() {
            optimized.worker_threads = Some(optimized.recommended_worker_threads_for(hardware));
        }
        optimized
    }

    pub fn hardware_recommendations(&self, hardware: &HardwareProfile) -> Vec<String> {
        let mut recommendations = Vec::new();
        let workers = self.recommended_worker_threads_for(hardware);
        if hardware.apple_silicon {
            recommendations.push(format!(
                "Apple Silicon detectado ({}/{} MB): usar {} workers para assets/scripts y mantener prefer_metal_on_macos=true",
                hardware.arch, hardware.memory_mb, workers
            ));
        } else {
            recommendations.push(format!(
                "{} CPU logicos detectados: usar {} workers para tareas de editor",
                hardware.logical_cpus, workers
            ));
        }
        if !self.parallel_asset_scan {
            recommendations.push(
                "parallel_asset_scan esta desactivado; activalo para proyectos con muchos assets"
                    .to_string(),
            );
        }
        if hardware.memory_mb >= 24 * 1024 && self.max_particles < 50_000 {
            recommendations.push(
                "Hay memoria suficiente para subir max_particles en escenas con efectos pesados"
                    .to_string(),
            );
        }
        recommendations
    }
}

impl Default for HardwareProfile {
    fn default() -> Self {
        let logical_cpus = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(4);
        let arch = std::env::consts::ARCH.to_string();
        let apple_silicon = cfg!(target_os = "macos") && arch == "aarch64";
        Self {
            logical_cpus,
            memory_mb: 0,
            os_name: std::env::consts::OS.to_string(),
            arch,
            apple_silicon,
            performance_tier: if logical_cpus >= 8 {
                "workstation".to_string()
            } else {
                "desktop".to_string()
            },
        }
    }
}

impl HardwareProfile {
    pub fn detect() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let logical_cpus = system.cpus().len().max(
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
        );
        let memory_mb = system.total_memory() / (1024 * 1024);
        let arch = std::env::consts::ARCH.to_string();
        let apple_silicon = cfg!(target_os = "macos") && arch == "aarch64";
        let os_name = System::long_os_version()
            .or_else(System::name)
            .unwrap_or_else(|| std::env::consts::OS.to_string());
        let performance_tier = if apple_silicon && logical_cpus >= 10 && memory_mb >= 24 * 1024 {
            "apple_silicon_pro"
        } else if apple_silicon && logical_cpus >= 8 {
            "apple_silicon"
        } else if logical_cpus >= 12 && memory_mb >= 32 * 1024 {
            "workstation"
        } else if logical_cpus <= 4 || (memory_mb > 0 && memory_mb < 8 * 1024) {
            "low_end"
        } else {
            "desktop"
        }
        .to_string();
        Self {
            logical_cpus,
            memory_mb,
            os_name,
            arch,
            apple_silicon,
            performance_tier,
        }
    }

    pub fn recommended_worker_threads(&self) -> usize {
        if self.logical_cpus <= 2 {
            return self.logical_cpus.max(1);
        }
        let reserve_for_ui = if self.apple_silicon { 2 } else { 1 };
        self.logical_cpus
            .saturating_sub(reserve_for_ui)
            .clamp(2, if self.apple_silicon { 12 } else { 16 })
    }
}

fn parse_worker_threads(value: Option<&Value>) -> Option<usize> {
    match value {
        Some(Value::Number(number)) => number.as_u64().map(|value| value.clamp(1, 64) as usize),
        Some(Value::String(text)) if text != "auto" => {
            text.parse::<usize>().ok().map(|value| value.clamp(1, 64))
        }
        _ => None,
    }
}
