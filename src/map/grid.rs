use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub tile_size: usize,
    pub chunk_size: usize,
    pub tiles: Vec<Vec<i32>>,
}

impl Grid {
    pub fn new(width: usize, height: usize, tile_size: usize, chunk_size: usize) -> Self {
        Self {
            width,
            height,
            tile_size,
            chunk_size,
            tiles: vec![vec![0; width]; height],
        }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.tiles[y as usize][x as usize] == 0
    }

    pub fn set_tile(&mut self, x: usize, y: usize, value: i32) {
        if x < self.width && y < self.height {
            self.tiles[y][x] = value;
        }
    }

    pub fn get_tile(&self, x: usize, y: usize) -> Option<i32> {
        self.tiles.get(y).and_then(|row| row.get(x)).copied()
    }

    pub fn set_rect(&mut self, x: usize, y: usize, width: usize, height: usize, value: i32) {
        let max_y = y.saturating_add(height).min(self.height);
        let max_x = x.saturating_add(width).min(self.width);
        for py in y..max_y {
            for px in x..max_x {
                self.tiles[py][px] = value;
            }
        }
    }

    pub fn world_to_cell(&self, x: f64, y: f64) -> (i32, i32) {
        let tile_size = self.tile_size.max(1) as f64;
        (
            (x / tile_size).floor() as i32,
            (y / tile_size).floor() as i32,
        )
    }

    pub fn cell_center(&self, x: i32, y: i32) -> (f64, f64) {
        let tile_size = self.tile_size.max(1) as f64;
        ((x as f64 + 0.5) * tile_size, (y as f64 + 0.5) * tile_size)
    }

    pub fn find_nearest_walkable(&self, target: (i32, i32), max_radius: i32) -> Option<(i32, i32)> {
        self.find_nearest_walkable_excluding(target, max_radius, &BTreeSet::new())
    }

    pub fn find_nearest_walkable_excluding(
        &self,
        target: (i32, i32),
        max_radius: i32,
        occupied: &BTreeSet<(i32, i32)>,
    ) -> Option<(i32, i32)> {
        let clamped = (
            target.0.clamp(0, self.width.saturating_sub(1) as i32),
            target.1.clamp(0, self.height.saturating_sub(1) as i32),
        );
        if self.is_walkable(clamped.0, clamped.1) && !occupied.contains(&clamped) {
            return Some(clamped);
        }

        let max_radius = max_radius.max(0);
        let mut best = None;
        let mut best_distance = i32::MAX;
        for radius in 1..=max_radius {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs().max(dy.abs()) != radius {
                        continue;
                    }
                    let candidate = (clamped.0 + dx, clamped.1 + dy);
                    if !self.is_walkable(candidate.0, candidate.1) || occupied.contains(&candidate)
                    {
                        continue;
                    }
                    let distance = (candidate.0 - target.0).abs() + (candidate.1 - target.1).abs();
                    if distance < best_distance {
                        best = Some(candidate);
                        best_distance = distance;
                    }
                }
            }
            if best.is_some() {
                return best;
            }
        }
        best
    }
}
