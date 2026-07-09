use std::collections::BTreeSet;

use crate::engine::formation::Formation;
use crate::entities::game_object::GameObject;
use crate::map::flow_field::FlowField;
use crate::map::grid::Grid;
use crate::map::pathfinding::{Point, astar, smooth_path_with_visibility, threat_aware_astar};

#[derive(Debug, Clone)]
pub struct CommandSystem {
    pub default_formation: String,
    pub formation_spacing: f64,
}

impl Default for CommandSystem {
    fn default() -> Self {
        Self {
            default_formation: "square".to_string(),
            formation_spacing: 1.0,
        }
    }
}

impl CommandSystem {
    pub fn build_path(grid: &Grid, start: (i32, i32), goal: (i32, i32)) -> Vec<(f64, f64)> {
        let Some(goal) = Self::clean_target(grid, goal) else {
            return Vec::new();
        };
        if !grid.in_bounds(start.0, start.1) {
            return Vec::new();
        }
        smooth_path_with_visibility(grid, &astar(grid, start, goal, 3000))
            .into_iter()
            .map(|(x, y)| (x as f64, y as f64))
            .collect()
    }

    pub fn build_flow_field_path(
        grid: &Grid,
        start: (i32, i32),
        goal: (i32, i32),
        max_steps: usize,
    ) -> Vec<(f64, f64)> {
        FlowField::build(grid, goal, grid.width.saturating_mul(grid.height).max(1))
            .map(|flow| {
                flow.path_from(grid, start, max_steps)
                    .into_iter()
                    .map(|(x, y)| (x as f64, y as f64))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn build_threat_aware_path(
        grid: &Grid,
        start: (i32, i32),
        goal: (i32, i32),
        threats: &[(Point, u32)],
    ) -> Vec<(f64, f64)> {
        let Some(goal) = Self::clean_target(grid, goal) else {
            return Vec::new();
        };
        smooth_path_with_visibility(grid, &threat_aware_astar(grid, start, goal, threats, 24))
            .into_iter()
            .map(|(x, y)| (x as f64, y as f64))
            .collect()
    }

    pub fn threat_aware_move_units(
        grid: &Grid,
        units: &mut [GameObject],
        target: (i32, i32),
        threats: &[(Point, u32)],
    ) {
        for unit in units {
            let start = (unit.x.round() as i32, unit.y.round() as i32);
            unit.path = Self::build_threat_aware_path(grid, start, target, threats);
            if unit.path.is_empty() {
                unit.path = vec![(target.0 as f64, target.1 as f64)];
            }
            unit.command = "THREAT_AWARE_MOVE".to_string();
            unit.state = "MOVING".to_string();
        }
    }

    pub fn flow_field_move_units(grid: &Grid, units: &mut [GameObject], target: (i32, i32)) {
        let Some(flow) =
            FlowField::build(grid, target, grid.width.saturating_mul(grid.height).max(1))
        else {
            return;
        };
        for unit in units {
            let start = (unit.x.round() as i32, unit.y.round() as i32);
            unit.path = flow
                .path_from(grid, start, 256)
                .into_iter()
                .map(|(x, y)| (x as f64, y as f64))
                .collect();
            if unit.path.is_empty() {
                unit.path = vec![(target.0 as f64, target.1 as f64)];
            }
            unit.command = "FLOW_FIELD_MOVE".to_string();
            unit.state = "MOVING".to_string();
        }
    }

    pub fn clean_target(grid: &Grid, target: (i32, i32)) -> Option<(i32, i32)> {
        let x = target.0.clamp(0, grid.width.saturating_sub(1) as i32);
        let y = target.1.clamp(0, grid.height.saturating_sub(1) as i32);
        if grid.is_walkable(x, y) {
            return Some((x, y));
        }

        grid.find_nearest_walkable(
            (x, y),
            grid.width.max(grid.height).min(i32::MAX as usize) as i32,
        )
    }

    pub fn move_unit_to(unit: &mut GameObject, target: (f64, f64)) {
        unit.command = "MOVE".to_string();
        unit.state = "MOVING".to_string();
        unit.path = vec![target];
    }

    pub fn move_unit_to_grid(grid: &Grid, unit: &mut GameObject, target: (i32, i32)) -> bool {
        let Some(target) = Self::clean_target(grid, target) else {
            return false;
        };
        let start = (unit.x as i32, unit.y as i32);
        unit.path = Self::build_path(grid, start, target);
        if unit.path.is_empty() {
            unit.path = vec![(target.0 as f64, target.1 as f64)];
        }
        unit.command = "MOVE".to_string();
        unit.state = "MOVING".to_string();
        true
    }

    pub fn move_units(units: &mut [GameObject], target: (f64, f64)) {
        for unit in units {
            Self::move_unit_to(unit, target);
        }
    }

    pub fn formation_move_units(
        grid: Option<&Grid>,
        units: &mut [GameObject],
        target: (f64, f64),
        formation: &str,
        spacing: f64,
    ) {
        let positions = Self::formation_targets(grid, units.len(), target, formation, spacing);
        for (unit, position) in units.iter_mut().zip(positions) {
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (position.0 as i32, position.1 as i32));
            } else {
                Self::move_unit_to(unit, position);
            }
            unit.command = "FORMATION_MOVE".to_string();
        }
    }

    pub fn formation_targets(
        grid: Option<&Grid>,
        count: usize,
        target: (f64, f64),
        formation: &str,
        spacing: f64,
    ) -> Vec<(f64, f64)> {
        let raw_positions = Formation::positions(formation, count, target, spacing.max(0.1));
        let Some(grid) = grid else {
            return raw_positions;
        };

        let mut occupied = BTreeSet::new();
        raw_positions
            .into_iter()
            .map(|position| {
                let cell = (position.0.round() as i32, position.1.round() as i32);
                let cleaned = grid
                    .find_nearest_walkable_excluding(cell, 8, &occupied)
                    .or_else(|| grid.find_nearest_walkable(cell, 16));
                if let Some(cleaned) = cleaned {
                    occupied.insert(cleaned);
                    (cleaned.0 as f64, cleaned.1 as f64)
                } else {
                    position
                }
            })
            .collect()
    }

    pub fn stop_units(units: &mut [GameObject]) {
        for unit in units {
            unit.command = "STOP".to_string();
            unit.state = "IDLE".to_string();
            unit.path.clear();
            unit.follow_target_id = None;
            unit.guard_target_id = None;
            unit.attack_move_target = None;
            unit.gather_target_id = None;
            if let Some(worker) = unit.get_component_mut("Worker") {
                worker.set("gather_target_id", serde_json::Value::Null);
            }
        }
    }

    pub fn hold_units(units: &mut [GameObject]) {
        for unit in units {
            unit.command = "HOLD".to_string();
            unit.state = "HOLD".to_string();
            unit.path.clear();
        }
    }

    pub fn cancel_units(units: &mut [GameObject]) {
        for unit in units {
            unit.command = "IDLE".to_string();
            unit.state = "IDLE".to_string();
            unit.path.clear();
            unit.follow_target_id = None;
            unit.guard_target_id = None;
            unit.attack_move_target = None;
            unit.gather_target_id = None;
            if let Some(worker) = unit.get_component_mut("Worker") {
                worker.set("gather_target_id", serde_json::Value::Null);
            }
        }
    }

    pub fn patrol_units(grid: Option<&Grid>, units: &mut [GameObject], target: (f64, f64)) {
        for unit in units {
            let start = (unit.x, unit.y);
            unit.patrol_points = vec![start, target];
            unit.patrol_index = 0;
            unit.command = "PATROL".to_string();
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (target.0 as i32, target.1 as i32));
            } else {
                Self::move_unit_to(unit, target);
            }
            unit.command = "PATROL".to_string();
        }
    }

    pub fn attack_move_units(grid: Option<&Grid>, units: &mut [GameObject], target: (f64, f64)) {
        for unit in units {
            unit.attack_move_target = Some(target);
            unit.command = "ATTACK_MOVE".to_string();
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (target.0 as i32, target.1 as i32));
            } else {
                Self::move_unit_to(unit, target);
            }
            unit.command = "ATTACK_MOVE".to_string();
        }
    }

    pub fn follow_units(grid: Option<&Grid>, units: &mut [GameObject], target: &GameObject) {
        for unit in units {
            if unit.id == target.id {
                continue;
            }
            unit.follow_target_id = Some(target.id);
            unit.guard_target_id = None;
            unit.command = "FOLLOW".to_string();
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (target.x as i32, target.y as i32));
            } else {
                Self::move_unit_to(unit, (target.x, target.y));
            }
            unit.command = "FOLLOW".to_string();
        }
    }

    pub fn guard_units(grid: Option<&Grid>, units: &mut [GameObject], target: &GameObject) {
        for unit in units {
            if unit.id == target.id {
                continue;
            }
            unit.guard_target_id = Some(target.id);
            unit.follow_target_id = None;
            unit.command = "GUARD".to_string();
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (target.x as i32, target.y as i32));
            } else {
                Self::move_unit_to(unit, (target.x, target.y));
            }
            unit.command = "GUARD".to_string();
        }
    }

    pub fn gather_units(
        grid: Option<&Grid>,
        units: &mut [GameObject],
        target: &GameObject,
    ) -> usize {
        if target.get_component("ResourceNode").is_none() {
            return 0;
        }
        let mut assigned = 0;
        for unit in units {
            if unit.get_component("Worker").is_none() {
                continue;
            }
            unit.gather_target_id = Some(target.id);
            unit.command = "GATHER".to_string();
            if let Some(worker) = unit.get_component_mut("Worker") {
                worker.set("gather_target_id", serde_json::json!(target.id));
            }
            if let Some(grid) = grid {
                Self::move_unit_to_grid(grid, unit, (target.x as i32, target.y as i32));
            } else {
                Self::move_unit_to(unit, (target.x, target.y));
            }
            unit.command = "GATHER".to_string();
            assigned += 1;
        }
        assigned
    }

    pub fn command_right_click(
        grid: Option<&Grid>,
        selected_units: &mut [GameObject],
        target_entity: Option<&GameObject>,
        grid_target: (f64, f64),
    ) {
        if let Some(target) = target_entity {
            if target.get_component("ResourceNode").is_some() {
                Self::gather_units(grid, selected_units, target);
                return;
            }
            if matches!(target.tag.as_str(), "Player" | "Neutral") {
                Self::follow_units(grid, selected_units, target);
                return;
            }
            if target.tag == "Enemy" {
                Self::attack_move_units(grid, selected_units, grid_target);
                return;
            }
        }
        Self::formation_move_units(grid, selected_units, grid_target, "square", 1.0);
    }
}
