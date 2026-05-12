use std::collections::{BTreeMap, HashMap};

use crate::map::grid::Grid;
use pathfinding::prelude::{astar as pathfinding_astar, dijkstra_all};

pub type Point = (i32, i32);

pub fn heuristic(a: Point, b: Point) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
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
    smoothed.push(*path.last().expect("non-empty path"));
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
        if !grid.line_of_sight(path[anchor], path[cursor]) {
            smoothed.push(path[cursor - 1]);
            anchor = cursor - 1;
        }
        cursor += 1;
    }
    smoothed.push(*path.last().expect("non-empty path"));
    smoothed
}

pub fn astar(grid: &Grid, start: Point, goal: Point, _max_iterations: usize) -> Vec<Point> {
    let Some(start) = grid.find_nearest_walkable(start, 12) else {
        return Vec::new();
    };
    let Some(goal) = grid.find_nearest_walkable(goal, 24) else {
        return Vec::new();
    };

    pathfinding_astar(
        &start,
        |point| weighted_neighbors(grid, *point, false, &HashMap::new(), 0),
        |point| (heuristic(*point, goal).max(0) as u32) * 10,
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
        |point| (heuristic(*point, goal).max(0) as u32) * 10,
        |point| *point == goal,
    )
    .map(|(path, _)| path)
    .unwrap_or_default()
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
    let mut offsets = vec![(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10)];
    if diagonal {
        offsets.extend([(1, 1, 14), (-1, 1, 14), (1, -1, 14), (-1, -1, 14)]);
    }
    offsets
        .into_iter()
        .map(|(dx, dy, cost)| ((current.0 + dx, current.1 + dy), cost))
        .filter(|(point, _)| grid.is_walkable(point.0, point.1))
        .map(|(point, cost)| {
            let threat_cost = threats
                .get(&point)
                .copied()
                .unwrap_or(0)
                .saturating_mul(threat_weight);
            (point, cost + threat_cost)
        })
        .collect()
}
