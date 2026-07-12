use std::collections::{BTreeMap, HashMap};

use crate::map::grid::Grid;
use pathfinding::prelude::{astar as pathfinding_astar, dijkstra_all};

pub type Point = (i32, i32);

#[derive(Debug, Clone, PartialEq)]
pub struct PathQueryReport {
    pub start: Point,
    pub goal: Point,
    pub found: bool,
    pub raw_len: usize,
    pub smoothed_len: usize,
    pub detour: i32,
    pub used_visibility_smoothing: bool,
    pub path: Vec<Point>,
}

pub fn heuristic(a: Point, b: Point) -> i32 {
    a.0.abs_diff(b.0)
        .saturating_add(a.1.abs_diff(b.1))
        .min(i32::MAX as u32) as i32
}

pub fn get_neighbors(grid: &Grid, current: Point) -> Vec<Point> {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .into_iter()
        .map(|(dx, dy)| (current.0 + dx, current.1 + dy))
        .filter(|&(x, y)| grid.is_walkable(x, y))
        .collect()
}

pub fn reconstruct_path(
    came_from: &HashMap<Point, Point>,
    start: Point,
    goal: Point,
) -> Vec<Point> {
    let mut current = goal;
    let mut path = vec![current];
    while current != start {
        let Some(next) = came_from.get(&current).copied() else {
            return Vec::new();
        };
        current = next;
        path.push(current);
    }
    path.reverse();
    path
}

pub fn smooth_path(path: &[Point]) -> Vec<Point> {
    if path.len() <= 2 {
        return path.to_vec();
    }
    let mut smoothed = vec![path[0]];
    let mut last_dir = (path[1].0 - path[0].0, path[1].1 - path[0].1);
    for window in path.windows(2).skip(1) {
        let dir = (window[1].0 - window[0].0, window[1].1 - window[0].1);
        if dir != last_dir {
            smoothed.push(window[0]);
            last_dir = dir;
        }
    }
    if let Some(last) = path.last().copied() {
        smoothed.push(last);
    }
    smoothed
}

pub fn smooth_path_with_visibility(grid: &Grid, path: &[Point]) -> Vec<Point> {
    if path.len() <= 2 {
        return path.to_vec();
    }
    let mut smoothed = vec![path[0]];
    let mut anchor = 0;
    let mut cursor = 2;
    while cursor < path.len() {
        if !safe_line_of_sight(grid, path[anchor], path[cursor]) {
            smoothed.push(path[cursor - 1]);
            anchor = cursor - 1;
        }
        cursor += 1;
    }
    if let Some(last) = path.last().copied() {
        smoothed.push(last);
    }
    smoothed
}

/// Finds a cardinal path while respecting an optional expansion budget.
///
/// A `max_iterations` value of `0` keeps the historical unlimited behavior.
/// Any other value caps the number of nodes whose neighbors may be expanded,
/// preventing a single NPC query from monopolizing a frame on a large map.
pub fn astar(grid: &Grid, start: Point, goal: Point, max_iterations: usize) -> Vec<Point> {
    let Some(start) = grid.find_nearest_walkable(start, 12) else {
        return Vec::new();
    };
    let Some(goal) = grid.find_nearest_walkable(goal, 24) else {
        return Vec::new();
    };

    let mut expanded = 0usize;
    pathfinding_astar(
        &start,
        |point| {
            if max_iterations != 0 && expanded >= max_iterations {
                return Vec::new();
            }
            expanded = expanded.saturating_add(1);
            weighted_neighbors(grid, *point, false, &HashMap::new(), 0)
        },
        |point| (heuristic(*point, goal).max(0) as u32).saturating_mul(10),
        |point| *point == goal,
    )
    .map(|(path, _)| path)
    .unwrap_or_default()
}

pub fn threat_aware_astar(
    grid: &Grid,
    start: Point,
    goal: Point,
    threats: &[(Point, u32)],
    threat_weight: u32,
) -> Vec<Point> {
    let Some(start) = grid.find_nearest_walkable(start, 12) else {
        return Vec::new();
    };
    let Some(goal) = grid.find_nearest_walkable(goal, 24) else {
        return Vec::new();
    };
    let threat_map = threats.iter().copied().collect::<HashMap<_, _>>();

    pathfinding_astar(
        &start,
        |point| weighted_neighbors(grid, *point, true, &threat_map, threat_weight),
        |point| diagonal_heuristic(*point, goal),
        |point| *point == goal,
    )
    .map(|(path, _)| path)
    .unwrap_or_default()
}

pub fn astar_report(
    grid: &Grid,
    start: Point,
    goal: Point,
    use_visibility_smoothing: bool,
) -> PathQueryReport {
    let raw = astar(grid, start, goal, 0);
    let path = if use_visibility_smoothing {
        smooth_path_with_visibility(grid, &raw)
    } else {
        smooth_path(&raw)
    };
    let detour = if raw.is_empty() {
        0
    } else {
        raw.len() as i32 - heuristic(start, goal).max(1)
    };
    PathQueryReport {
        start,
        goal,
        found: !raw.is_empty(),
        raw_len: raw.len(),
        smoothed_len: path.len(),
        detour,
        used_visibility_smoothing: use_visibility_smoothing,
        path,
    }
}

pub fn distance_map(grid: &Grid, goal: Point, max_cells: usize) -> BTreeMap<Point, u32> {
    let Some(goal) = grid.find_nearest_walkable(goal, 24) else {
        return BTreeMap::new();
    };
    let result = dijkstra_all(&goal, |point| {
        weighted_neighbors(grid, *point, true, &HashMap::new(), 0)
    });
    let mut distances = BTreeMap::from([(goal, 0)]);
    for (point, (_, cost)) in result.into_iter().take(max_cells.max(1)) {
        distances.insert(point, cost);
    }
    distances
}

pub fn influence_map(
    grid: &Grid,
    sources: &[(Point, i32)],
    falloff_per_step: i32,
) -> BTreeMap<Point, i32> {
    let mut influence = BTreeMap::new();
    for (source, strength) in sources {
        for (point, cost) in distance_map(grid, *source, grid.width.saturating_mul(grid.height)) {
            let steps = (cost / 10) as i32;
            let decay = steps.saturating_mul(falloff_per_step.max(0));
            let value = if *strength >= 0 {
                strength.saturating_sub(decay).max(0)
            } else {
                strength.saturating_add(decay).min(0)
            };
            if value == 0 {
                continue;
            }
            *influence.entry(point).or_insert(0) += value;
        }
    }
    influence
}

fn weighted_neighbors(
    grid: &Grid,
    current: Point,
    diagonal: bool,
    threats: &HashMap<Point, u32>,
    threat_weight: u32,
) -> Vec<(Point, u32)> {
    let mut offsets: Vec<(i32, i32, u32)> = vec![(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10)];
    if diagonal {
        offsets.extend([(1, 1, 14), (-1, 1, 14), (1, -1, 14), (-1, -1, 14)]);
    }
    offsets
        .into_iter()
        .map(|(dx, dy, cost)| ((current.0 + dx, current.1 + dy), cost))
        .filter(|(point, _)| {
            if !grid.is_walkable(point.0, point.1) {
                return false;
            }
            let dx = point.0 - current.0;
            let dy = point.1 - current.1;
            dx == 0
                || dy == 0
                || (grid.is_walkable(current.0 + dx, current.1)
                    && grid.is_walkable(current.0, current.1 + dy))
        })
        .map(|(point, cost)| {
            let step_cost: u32 = cost;
            let threat_cost = threats
                .get(&point)
                .copied()
                .unwrap_or(0)
                .saturating_mul(threat_weight);
            (point, step_cost.saturating_add(threat_cost))
        })
        .collect()
}

fn diagonal_heuristic(a: Point, b: Point) -> u32 {
    let dx = a.0.abs_diff(b.0);
    let dy = a.1.abs_diff(b.1);
    let diagonal = dx.min(dy);
    let straight = dx.max(dy).saturating_sub(diagonal);
    diagonal
        .saturating_mul(14)
        .saturating_add(straight.saturating_mul(10))
}

/// Grid line-of-sight that also rejects diagonal movement through the seam
/// between two blocked cells. The base grid ray is intentionally retained for
/// compatibility, while the extra checks make path smoothing match the rules
/// used by diagonal A* expansion.
fn safe_line_of_sight(grid: &Grid, start: Point, end: Point) -> bool {
    if !grid.line_of_sight(start, end) {
        return false;
    }

    let (mut x, mut y) = start;
    let dx = (end.0 - x).abs();
    let sx = if x < end.0 { 1 } else { -1 };
    let dy = -(end.1 - y).abs();
    let sy = if y < end.1 { 1 } else { -1 };
    let mut error = dx + dy;

    while (x, y) != end {
        let doubled = error.saturating_mul(2);
        let step_x = doubled >= dy;
        let step_y = doubled <= dx;
        let next_x = if step_x { x + sx } else { x };
        let next_y = if step_y { y + sy } else { y };

        if step_x && step_y && (!grid.is_walkable(next_x, y) || !grid.is_walkable(x, next_y)) {
            return false;
        }
        if step_x {
            error += dy;
        }
        if step_y {
            error += dx;
        }
        x = next_x;
        y = next_y;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astar_honors_expansion_budget_and_zero_remains_unlimited() {
        let grid = Grid::new(10, 1, 1, 1);

        assert!(astar(&grid, (0, 0), (9, 0), 1).is_empty());
        assert_eq!(astar(&grid, (0, 0), (9, 0), 0).len(), 10);
        assert_eq!(astar(&grid, (0, 0), (9, 0), 10).len(), 10);
    }

    #[test]
    fn diagonal_navigation_does_not_cut_through_blocked_corners() {
        let mut grid = Grid::new(2, 2, 1, 1);
        grid.set_tile(1, 0, 1);
        grid.set_tile(0, 1, 1);

        assert!(threat_aware_astar(&grid, (0, 0), (1, 1), &[], 0).is_empty());
    }

    #[test]
    fn visibility_smoothing_preserves_waypoint_around_blocked_corner() {
        let mut grid = Grid::new(2, 2, 1, 1);
        grid.set_tile(1, 0, 1);
        let path = vec![(0, 0), (0, 1), (1, 1)];

        assert_eq!(smooth_path_with_visibility(&grid, &path), path);
    }

    #[test]
    fn heuristic_handles_extreme_coordinates_without_overflow() {
        assert_eq!(
            heuristic((i32::MIN, i32::MIN), (i32::MAX, i32::MAX)),
            i32::MAX
        );
    }
}
