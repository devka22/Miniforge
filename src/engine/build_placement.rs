use serde_json::{Value, json};

use crate::entities::game_object::GameObject;
use crate::map::grid::Grid;

#[derive(Debug, Clone, PartialEq)]
pub struct BuildFootprint {
    pub width: usize,
    pub height: usize,
    pub clearance: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementResult {
    pub valid: bool,
    pub cell: (i32, i32),
    pub reason: Option<String>,
}

pub struct BuildPlacement;

impl BuildPlacement {
    pub fn footprint_from_component(
        component: Option<&crate::engine::component::Component>,
    ) -> BuildFootprint {
        let Some(component) = component else {
            return BuildFootprint::default();
        };
        BuildFootprint {
            width: component.get_i64("footprint_w", 2).max(1) as usize,
            height: component.get_i64("footprint_h", 2).max(1) as usize,
            clearance: component.get_i64("clearance", 0).max(0) as usize,
        }
    }

    pub fn validate(
        grid: &Grid,
        entities: &[GameObject],
        cell: (i32, i32),
        footprint: &BuildFootprint,
        team_id: Option<i64>,
    ) -> PlacementResult {
        if cell.0 < 0 || cell.1 < 0 {
            return PlacementResult::invalid(cell, "out_of_bounds");
        }
        let width = footprint.width.max(1) as i32;
        let height = footprint.height.max(1) as i32;
        let clearance = footprint.clearance as i32;
        let min_x = cell.0 - clearance;
        let min_y = cell.1 - clearance;
        let max_x = cell.0 + width - 1 + clearance;
        let max_y = cell.1 + height - 1 + clearance;

        if min_x < 0 || min_y < 0 || max_x >= grid.width as i32 || max_y >= grid.height as i32 {
            return PlacementResult::invalid(cell, "out_of_bounds");
        }

        for y in cell.1..cell.1 + height {
            for x in cell.0..cell.0 + width {
                if !grid.is_walkable(x, y) {
                    return PlacementResult::invalid(cell, "blocked_tile");
                }
            }
        }

        for entity in entities {
            if !entity.enabled || entity.get_component("ConstructionSite").is_some() {
                continue;
            }
            if team_id.is_some_and(|team_id| entity_team_id(entity) == Some(team_id))
                && !matches!(entity.layer.as_str(), "Buildings" | "Units")
            {
                continue;
            }
            let entity_min_x =
                (entity.x - entity.width.max(entity.radius * 2.0) * 0.5).floor() as i32;
            let entity_max_x =
                (entity.x + entity.width.max(entity.radius * 2.0) * 0.5).ceil() as i32;
            let entity_min_y =
                (entity.y - entity.height.max(entity.radius * 2.0) * 0.5).floor() as i32;
            let entity_max_y =
                (entity.y + entity.height.max(entity.radius * 2.0) * 0.5).ceil() as i32;
            if ranges_overlap(min_x, max_x, entity_min_x, entity_max_x)
                && ranges_overlap(min_y, max_y, entity_min_y, entity_max_y)
            {
                return PlacementResult::invalid(cell, "occupied");
            }
        }

        PlacementResult {
            valid: true,
            cell,
            reason: None,
        }
    }

    pub fn find_nearest_valid(
        grid: &Grid,
        entities: &[GameObject],
        desired: (i32, i32),
        footprint: &BuildFootprint,
        search_radius: i32,
        team_id: Option<i64>,
    ) -> Option<PlacementResult> {
        let search_radius = search_radius.max(0);
        for radius in 0..=search_radius {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs().max(dy.abs()) != radius {
                        continue;
                    }
                    let cell = (desired.0 + dx, desired.1 + dy);
                    let result = Self::validate(grid, entities, cell, footprint, team_id);
                    if result.valid {
                        return Some(result);
                    }
                }
            }
        }
        None
    }

    pub fn reserve_on_grid(
        grid: &mut Grid,
        cell: (i32, i32),
        footprint: &BuildFootprint,
        value: i32,
    ) {
        if cell.0 < 0 || cell.1 < 0 {
            return;
        }
        grid.set_rect(
            cell.0 as usize,
            cell.1 as usize,
            footprint.width.max(1),
            footprint.height.max(1),
            value,
        );
    }

    pub fn preview_payload(result: &PlacementResult, footprint: &BuildFootprint) -> Value {
        json!({
            "valid": result.valid,
            "cell": [result.cell.0, result.cell.1],
            "reason": result.reason,
            "footprint": {
                "width": footprint.width,
                "height": footprint.height,
                "clearance": footprint.clearance,
            }
        })
    }
}

impl Default for BuildFootprint {
    fn default() -> Self {
        Self {
            width: 2,
            height: 2,
            clearance: 0,
        }
    }
}

impl PlacementResult {
    fn invalid(cell: (i32, i32), reason: &str) -> Self {
        Self {
            valid: false,
            cell,
            reason: Some(reason.to_string()),
        }
    }
}

fn ranges_overlap(a_min: i32, a_max: i32, b_min: i32, b_max: i32) -> bool {
    a_min <= b_max && b_min <= a_max
}

fn entity_team_id(entity: &GameObject) -> Option<i64> {
    entity
        .get_component("Team")
        .map(|team| team.get_i64("team_id", 0))
}
