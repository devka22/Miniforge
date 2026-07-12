use std::collections::{BTreeSet, VecDeque};
use std::io;
use std::path::{Path, PathBuf};

use image::{ImageBuffer, ImageReader, Rgba};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpriteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl SpriteColor {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };

    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub fn rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteEditorCanvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<SpriteColor>,
    pub active_color: SpriteColor,
    pub secondary_color: SpriteColor,
    pub zoom: u32,
    pub last_path: Option<PathBuf>,
    #[serde(skip)]
    undo_stack: VecDeque<SpriteCanvasSnapshot>,
    #[serde(skip)]
    redo_stack: VecDeque<SpriteCanvasSnapshot>,
    #[serde(skip)]
    pending_edit: Option<SpriteCanvasSnapshot>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SpriteBrushMirror {
    None,
    Horizontal,
    Vertical,
    Quad,
}

impl Default for SpriteBrushMirror {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteArtStats {
    pub opaque_pixels: usize,
    pub transparent_pixels: usize,
    pub unique_colors: usize,
    pub bounds: Option<[u32; 4]>,
}

#[derive(Debug, Clone, PartialEq)]
struct SpriteCanvasSnapshot {
    width: u32,
    height: u32,
    pixels: Vec<SpriteColor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimationFrameDraft {
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub duration: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimationClipDraft {
    pub name: String,
    pub source_path: Option<PathBuf>,
    pub fps: f32,
    pub frames: Vec<SpriteAnimationFrameDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimationTimelineMarker {
    pub frame_index: usize,
    pub label: String,
    pub start_time: f32,
    pub duration: f32,
    pub normalized_start: f32,
    pub normalized_end: f32,
    pub source_rect: [u32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimationTimelinePreview {
    pub clip_name: String,
    pub frame_count: usize,
    pub total_duration: f32,
    pub fps: f32,
    pub markers: Vec<SpriteAnimationTimelineMarker>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAnimationPlaybackSample {
    pub time: f32,
    pub looping: bool,
    pub frame_index: usize,
    pub source_rect: [u32; 4],
    pub normalized_time: f32,
}

impl SpriteAnimationClipDraft {
    pub fn total_duration(&self) -> f32 {
        self.frames
            .iter()
            .map(|frame| frame.duration.max(0.0))
            .sum()
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.frames.is_empty() {
            warnings.push("Animation clip has no frames".to_string());
        }
        if self.fps <= 0.0 {
            warnings.push("Animation clip fps must be greater than zero".to_string());
        }
        if self
            .frames
            .iter()
            .any(|frame| frame.width == 0 || frame.height == 0)
        {
            warnings.push("Animation clip contains empty frame rectangles".to_string());
        }
        if self.frames.iter().any(|frame| frame.duration <= 0.0) {
            warnings.push("Animation clip contains non-positive frame durations".to_string());
        }
        warnings
    }

    pub fn is_timeline_ready(&self) -> bool {
        self.warnings().is_empty()
    }

    pub fn timeline_preview(&self) -> SpriteAnimationTimelinePreview {
        let total_duration = self.total_duration();
        let safe_total = total_duration.max(f32::EPSILON);
        let mut elapsed = 0.0;
        let markers = self
            .frames
            .iter()
            .map(|frame| {
                let start_time = elapsed;
                let duration = frame.duration.max(0.0);
                elapsed += duration;
                SpriteAnimationTimelineMarker {
                    frame_index: frame.index,
                    label: format!("F{}", frame.index + 1),
                    start_time,
                    duration,
                    normalized_start: (start_time / safe_total).clamp(0.0, 1.0),
                    normalized_end: (elapsed / safe_total).clamp(0.0, 1.0),
                    source_rect: [frame.x, frame.y, frame.width, frame.height],
                }
            })
            .collect();
        SpriteAnimationTimelinePreview {
            clip_name: self.name.clone(),
            frame_count: self.frames.len(),
            total_duration,
            fps: self.fps,
            markers,
            warnings: self.warnings(),
        }
    }

    pub fn sample_at(&self, time: f32, looping: bool) -> Option<SpriteAnimationPlaybackSample> {
        if self.frames.is_empty() {
            return None;
        }
        let total = self.total_duration();
        if total <= 0.0 {
            let frame = &self.frames[0];
            return Some(playback_sample(frame, 0.0, looping, 0.0));
        }

        let mut t = time.max(0.0);
        if looping {
            t %= total;
        } else {
            t = t.min((total - f32::EPSILON).max(0.0));
        }

        let mut elapsed = 0.0;
        for frame in &self.frames {
            let duration = frame.duration.max(0.0);
            let end = elapsed + duration;
            if t < end || frame.index == self.frames.last().map(|last| last.index).unwrap_or(0) {
                return Some(playback_sample(frame, t, looping, t / total));
            }
            elapsed = end;
        }
        self.frames
            .last()
            .map(|frame| playback_sample(frame, t, looping, 1.0))
    }
}

fn playback_sample(
    frame: &SpriteAnimationFrameDraft,
    time: f32,
    looping: bool,
    normalized_time: f32,
) -> SpriteAnimationPlaybackSample {
    SpriteAnimationPlaybackSample {
        time,
        looping,
        frame_index: frame.index,
        source_rect: [frame.x, frame.y, frame.width, frame.height],
        normalized_time: normalized_time.clamp(0.0, 1.0),
    }
}

impl SpriteEditorCanvas {
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.clamp(1, 512);
        let height = height.clamp(1, 512);
        Self {
            width,
            height,
            pixels: vec![SpriteColor::TRANSPARENT; (width * height) as usize],
            active_color: SpriteColor::WHITE,
            secondary_color: SpriteColor::TRANSPARENT,
            zoom: 12,
            last_path: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            pending_edit: None,
        }
    }

    pub fn load_png(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::load_image(path)
    }

    pub fn load_image(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let image = ImageReader::open(path)
            .map_err(io::Error::other)?
            .decode()
            .map_err(io::Error::other)?
            .to_rgba8();
        let (width, height) = image.dimensions();
        let pixels = image
            .pixels()
            .map(|pixel| SpriteColor {
                r: pixel[0],
                g: pixel[1],
                b: pixel[2],
                a: pixel[3],
            })
            .collect();
        Ok(Self {
            width,
            height,
            pixels,
            active_color: SpriteColor::WHITE,
            secondary_color: SpriteColor::TRANSPARENT,
            zoom: 12,
            last_path: Some(path.to_path_buf()),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            pending_edit: None,
        })
    }

    /// Starts a coalesced edit. A complete brush stroke becomes one undo step.
    pub fn begin_edit(&mut self) {
        if self.pending_edit.is_none() {
            self.pending_edit = Some(self.snapshot());
        }
    }

    pub fn commit_edit(&mut self) -> bool {
        let Some(before) = self.pending_edit.take() else {
            return false;
        };
        if before == self.snapshot() {
            return false;
        }
        push_bounded(&mut self.undo_stack, before);
        self.redo_stack.clear();
        true
    }

    pub fn undo(&mut self) -> bool {
        let _ = self.commit_edit();
        let Some(previous) = self.undo_stack.pop_back() else {
            return false;
        };
        let current = self.snapshot();
        push_bounded(&mut self.redo_stack, current);
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop_back() else {
            return false;
        };
        let current = self.snapshot();
        push_bounded(&mut self.undo_stack, current);
        self.restore(next);
        true
    }

    pub fn can_undo(&self) -> bool {
        self.pending_edit.is_some() || !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn copy_region(&self, x: u32, y: u32, width: u32, height: u32) -> Vec<SpriteColor> {
        let width = width.min(self.width.saturating_sub(x));
        let height = height.min(self.height.saturating_sub(y));
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for py in y..y + height {
            for px in x..x + width {
                pixels.push(self.get_pixel(px, py).unwrap_or(SpriteColor::TRANSPARENT));
            }
        }
        pixels
    }

    pub fn paste_region(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        pixels: &[SpriteColor],
    ) -> usize {
        self.begin_edit();
        let mut changed = 0;
        for py in 0..height {
            for px in 0..width {
                let Some(color) = pixels.get((py * width + px) as usize).copied() else {
                    continue;
                };
                if self.get_pixel(x + px, y + py) != Some(color)
                    && self.set_pixel(x + px, y + py, color)
                {
                    changed += 1;
                }
            }
        }
        let _ = self.commit_edit();
        changed
    }

    pub fn save_png(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        self.save_image(path)
    }

    pub fn save_image(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.get_pixel(x, y).unwrap_or(SpriteColor::TRANSPARENT);
                image.put_pixel(x, y, Rgba(color.rgba()));
            }
        }
        image.save(&path).map_err(io::Error::other)?;
        self.last_path = Some(path.clone());
        Ok(path)
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<SpriteColor> {
        self.index(x, y)
            .and_then(|index| self.pixels.get(index).copied())
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: SpriteColor) -> bool {
        let Some(index) = self.index(x, y) else {
            return false;
        };
        if self.pixels[index] == color {
            return false;
        }
        self.pixels[index] = color;
        true
    }

    pub fn paint_brush(
        &mut self,
        x: i32,
        y: i32,
        radius: u32,
        color: SpriteColor,
        mirror: SpriteBrushMirror,
    ) -> usize {
        let radius = radius.min(64) as i32;
        let mut changed = 0usize;
        self.begin_edit();
        for (cx, cy) in self.mirrored_points(x, y, mirror) {
            for py in cy - radius..=cy + radius {
                for px in cx - radius..=cx + radius {
                    let dx = px - cx;
                    let dy = py - cy;
                    if dx * dx + dy * dy > radius * radius {
                        continue;
                    }
                    if px >= 0
                        && py >= 0
                        && self.get_pixel(px as u32, py as u32) != Some(color)
                        && self.set_pixel(px as u32, py as u32, color)
                    {
                        changed += 1;
                    }
                }
            }
        }
        let _ = self.commit_edit();
        changed
    }

    pub fn clear(&mut self, color: SpriteColor) {
        self.pixels.fill(color);
    }

    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: SpriteColor) {
        for py in y..(y + height).min(self.height) {
            for px in x..(x + width).min(self.width) {
                self.set_pixel(px, py, color);
            }
        }
    }

    pub fn fill_circle(&mut self, center_x: i32, center_y: i32, radius: u32, color: SpriteColor) {
        let radius = radius.min(256) as i32;
        for py in center_y - radius..=center_y + radius {
            for px in center_x - radius..=center_x + radius {
                let dx = px - center_x;
                let dy = py - center_y;
                if dx * dx + dy * dy <= radius * radius && px >= 0 && py >= 0 {
                    self.set_pixel(px as u32, py as u32, color);
                }
            }
        }
    }

    pub fn draw_circle_outline(
        &mut self,
        center_x: i32,
        center_y: i32,
        radius: u32,
        color: SpriteColor,
    ) {
        let radius = radius.min(256) as i32;
        let inner = (radius - 1).max(0);
        for py in center_y - radius..=center_y + radius {
            for px in center_x - radius..=center_x + radius {
                let dx = px - center_x;
                let dy = py - center_y;
                let distance = dx * dx + dy * dy;
                if distance <= radius * radius && distance >= inner * inner && px >= 0 && py >= 0 {
                    self.set_pixel(px as u32, py as u32, color);
                }
            }
        }
    }

    pub fn draw_rect_outline(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: SpriteColor,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        let max_x = x
            .saturating_add(width)
            .saturating_sub(1)
            .min(self.width - 1);
        let max_y = y
            .saturating_add(height)
            .saturating_sub(1)
            .min(self.height - 1);
        for px in x.min(self.width - 1)..=max_x {
            self.set_pixel(px, y.min(self.height - 1), color);
            self.set_pixel(px, max_y, color);
        }
        for py in y.min(self.height - 1)..=max_y {
            self.set_pixel(x.min(self.width - 1), py, color);
            self.set_pixel(max_x, py, color);
        }
    }

    pub fn bucket_fill(&mut self, x: u32, y: u32, color: SpriteColor) -> usize {
        let Some(target) = self.get_pixel(x, y) else {
            return 0;
        };
        if target == color {
            return 0;
        }
        let mut changed = 0usize;
        let mut queue = VecDeque::from([(x, y)]);
        let mut visited = BTreeSet::new();
        while let Some((px, py)) = queue.pop_front() {
            if px >= self.width || py >= self.height || !visited.insert((px, py)) {
                continue;
            }
            if self.get_pixel(px, py) != Some(target) {
                continue;
            }
            self.set_pixel(px, py, color);
            changed += 1;
            if px > 0 {
                queue.push_back((px - 1, py));
            }
            if py > 0 {
                queue.push_back((px, py - 1));
            }
            queue.push_back((px + 1, py));
            queue.push_back((px, py + 1));
        }
        changed
    }

    pub fn replace_color(&mut self, from: SpriteColor, to: SpriteColor) -> usize {
        let mut changed = 0usize;
        for pixel in &mut self.pixels {
            if *pixel == from {
                *pixel = to;
                changed += 1;
            }
        }
        changed
    }

    pub fn outline_alpha(&mut self, color: SpriteColor) -> usize {
        let original = self.pixels.clone();
        let mut changed = 0usize;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y).unwrap();
                if original[index].a > 0 {
                    continue;
                }
                let touches_opaque = neighbors4(x, y, self.width, self.height)
                    .into_iter()
                    .filter_map(|(nx, ny)| self.index(nx, ny))
                    .any(|neighbor| original[neighbor].a > 0);
                if touches_opaque {
                    self.pixels[index] = color;
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn outline_alpha_thick(&mut self, thickness: u32, color: SpriteColor) -> usize {
        let thickness = thickness.clamp(1, 16) as i32;
        let original = self.pixels.clone();
        let mut changed = 0usize;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.index(x, y).unwrap();
                if original[index].a > 0 {
                    continue;
                }
                let mut touches_opaque = false;
                'search: for oy in -thickness..=thickness {
                    for ox in -thickness..=thickness {
                        if ox * ox + oy * oy > thickness * thickness {
                            continue;
                        }
                        let nx = x as i32 + ox;
                        let ny = y as i32 + oy;
                        if nx < 0 || ny < 0 {
                            continue;
                        }
                        if let Some(neighbor) = self.index(nx as u32, ny as u32)
                            && original[neighbor].a > 0
                        {
                            touches_opaque = true;
                            break 'search;
                        }
                    }
                }
                if touches_opaque {
                    self.pixels[index] = color;
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn drop_shadow(&mut self, offset_x: i32, offset_y: i32, color: SpriteColor) -> usize {
        let original = self.pixels.clone();
        let mut changed = 0usize;
        for y in 0..self.height {
            for x in 0..self.width {
                let source = self.index(x, y).unwrap();
                if original[source].a == 0 {
                    continue;
                }
                let tx = x as i32 + offset_x;
                let ty = y as i32 + offset_y;
                if tx < 0 || ty < 0 {
                    continue;
                }
                let Some(target) = self.index(tx as u32, ty as u32) else {
                    continue;
                };
                if original[target].a == 0 && self.pixels[target] != color {
                    self.pixels[target] = color;
                    changed += 1;
                }
            }
        }
        changed
    }

    pub fn quantize_palette(&mut self, palette: &[SpriteColor]) -> usize {
        if palette.is_empty() {
            return 0;
        }
        let mut changed = 0usize;
        for pixel in &mut self.pixels {
            if pixel.a == 0 {
                continue;
            }
            let nearest = nearest_color(*pixel, palette);
            if *pixel != nearest {
                *pixel = nearest;
                changed += 1;
            }
        }
        changed
    }

    pub fn art_stats(&self) -> SpriteArtStats {
        let opaque_pixels = self.pixels.iter().filter(|pixel| pixel.a > 0).count();
        SpriteArtStats {
            opaque_pixels,
            transparent_pixels: self.pixels.len().saturating_sub(opaque_pixels),
            unique_colors: self.palette(usize::MAX).len(),
            bounds: self
                .content_bounds()
                .map(|(min_x, min_y, max_x, max_y)| [min_x, min_y, max_x, max_y]),
        }
    }

    pub fn palette_ramp(base: SpriteColor, steps: usize) -> Vec<SpriteColor> {
        let steps = steps.clamp(1, 32);
        if steps == 1 {
            return vec![base];
        }
        (0..steps)
            .map(|index| {
                let t = index as f32 / (steps - 1) as f32;
                let factor = 0.42 + t * 1.18;
                SpriteColor {
                    r: scale_channel(base.r, factor),
                    g: scale_channel(base.g, factor),
                    b: scale_channel(base.b, factor),
                    a: base.a,
                }
            })
            .collect()
    }

    pub fn create_pixel_art_character(
        width: u32,
        height: u32,
        primary: SpriteColor,
        accent: SpriteColor,
    ) -> Self {
        let mut canvas = Self::new(width.max(16), height.max(16));
        let shadow = SpriteColor {
            r: 16,
            g: 18,
            b: 24,
            a: 120,
        };
        let outline = SpriteColor {
            r: 18,
            g: 24,
            b: 31,
            a: 255,
        };
        let mid_x = (canvas.width / 2) as i32;
        let head_y = (canvas.height as f32 * 0.28) as i32;
        let body_y = (canvas.height as f32 * 0.54) as u32;
        let head_radius = (canvas.width.min(canvas.height) / 7).max(3);
        let body_w = (canvas.width / 3).max(5);
        let body_h = (canvas.height / 3).max(6);

        canvas.fill_circle(mid_x, head_y, head_radius + 1, outline);
        canvas.fill_circle(mid_x, head_y, head_radius, primary);
        canvas.fill_rect(
            mid_x.saturating_sub((body_w / 2) as i32) as u32,
            body_y,
            body_w,
            body_h,
            outline,
        );
        canvas.fill_rect(
            mid_x.saturating_sub((body_w / 2) as i32) as u32 + 1,
            body_y + 1,
            body_w.saturating_sub(2),
            body_h.saturating_sub(2),
            primary,
        );
        canvas.fill_rect(mid_x as u32, body_y + 3, (body_w / 2).max(2), 3, accent);
        canvas.paint_brush(
            mid_x - (head_radius as i32 / 2),
            head_y - 1,
            1,
            SpriteColor::WHITE,
            SpriteBrushMirror::Horizontal,
        );
        canvas.drop_shadow(2, 3, shadow);
        canvas
    }

    pub fn draw_line(&mut self, from: (i32, i32), to: (i32, i32), color: SpriteColor) {
        let (mut x0, mut y0) = from;
        let (x1, y1) = to;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as u32, y0 as u32, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn flip_horizontal(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width / 2 {
                let left = self.index(x, y).unwrap();
                let right = self.index(self.width - 1 - x, y).unwrap();
                self.pixels.swap(left, right);
            }
        }
    }

    pub fn flip_vertical(&mut self) {
        for y in 0..self.height / 2 {
            for x in 0..self.width {
                let top = self.index(x, y).unwrap();
                let bottom = self.index(x, self.height - 1 - y).unwrap();
                self.pixels.swap(top, bottom);
            }
        }
    }

    pub fn rotate_right(&mut self) {
        let mut next = vec![SpriteColor::TRANSPARENT; self.pixels.len()];
        let new_width = self.height;
        let new_height = self.width;
        for y in 0..self.height {
            for x in 0..self.width {
                let source = self.index(x, y).unwrap();
                let nx = self.height - 1 - y;
                let ny = x;
                let target = (ny * new_width + nx) as usize;
                next[target] = self.pixels[source];
            }
        }
        self.width = new_width;
        self.height = new_height;
        self.pixels = next;
    }

    pub fn crop_to_content(&mut self, padding: u32) -> bool {
        let Some((min_x, min_y, max_x, max_y)) = self.content_bounds() else {
            return false;
        };
        let min_x = min_x.saturating_sub(padding);
        let min_y = min_y.saturating_sub(padding);
        let max_x = (max_x + padding).min(self.width - 1);
        let max_y = (max_y + padding).min(self.height - 1);
        let new_width = max_x - min_x + 1;
        let new_height = max_y - min_y + 1;
        let mut next = vec![SpriteColor::TRANSPARENT; (new_width * new_height) as usize];
        for y in 0..new_height {
            for x in 0..new_width {
                let source = self.index(min_x + x, min_y + y).unwrap();
                let target = (y * new_width + x) as usize;
                next[target] = self.pixels[source];
            }
        }
        self.width = new_width;
        self.height = new_height;
        self.pixels = next;
        true
    }

    pub fn animation_clip_draft(
        &self,
        name: impl Into<String>,
        frame_width: u32,
        frame_height: u32,
        fps: f32,
    ) -> SpriteAnimationClipDraft {
        let frame_width = frame_width.max(1);
        let frame_height = frame_height.max(1);
        let mut frames = Vec::new();
        let mut index = 0usize;
        let duration = 1.0 / fps.max(1.0);
        let mut y = 0;
        while y + frame_height <= self.height {
            let mut x = 0;
            while x + frame_width <= self.width {
                frames.push(SpriteAnimationFrameDraft {
                    index,
                    x,
                    y,
                    width: frame_width,
                    height: frame_height,
                    duration,
                });
                index += 1;
                x += frame_width;
            }
            y += frame_height;
        }
        SpriteAnimationClipDraft {
            name: name.into(),
            source_path: self.last_path.clone(),
            fps,
            frames,
        }
    }

    pub fn palette(&self, limit: usize) -> Vec<SpriteColor> {
        let mut colors = BTreeSet::new();
        for pixel in &self.pixels {
            if pixel.a > 0 {
                colors.insert(*pixel);
            }
            if colors.len() >= limit {
                break;
            }
        }
        colors.into_iter().collect()
    }

    pub fn resize_canvas(&mut self, width: u32, height: u32) {
        let width = width.clamp(1, 512);
        let height = height.clamp(1, 512);
        let mut next = vec![SpriteColor::TRANSPARENT; (width * height) as usize];
        let copy_w = self.width.min(width);
        let copy_h = self.height.min(height);
        for y in 0..copy_h {
            for x in 0..copy_w {
                let source = self.index(x, y).unwrap();
                let target = (y * width + x) as usize;
                next[target] = self.pixels[source];
            }
        }
        self.width = width;
        self.height = height;
        self.pixels = next;
    }

    fn content_bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found = false;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get_pixel(x, y).is_some_and(|color| color.a > 0) {
                    found = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        found.then_some((min_x, min_y, max_x, max_y))
    }

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }

    fn snapshot(&self) -> SpriteCanvasSnapshot {
        SpriteCanvasSnapshot {
            width: self.width,
            height: self.height,
            pixels: self.pixels.clone(),
        }
    }

    fn restore(&mut self, snapshot: SpriteCanvasSnapshot) {
        self.width = snapshot.width;
        self.height = snapshot.height;
        self.pixels = snapshot.pixels;
        self.pending_edit = None;
    }

    fn mirrored_points(&self, x: i32, y: i32, mirror: SpriteBrushMirror) -> Vec<(i32, i32)> {
        let mx = self.width as i32 - 1 - x;
        let my = self.height as i32 - 1 - y;
        let mut points = match mirror {
            SpriteBrushMirror::None => vec![(x, y)],
            SpriteBrushMirror::Horizontal => vec![(x, y), (mx, y)],
            SpriteBrushMirror::Vertical => vec![(x, y), (x, my)],
            SpriteBrushMirror::Quad => vec![(x, y), (mx, y), (x, my), (mx, my)],
        };
        points.sort_unstable();
        points.dedup();
        points
    }
}

fn push_bounded(stack: &mut VecDeque<SpriteCanvasSnapshot>, snapshot: SpriteCanvasSnapshot) {
    const MAX_HISTORY: usize = 64;
    if stack.len() >= MAX_HISTORY {
        stack.pop_front();
    }
    stack.push_back(snapshot);
}

fn neighbors4(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::new();
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn scale_channel(channel: u8, factor: f32) -> u8 {
    ((channel as f32 * factor).round() as i32).clamp(0, 255) as u8
}

fn nearest_color(color: SpriteColor, palette: &[SpriteColor]) -> SpriteColor {
    palette
        .iter()
        .copied()
        .min_by_key(|candidate| color_distance(*candidate, color))
        .unwrap_or(color)
}

fn color_distance(a: SpriteColor, b: SpriteColor) -> u32 {
    let dr = a.r as i32 - b.r as i32;
    let dg = a.g as i32 - b.g as i32;
    let db = a.b as i32 - b.b as i32;
    let da = a.a as i32 - b.a as i32;
    (dr * dr + dg * dg + db * db + da * da) as u32
}

impl Default for SpriteEditorCanvas {
    fn default() -> Self {
        Self::new(16, 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_stroke_is_one_undo_step_and_redo_restores_it() {
        let mut canvas = SpriteEditorCanvas::new(8, 8);
        let ink = SpriteColor {
            r: 255,
            g: 80,
            b: 120,
            a: 255,
        };
        canvas.begin_edit();
        canvas.set_pixel(1, 1, ink);
        canvas.set_pixel(2, 1, ink);
        canvas.set_pixel(3, 1, ink);
        assert!(canvas.commit_edit());
        assert!(canvas.undo());
        assert_eq!(canvas.get_pixel(2, 1), Some(SpriteColor::TRANSPARENT));
        assert!(canvas.redo());
        assert_eq!(canvas.get_pixel(2, 1), Some(ink));
    }

    #[test]
    fn copy_and_paste_region_clips_to_canvas_and_is_undoable() {
        let mut canvas = SpriteEditorCanvas::new(4, 4);
        canvas.set_pixel(0, 0, SpriteColor::WHITE);
        let copied = canvas.copy_region(0, 0, 2, 2);
        assert_eq!(canvas.paste_region(3, 3, 2, 2, &copied), 1);
        assert_eq!(canvas.get_pixel(3, 3), Some(SpriteColor::WHITE));
        assert!(canvas.undo());
        assert_eq!(canvas.get_pixel(3, 3), Some(SpriteColor::TRANSPARENT));
    }

    #[test]
    fn advanced_sprite_tools_create_editable_pixel_art() {
        let primary = SpriteColor {
            r: 90,
            g: 190,
            b: 125,
            a: 255,
        };
        let accent = SpriteColor {
            r: 246,
            g: 210,
            b: 98,
            a: 255,
        };
        let mut canvas = SpriteEditorCanvas::create_pixel_art_character(32, 32, primary, accent);
        assert!(canvas.art_stats().opaque_pixels > 0);
        assert!(canvas.art_stats().unique_colors >= 3);

        let before = canvas.art_stats().opaque_pixels;
        canvas.outline_alpha_thick(2, SpriteColor::WHITE);
        assert!(canvas.art_stats().opaque_pixels > before);

        let ramp = SpriteEditorCanvas::palette_ramp(primary, 4);
        assert_eq!(ramp.len(), 4);
        assert!(canvas.quantize_palette(&ramp) > 0);
    }

    #[test]
    fn mirrored_brush_paints_symmetric_strokes_as_one_undo_step() {
        let mut canvas = SpriteEditorCanvas::new(12, 10);
        let ink = SpriteColor {
            r: 30,
            g: 180,
            b: 240,
            a: 255,
        };
        assert!(canvas.paint_brush(2, 3, 1, ink, SpriteBrushMirror::Quad) >= 4);
        assert_eq!(canvas.get_pixel(2, 3), Some(ink));
        assert_eq!(canvas.get_pixel(9, 3), Some(ink));
        assert_eq!(canvas.get_pixel(2, 6), Some(ink));
        assert_eq!(canvas.get_pixel(9, 6), Some(ink));
        assert!(canvas.undo());
        assert_eq!(canvas.art_stats().opaque_pixels, 0);
    }
}
