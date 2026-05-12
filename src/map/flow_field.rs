use crate::map::grid::Grid;
use crate::map::pathfinding::{Point, distance_map};

#[derive(Debug, Clone, PartialEq)]
pub struct FlowField {
    pub width: usize,
    pub height: usize,
    pub goal: Point,
    pub costs: Vec<Vec<Option<u32>>>,
}

impl FlowField {
    pub fn build(grid: &Grid, goal: Point, max_iterations: usize) -> Option<Self> {
        let goal = grid.find_nearest_walkable(goal, 24)?;
        let mut costs = vec![vec![None; grid.width]; grid.height];
        for (point, cost) in distance_map(grid, goal, max_iterations) {
            if point.0 < 0 || point.1 < 0 {
                continue;
            }
            if let Some(row) = costs.get_mut(point.1 as usize)
                && let Some(slot) = row.get_mut(point.0 as usize)
            {
                *slot = Some(cost / 10);
            }
        }

        Some(Self {
            width: grid.width,
            height: grid.height,
            goal,
            costs,
        })
    }

    pub fn cost(&self, point: Point) -> Option<u32> {
        if point.0 < 0 || point.1 < 0 {
            return None;
        }
        self.costs
            .get(point.1 as usize)
            .and_then(|row| row.get(point.0 as usize))
            .copied()
            .flatten()
    }

    pub fn next_step(&self, grid: &Grid, point: Point) -> Option<Point> {
        if point == self.goal {
            return Some(point);
        }
        neighbors(grid, point)
            .into_iter()
            .filter_map(|candidate| self.cost(candidate).map(|cost| (candidate, cost)))
            .min_by_key(|(_, cost)| *cost)
            .map(|(candidate, _)| candidate)
    }

    pub fn direction(&self, grid: &Grid, point: Point) -> Option<(f64, f64)> {
        let next = self.next_step(grid, point)?;
        if next == point {
            return Some((0.0, 0.0));
        }
        let dx = (next.0 - point.0) as f64;
        let dy = (next.1 - point.1) as f64;
        let len = (dx * dx + dy * dy).sqrt();
        if len <= f64::EPSILON {
            Some((0.0, 0.0))
        } else {
            Some((dx / len, dy / len))
        }
    }

    pub fn path_from(&self, grid: &Grid, start: Point, max_steps: usize) -> Vec<Point> {
        let Some(mut current) = grid.find_nearest_walkable(start, 12) else {
            return Vec::new();
        };
        if self.cost(current).is_none() {
            return Vec::new();
        }

        let mut path = vec![current];
        for _ in 0..max_steps {
            if current == self.goal {
                break;
            }
            let Some(next) = self.next_step(grid, current) else {
                break;
            };
            if next == current {
                break;
            }
            current = next;
            path.push(current);
        }
        path
    }

    pub fn reachable_cells(&self) -> usize {
        self.costs
            .iter()
            .map(|row| row.iter().filter(|cell| cell.is_some()).count())
            .sum()
    }
}

fn neighbors(grid: &Grid, current: Point) -> Vec<Point> {
    [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
    ]
    .into_iter()
    .map(|(dx, dy)| (current.0 + dx, current.1 + dy))
    .filter(|&(x, y)| grid.is_walkable(x, y))
    .collect()
}
