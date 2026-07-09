use std::collections::VecDeque;

use crate::engine::tilemap_layers::TilemapLayers;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileBrushMode {
    Pencil,
    Eraser,
    Fill,
    Rectangle,
    Line,
    Collision,
}

impl TileBrushMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pencil => "Pencil",
            Self::Eraser => "Eraser",
            Self::Fill => "Fill",
            Self::Rectangle => "Rect",
            Self::Line => "Line",
            Self::Collision => "Collision",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Pencil => Self::Eraser,
            Self::Eraser => Self::Fill,
            Self::Fill => Self::Rectangle,
            Self::Rectangle => Self::Line,
            Self::Line => Self::Collision,
            Self::Collision => Self::Pencil,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TilePaintChange {
    pub x: usize,
    pub y: usize,
    pub before: i32,
    pub after: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileBrushStroke {
    pub layer: usize,
    pub changes: Vec<TilePaintChange>,
}

pub struct TileBrush;

impl TileBrush {
    pub fn apply(
        tilemap: &mut TilemapLayers,
        mode: TileBrushMode,
        start: (usize, usize),
        end: (usize, usize),
        value: i32,
    ) -> TileBrushStroke {
        let layer = target_layer(tilemap, mode);
        let mut stroke = TileBrushStroke {
            layer,
            changes: Vec::new(),
        };
        if tilemap.width == 0 || tilemap.height == 0 || layer >= tilemap.layers.len() {
            return stroke;
        }

        let paint_value = match mode {
            TileBrushMode::Eraser => 0,
            TileBrushMode::Collision => value.max(1),
            _ => value,
        };

        match mode {
            TileBrushMode::Pencil | TileBrushMode::Eraser | TileBrushMode::Collision => {
                paint_cell(tilemap, layer, end.0, end.1, paint_value, &mut stroke);
            }
            TileBrushMode::Rectangle => {
                let min_x = start.0.min(end.0);
                let max_x = start.0.max(end.0);
                let min_y = start.1.min(end.1);
                let max_y = start.1.max(end.1);
                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        paint_cell(tilemap, layer, x, y, paint_value, &mut stroke);
                    }
                }
            }
            TileBrushMode::Line => {
                paint_line(tilemap, layer, start, end, paint_value, &mut stroke);
            }
            TileBrushMode::Fill => flood_fill(tilemap, layer, start, paint_value, &mut stroke),
        }

        stroke
    }
}

fn paint_line(
    tilemap: &mut TilemapLayers,
    layer: usize,
    start: (usize, usize),
    end: (usize, usize),
    value: i32,
    stroke: &mut TileBrushStroke,
) {
    let (mut x0, mut y0) = (start.0 as isize, start.1 as isize);
    let (x1, y1) = (end.0 as isize, end.1 as isize);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 {
            paint_cell(tilemap, layer, x0 as usize, y0 as usize, value, stroke);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x0 += sx;
        }
        if doubled <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn target_layer(tilemap: &TilemapLayers, mode: TileBrushMode) -> usize {
    if mode == TileBrushMode::Collision
        && let Some(index) = tilemap
            .layers
            .iter()
            .position(|layer| layer.name == "Collision")
    {
        return index;
    }
    tilemap
        .active_layer
        .min(tilemap.layers.len().saturating_sub(1))
}

fn paint_cell(
    tilemap: &mut TilemapLayers,
    layer: usize,
    x: usize,
    y: usize,
    value: i32,
    stroke: &mut TileBrushStroke,
) {
    if x >= tilemap.width || y >= tilemap.height {
        return;
    }
    let before = tilemap.layers[layer].get(x, y);
    if before == value {
        return;
    }
    tilemap.layers[layer].set(x, y, value);
    stroke.changes.push(TilePaintChange {
        x,
        y,
        before,
        after: value,
    });
}

fn flood_fill(
    tilemap: &mut TilemapLayers,
    layer: usize,
    start: (usize, usize),
    value: i32,
    stroke: &mut TileBrushStroke,
) {
    if start.0 >= tilemap.width || start.1 >= tilemap.height {
        return;
    }
    let target = tilemap.layers[layer].get(start.0, start.1);
    if target == value {
        return;
    }
    let mut visited = vec![vec![false; tilemap.width]; tilemap.height];
    let mut queue = VecDeque::from([start]);
    while let Some((x, y)) = queue.pop_front() {
        if x >= tilemap.width || y >= tilemap.height || visited[y][x] {
            continue;
        }
        visited[y][x] = true;
        if tilemap.layers[layer].get(x, y) != target {
            continue;
        }
        paint_cell(tilemap, layer, x, y, value, stroke);
        if x > 0 {
            queue.push_back((x - 1, y));
        }
        if y > 0 {
            queue.push_back((x, y - 1));
        }
        queue.push_back((x + 1, y));
        queue.push_back((x, y + 1));
    }
}
