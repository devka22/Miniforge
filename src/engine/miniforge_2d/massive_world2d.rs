use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldPartition2D {
    pub cell_size: f32,
    pub load_radius_cells: i32,
    pub keepalive_radius_cells: i32,
    pub max_loaded_chunks: usize,
    #[serde(default)]
    pub chunks: Vec<WorldChunk2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldChunk2D {
    pub cell_x: i32,
    pub cell_y: i32,
    pub scene_path: String,
    pub priority: i32,
    pub loaded: bool,
    pub entity_count: usize,
    pub last_touched_frame: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamingPlan2D {
    pub focus_cell: (i32, i32),
    #[serde(default)]
    pub load: Vec<StreamingAction2D>,
    #[serde(default)]
    pub keep_loaded: Vec<StreamingAction2D>,
    #[serde(default)]
    pub unload: Vec<StreamingAction2D>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamingAction2D {
    pub cell_x: i32,
    pub cell_y: i32,
    pub scene_path: String,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeBudget2D {
    pub target_fps: u32,
    pub max_entities: usize,
    pub max_visible_sprites: usize,
    pub max_particles: usize,
    pub max_draw_calls: usize,
    pub max_loaded_chunks: usize,
    pub max_script_ms: f32,
    pub max_physics_ms: f32,
    pub max_ui_ms: f32,
    pub max_memory_mb: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RuntimeBudgetStats2D {
    pub entities: usize,
    pub visible_sprites: usize,
    pub particles: usize,
    pub draw_calls: usize,
    pub loaded_chunks: usize,
    pub script_ms: f32,
    pub physics_ms: f32,
    pub ui_ms: f32,
    pub memory_mb: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetIssue2D {
    pub metric: String,
    pub severity: String,
    pub value: String,
    pub budget: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectPool2D {
    #[serde(default)]
    pub buckets: Vec<PoolBucket2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoolBucket2D {
    pub prefab: String,
    pub warm: usize,
    pub active: usize,
    pub inactive: usize,
    pub hard_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PoolAcquireResult2D {
    pub prefab: String,
    pub reused: bool,
    pub allowed: bool,
    pub active: usize,
    pub inactive: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnDirector2D {
    #[serde(default)]
    pub rules: Vec<SpawnRule2D>,
    pub max_spawn_per_tick: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnRule2D {
    pub prefab: String,
    pub tag: String,
    pub min_distance_from_camera: f32,
    pub max_distance_from_camera: f32,
    pub max_alive: usize,
    pub weight: f32,
    pub cooldown_frames: u64,
    pub last_spawn_frame: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpawnRequest2D {
    pub prefab: String,
    pub tag: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveSharding2D {
    pub shard_size_cells: i32,
    pub global_save_path: String,
    #[serde(default)]
    pub dirty_cells: Vec<(i32, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveShardPlan2D {
    pub global_save_path: String,
    #[serde(default)]
    pub shard_paths: Vec<String>,
}

impl Default for WorldPartition2D {
    fn default() -> Self {
        Self {
            cell_size: 64.0,
            load_radius_cells: 2,
            keepalive_radius_cells: 3,
            max_loaded_chunks: 49,
            chunks: Vec::new(),
        }
    }
}

impl WorldPartition2D {
    pub fn cell_for(&self, x: f32, y: f32) -> (i32, i32) {
        let size = self.cell_size.max(1.0);
        ((x / size).floor() as i32, (y / size).floor() as i32)
    }

    pub fn register_chunk(
        &mut self,
        cell_x: i32,
        cell_y: i32,
        scene_path: impl Into<String>,
    ) -> bool {
        if self
            .chunks
            .iter()
            .any(|chunk| chunk.cell_x == cell_x && chunk.cell_y == cell_y)
        {
            return false;
        }
        self.chunks.push(WorldChunk2D {
            cell_x,
            cell_y,
            scene_path: scene_path.into(),
            priority: 0,
            loaded: false,
            entity_count: 0,
            last_touched_frame: 0,
        });
        true
    }

    pub fn streaming_plan(&self, focus_x: f32, focus_y: f32) -> StreamingPlan2D {
        let focus_cell = self.cell_for(focus_x, focus_y);
        let load_cells = cells_in_radius(focus_cell, self.load_radius_cells);
        let keepalive_cells = cells_in_radius(focus_cell, self.keepalive_radius_cells);
        let mut load = Vec::new();
        let mut keep_loaded = Vec::new();
        let mut unload = Vec::new();

        for chunk in &self.chunks {
            let key = (chunk.cell_x, chunk.cell_y);
            let action = StreamingAction2D {
                cell_x: chunk.cell_x,
                cell_y: chunk.cell_y,
                scene_path: chunk.scene_path.clone(),
                priority: chunk.priority,
            };
            if load_cells.contains(&key) {
                if chunk.loaded {
                    keep_loaded.push(action);
                } else {
                    load.push(action);
                }
            } else if chunk.loaded && !keepalive_cells.contains(&key) {
                unload.push(action);
            } else if chunk.loaded {
                keep_loaded.push(action);
            }
        }

        load.sort_by_key(|action| {
            (
                -action.priority,
                manhattan((action.cell_x, action.cell_y), focus_cell),
            )
        });
        keep_loaded.sort_by_key(|action| manhattan((action.cell_x, action.cell_y), focus_cell));
        unload.sort_by_key(|action| -manhattan((action.cell_x, action.cell_y), focus_cell));

        let mut warnings = Vec::new();
        let projected_loaded = keep_loaded.len() + load.len();
        if projected_loaded > self.max_loaded_chunks {
            warnings.push(format!(
                "Streaming projected {projected_loaded} loaded chunks over budget {}",
                self.max_loaded_chunks
            ));
            load.truncate(self.max_loaded_chunks.saturating_sub(keep_loaded.len()));
        }

        StreamingPlan2D {
            focus_cell,
            load,
            keep_loaded,
            unload,
            warnings,
        }
    }

    pub fn estimate_entities_near(&self, focus_x: f32, focus_y: f32, radius_cells: i32) -> usize {
        let focus = self.cell_for(focus_x, focus_y);
        self.chunks
            .iter()
            .filter(|chunk| manhattan((chunk.cell_x, chunk.cell_y), focus) <= radius_cells)
            .map(|chunk| chunk.entity_count)
            .sum()
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.cell_size <= 0.0 {
            issues.push("WorldPartition2D cell_size debe ser > 0".to_string());
        }
        if self.load_radius_cells < 0 || self.keepalive_radius_cells < self.load_radius_cells {
            issues.push("WorldPartition2D radios de streaming invalidos".to_string());
        }
        if self.max_loaded_chunks == 0 {
            issues.push("WorldPartition2D max_loaded_chunks debe ser > 0".to_string());
        }
        let mut seen = BTreeSet::new();
        for chunk in &self.chunks {
            if !seen.insert((chunk.cell_x, chunk.cell_y)) {
                issues.push(format!(
                    "WorldPartition2D chunk duplicado: {},{}",
                    chunk.cell_x, chunk.cell_y
                ));
            }
            if chunk.scene_path.trim().is_empty() {
                issues.push(format!(
                    "WorldPartition2D chunk {},{} sin scene_path",
                    chunk.cell_x, chunk.cell_y
                ));
            }
        }
        issues
    }
}

impl Default for RuntimeBudget2D {
    fn default() -> Self {
        Self {
            target_fps: 60,
            max_entities: 20_000,
            max_visible_sprites: 8_000,
            max_particles: 25_000,
            max_draw_calls: 500,
            max_loaded_chunks: 49,
            max_script_ms: 4.0,
            max_physics_ms: 4.0,
            max_ui_ms: 2.0,
            max_memory_mb: 1024.0,
        }
    }
}

impl RuntimeBudget2D {
    pub fn assess(&self, stats: &RuntimeBudgetStats2D) -> Vec<BudgetIssue2D> {
        let mut issues = Vec::new();
        push_usize_budget(&mut issues, "entities", stats.entities, self.max_entities);
        push_usize_budget(
            &mut issues,
            "visible_sprites",
            stats.visible_sprites,
            self.max_visible_sprites,
        );
        push_usize_budget(
            &mut issues,
            "particles",
            stats.particles,
            self.max_particles,
        );
        push_usize_budget(
            &mut issues,
            "draw_calls",
            stats.draw_calls,
            self.max_draw_calls,
        );
        push_usize_budget(
            &mut issues,
            "loaded_chunks",
            stats.loaded_chunks,
            self.max_loaded_chunks,
        );
        push_f32_budget(
            &mut issues,
            "script_ms",
            stats.script_ms,
            self.max_script_ms,
        );
        push_f32_budget(
            &mut issues,
            "physics_ms",
            stats.physics_ms,
            self.max_physics_ms,
        );
        push_f32_budget(&mut issues, "ui_ms", stats.ui_ms, self.max_ui_ms);
        push_f32_budget(
            &mut issues,
            "memory_mb",
            stats.memory_mb,
            self.max_memory_mb,
        );
        issues
    }
}

impl ObjectPool2D {
    pub fn with_bucket(prefab: impl Into<String>, warm: usize, hard_limit: usize) -> Self {
        Self {
            buckets: vec![PoolBucket2D {
                prefab: prefab.into(),
                warm,
                active: 0,
                inactive: warm,
                hard_limit,
            }],
        }
    }

    pub fn acquire(&mut self, prefab: &str) -> PoolAcquireResult2D {
        let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| bucket.prefab == prefab)
        else {
            return PoolAcquireResult2D {
                prefab: prefab.to_string(),
                reused: false,
                allowed: false,
                active: 0,
                inactive: 0,
            };
        };
        if bucket.inactive > 0 {
            bucket.inactive -= 1;
            bucket.active += 1;
            return PoolAcquireResult2D {
                prefab: prefab.to_string(),
                reused: true,
                allowed: true,
                active: bucket.active,
                inactive: bucket.inactive,
            };
        }
        if bucket.active >= bucket.hard_limit {
            return PoolAcquireResult2D {
                prefab: prefab.to_string(),
                reused: false,
                allowed: false,
                active: bucket.active,
                inactive: bucket.inactive,
            };
        }
        bucket.active += 1;
        PoolAcquireResult2D {
            prefab: prefab.to_string(),
            reused: false,
            allowed: true,
            active: bucket.active,
            inactive: bucket.inactive,
        }
    }

    pub fn release(&mut self, prefab: &str) -> bool {
        let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| bucket.prefab == prefab)
        else {
            return false;
        };
        if bucket.active == 0 {
            return false;
        }
        bucket.active -= 1;
        bucket.inactive += 1;
        true
    }
}

impl Default for SpawnDirector2D {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
            max_spawn_per_tick: 8,
        }
    }
}

impl SpawnDirector2D {
    pub fn requests(
        &mut self,
        frame: u64,
        camera_x: f32,
        camera_y: f32,
        alive_by_prefab: impl Fn(&str) -> usize,
    ) -> Vec<SpawnRequest2D> {
        let mut requests = Vec::new();
        let max_spawn_per_tick = self.max_spawn_per_tick;
        for rule in &mut self.rules {
            if requests.len() >= max_spawn_per_tick {
                break;
            }
            if frame.saturating_sub(rule.last_spawn_frame) < rule.cooldown_frames {
                continue;
            }
            if alive_by_prefab(&rule.prefab) >= rule.max_alive {
                continue;
            }
            let angle_seed = stable_unit(frame, &rule.prefab) * std::f32::consts::TAU;
            let distance = rule.min_distance_from_camera
                + (rule.max_distance_from_camera - rule.min_distance_from_camera).max(0.0)
                    * stable_unit(frame + 17, &rule.tag);
            rule.last_spawn_frame = frame;
            requests.push(SpawnRequest2D {
                prefab: rule.prefab.clone(),
                tag: rule.tag.clone(),
                x: camera_x + angle_seed.cos() * distance,
                y: camera_y + angle_seed.sin() * distance,
            });
        }
        requests
    }
}

impl SaveSharding2D {
    pub fn new(shard_size_cells: i32, global_save_path: impl Into<String>) -> Self {
        Self {
            shard_size_cells: shard_size_cells.max(1),
            global_save_path: global_save_path.into(),
            dirty_cells: Vec::new(),
        }
    }

    pub fn mark_dirty(&mut self, cell_x: i32, cell_y: i32) {
        if !self.dirty_cells.contains(&(cell_x, cell_y)) {
            self.dirty_cells.push((cell_x, cell_y));
            self.dirty_cells.sort();
        }
    }

    pub fn mark_position_dirty(&mut self, partition: &WorldPartition2D, x: f32, y: f32) {
        let (cell_x, cell_y) = partition.cell_for(x, y);
        self.mark_dirty(cell_x, cell_y);
    }

    pub fn flush_plan(&self) -> SaveShardPlan2D {
        let mut shards = BTreeSet::new();
        let shard_size = self.shard_size_cells.max(1);
        for (cell_x, cell_y) in &self.dirty_cells {
            let shard_x = div_floor(*cell_x, shard_size);
            let shard_y = div_floor(*cell_y, shard_size);
            shards.insert(format!("saves/shards/shard_{shard_x}_{shard_y}.json"));
        }
        SaveShardPlan2D {
            global_save_path: self.global_save_path.clone(),
            shard_paths: shards.into_iter().collect(),
        }
    }
}

pub fn minimal_massive_world2d() -> (WorldPartition2D, RuntimeBudget2D, ObjectPool2D) {
    let mut partition = WorldPartition2D::default();
    for y in -2..=2 {
        for x in -2..=2 {
            partition.register_chunk(x, y, format!("saves/scenes/chunks/chunk_{x}_{y}.scene"));
        }
    }
    (
        partition,
        RuntimeBudget2D::default(),
        ObjectPool2D::with_bucket("assets/prefabs/projectile.prefab", 128, 1024),
    )
}

fn cells_in_radius(center: (i32, i32), radius: i32) -> BTreeSet<(i32, i32)> {
    let mut cells = BTreeSet::new();
    let radius = radius.max(0);
    for y in center.1 - radius..=center.1 + radius {
        for x in center.0 - radius..=center.0 + radius {
            if manhattan((x, y), center) <= radius {
                cells.insert((x, y));
            }
        }
    }
    cells
}

fn manhattan(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn push_usize_budget(issues: &mut Vec<BudgetIssue2D>, metric: &str, value: usize, budget: usize) {
    if value <= budget {
        return;
    }
    issues.push(BudgetIssue2D {
        metric: metric.to_string(),
        severity: if value > budget.saturating_mul(2) {
            "critical".to_string()
        } else {
            "warning".to_string()
        },
        value: value.to_string(),
        budget: budget.to_string(),
    });
}

fn push_f32_budget(issues: &mut Vec<BudgetIssue2D>, metric: &str, value: f32, budget: f32) {
    if value <= budget {
        return;
    }
    issues.push(BudgetIssue2D {
        metric: metric.to_string(),
        severity: if value > budget * 2.0 {
            "critical".to_string()
        } else {
            "warning".to_string()
        },
        value: format!("{value:.2}"),
        budget: format!("{budget:.2}"),
    });
}

fn stable_unit(frame: u64, seed: &str) -> f32 {
    let mut hash = frame.wrapping_mul(1469598103934665603);
    for byte in seed.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    (hash % 10_000) as f32 / 10_000.0
}

fn div_floor(value: i32, divisor: i32) -> i32 {
    let mut result = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && (remainder < 0) != (divisor < 0) {
        result -= 1;
    }
    result
}
