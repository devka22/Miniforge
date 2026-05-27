use std::collections::BTreeSet;
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
        }
    }

    pub fn load_png(path: impl AsRef<Path>) -> io::Result<Self> {
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
        })
    }

    pub fn save_png(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
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
        self.pixels[index] = color;
        true
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

    fn index(&self, x: u32, y: u32) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }
}

impl Default for SpriteEditorCanvas {
    fn default() -> Self {
        Self::new(16, 16)
    }
}
