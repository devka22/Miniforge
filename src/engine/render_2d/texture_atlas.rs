//! Dynamic texture-atlas allocation and PNG/metadata export.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use guillotiere::{AllocId, AtlasAllocator, size2};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use super::{AtlasRegion2D, TextureAtlas2D};

/// Errors produced while allocating, uploading, or exporting an atlas.
#[derive(Debug)]
pub enum TextureAtlasError2D {
    InvalidAtlasSize {
        width: u32,
        height: u32,
    },
    InvalidImageSize {
        width: u32,
        height: u32,
    },
    DuplicateName(String),
    MissingRegion(String),
    SizeMismatch {
        name: String,
        expected: (u32, u32),
        actual: (u32, u32),
    },
    OutOfSpace {
        width: u32,
        height: u32,
        extrude: u32,
    },
    SizeOverflow,
    Image(image::ImageError),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for TextureAtlasError2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAtlasSize { width, height } => {
                write!(f, "invalid atlas size {width}x{height}")
            }
            Self::InvalidImageSize { width, height } => {
                write!(f, "invalid image size {width}x{height}")
            }
            Self::DuplicateName(name) => write!(f, "atlas region already exists: {name}"),
            Self::MissingRegion(name) => write!(f, "atlas region does not exist: {name}"),
            Self::SizeMismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "image size for {name} is {}x{}, expected {}x{}",
                actual.0, actual.1, expected.0, expected.1
            ),
            Self::OutOfSpace {
                width,
                height,
                extrude,
            } => write!(
                f,
                "atlas has no room for {width}x{height} with {extrude}px extrusion"
            ),
            Self::SizeOverflow => write!(f, "atlas dimensions exceed supported integer limits"),
            Self::Image(error) => write!(f, "image error: {error}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Json(error) => write!(f, "atlas metadata error: {error}"),
        }
    }
}

impl std::error::Error for TextureAtlasError2D {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Image(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<image::ImageError> for TextureAtlasError2D {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value)
    }
}

impl From<std::io::Error> for TextureAtlasError2D {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for TextureAtlasError2D {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureAtlasStats2D {
    pub region_count: usize,
    pub content_pixels: u64,
    pub allocated_pixels: u64,
    pub total_pixels: u64,
    pub occupancy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpriteAtlasExportOptions2D {
    pub atlas_name: String,
    pub width: u32,
    pub height: u32,
    pub extrude: u32,
    pub trim_transparent: bool,
    pub power_of_two_pages: bool,
    pub output_prefix: String,
    pub source_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAtlasExportReport2D {
    pub manifest_path: PathBuf,
    #[serde(default)]
    pub output_files: Vec<PathBuf>,
    pub manifest: SpriteAtlasExportManifest2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAtlasExportManifest2D {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub pages: Vec<SpriteAtlasPageManifest2D>,
    #[serde(default)]
    pub regions: BTreeMap<String, SpriteAtlasRegionManifest2D>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAtlasPageManifest2D {
    pub index: usize,
    pub image: String,
    pub atlas: String,
    pub width: u32,
    pub height: u32,
    pub region_count: usize,
    pub occupancy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpriteAtlasRegionManifest2D {
    pub page: usize,
    pub source: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub original_w: u32,
    pub original_h: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub extrude: u32,
    pub uv: [f32; 4],
}

#[derive(Clone, Copy)]
struct DynamicAllocation2D {
    id: AllocId,
    outer_x: u32,
    outer_y: u32,
    outer_width: u32,
    outer_height: u32,
}

/// A mutable RGBA atlas backed by `guillotiere::AtlasAllocator`.
///
/// `TextureAtlas2D` remains the serializable asset-facing description. This type
/// owns the runtime allocation IDs and pixel buffer needed to release and reuse
/// space without rebuilding the whole atlas.
pub struct DynamicTextureAtlas2D {
    atlas: TextureAtlas2D,
    allocator: AtlasAllocator,
    allocations: BTreeMap<String, DynamicAllocation2D>,
    pixels: RgbaImage,
}

impl DynamicTextureAtlas2D {
    pub fn new(
        name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<Self, TextureAtlasError2D> {
        if width == 0 || height == 0 || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return Err(TextureAtlasError2D::InvalidAtlasSize { width, height });
        }

        Ok(Self {
            atlas: TextureAtlas2D {
                name: name.into(),
                width,
                height,
                regions: BTreeMap::new(),
            },
            allocator: AtlasAllocator::new(size2(width as i32, height as i32)),
            allocations: BTreeMap::new(),
            pixels: RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0])),
        })
    }

    pub fn atlas(&self) -> &TextureAtlas2D {
        &self.atlas
    }

    pub fn pixels(&self) -> &RgbaImage {
        &self.pixels
    }

    pub fn contains(&self, name: &str) -> bool {
        self.allocations.contains_key(name)
    }

    pub fn region(&self, name: &str) -> Option<&AtlasRegion2D> {
        self.atlas.regions.get(name)
    }

    pub fn uv_rect(&self, name: &str) -> Option<[f32; 4]> {
        self.atlas.uv_rect(name)
    }

    /// Allocates a transparent region. Use [`Self::upload`] to fill it later.
    pub fn allocate(
        &mut self,
        name: impl Into<String>,
        width: u32,
        height: u32,
        extrude: u32,
    ) -> Result<AtlasRegion2D, TextureAtlasError2D> {
        let name = name.into();
        if self.allocations.contains_key(&name) {
            return Err(TextureAtlasError2D::DuplicateName(name));
        }
        if width == 0 || height == 0 {
            return Err(TextureAtlasError2D::InvalidImageSize { width, height });
        }

        let doubled_extrude = extrude
            .checked_mul(2)
            .ok_or(TextureAtlasError2D::SizeOverflow)?;
        let outer_width = width
            .checked_add(doubled_extrude)
            .ok_or(TextureAtlasError2D::SizeOverflow)?;
        let outer_height = height
            .checked_add(doubled_extrude)
            .ok_or(TextureAtlasError2D::SizeOverflow)?;
        if outer_width > i32::MAX as u32 || outer_height > i32::MAX as u32 {
            return Err(TextureAtlasError2D::SizeOverflow);
        }

        let allocation = self
            .allocator
            .allocate(size2(outer_width as i32, outer_height as i32))
            .ok_or(TextureAtlasError2D::OutOfSpace {
                width,
                height,
                extrude,
            })?;
        let outer_x = allocation.rectangle.min.x as u32;
        let outer_y = allocation.rectangle.min.y as u32;
        let region = AtlasRegion2D {
            x: outer_x + extrude,
            y: outer_y + extrude,
            width,
            height,
            extrude,
        };
        self.allocations.insert(
            name.clone(),
            DynamicAllocation2D {
                id: allocation.id,
                outer_x,
                outer_y,
                outer_width,
                outer_height,
            },
        );
        self.atlas.regions.insert(name, region);
        Ok(region)
    }

    /// Allocates and copies an RGBA image, replicating edge pixels into padding.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        image: &RgbaImage,
        extrude: u32,
    ) -> Result<AtlasRegion2D, TextureAtlasError2D> {
        let name = name.into();
        let region = self.allocate(name.clone(), image.width(), image.height(), extrude)?;
        // Allocation succeeded, so upload can only fail if internal state is inconsistent.
        if let Err(error) = self.upload(&name, image) {
            self.remove(&name);
            return Err(error);
        }
        Ok(region)
    }

    pub fn insert_file(
        &mut self,
        name: impl Into<String>,
        path: impl AsRef<Path>,
        extrude: u32,
    ) -> Result<AtlasRegion2D, TextureAtlasError2D> {
        let image = image::open(path)?.to_rgba8();
        self.insert(name, &image, extrude)
    }

    /// Replaces the pixels in an existing slot without changing its UVs.
    pub fn upload(&mut self, name: &str, image: &RgbaImage) -> Result<(), TextureAtlasError2D> {
        let region = self
            .atlas
            .regions
            .get(name)
            .copied()
            .ok_or_else(|| TextureAtlasError2D::MissingRegion(name.to_string()))?;
        if (image.width(), image.height()) != (region.width, region.height) {
            return Err(TextureAtlasError2D::SizeMismatch {
                name: name.to_string(),
                expected: (region.width, region.height),
                actual: (image.width(), image.height()),
            });
        }

        blit_with_extrusion(&mut self.pixels, image, region);
        Ok(())
    }

    /// Releases a slot and immediately makes its rectangle available for reuse.
    pub fn remove(&mut self, name: &str) -> Option<AtlasRegion2D> {
        let allocation = self.allocations.remove(name)?;
        self.allocator.deallocate(allocation.id);
        clear_rectangle(
            &mut self.pixels,
            allocation.outer_x,
            allocation.outer_y,
            allocation.outer_width,
            allocation.outer_height,
        );
        self.atlas.regions.remove(name)
    }

    pub fn clear(&mut self) {
        self.allocator.clear();
        self.allocations.clear();
        self.atlas.regions.clear();
        self.pixels.fill(0);
    }

    pub fn stats(&self) -> TextureAtlasStats2D {
        let content_pixels = self
            .atlas
            .regions
            .values()
            .map(|region| u64::from(region.width) * u64::from(region.height))
            .sum();
        let allocated_pixels = self
            .allocations
            .values()
            .map(|allocation| {
                u64::from(allocation.outer_width) * u64::from(allocation.outer_height)
            })
            .sum();
        let total_pixels = u64::from(self.atlas.width) * u64::from(self.atlas.height);
        TextureAtlasStats2D {
            region_count: self.allocations.len(),
            content_pixels,
            allocated_pixels,
            total_pixels,
            occupancy: allocated_pixels as f32 / total_pixels.max(1) as f32,
        }
    }

    /// Writes a PNG and an importer-compatible `.atlas.json` sidecar.
    pub fn export(
        &self,
        png_path: impl AsRef<Path>,
        atlas_json_path: impl AsRef<Path>,
    ) -> Result<(), TextureAtlasError2D> {
        let png_path = png_path.as_ref();
        let atlas_json_path = atlas_json_path.as_ref();
        ensure_parent(png_path)?;
        ensure_parent(atlas_json_path)?;
        self.pixels.save(png_path)?;

        let image_path = png_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| png_path.to_string_lossy().into_owned());
        let regions = self
            .atlas
            .regions
            .iter()
            .map(|(name, region)| {
                (
                    name.clone(),
                    ExportRegion2D {
                        x: region.x,
                        y: region.y,
                        w: region.width,
                        h: region.height,
                    },
                )
            })
            .collect();
        let manifest = ExportAtlas2D {
            image: image_path,
            regions,
        };
        fs::write(atlas_json_path, serde_json::to_vec_pretty(&manifest)?)?;
        Ok(())
    }
}

/// Packs files such as `player.png`, `enemy.png`, and `coin.png`, then exports
/// one image plus an importer-compatible metadata sidecar.
pub fn build_texture_atlas_from_files(
    name: impl Into<String>,
    width: u32,
    height: u32,
    sources: &[(String, PathBuf)],
    extrude: u32,
    png_path: impl AsRef<Path>,
    atlas_json_path: impl AsRef<Path>,
) -> Result<TextureAtlas2D, TextureAtlasError2D> {
    let mut atlas = DynamicTextureAtlas2D::new(name, width, height)?;
    for (region_name, source_path) in sources {
        atlas.insert_file(region_name.clone(), source_path, extrude)?;
    }
    atlas.export(png_path, atlas_json_path)?;
    Ok(atlas.atlas.clone())
}

pub fn export_sprite_atlas_pages_from_files(
    sources: &[(String, PathBuf)],
    output_dir: impl AsRef<Path>,
    options: SpriteAtlasExportOptions2D,
) -> Result<SpriteAtlasExportReport2D, TextureAtlasError2D> {
    let options = options.normalized()?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let mut sorted_sources = sources.to_vec();
    sorted_sources.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let mut manifest = SpriteAtlasExportManifest2D {
        version: 1,
        name: options.atlas_name.clone(),
        pages: Vec::new(),
        regions: BTreeMap::new(),
        warnings: Vec::new(),
    };
    let mut output_files = Vec::new();
    let mut page_index = 0usize;
    let mut atlas = DynamicTextureAtlas2D::new(
        page_name(&options.output_prefix, page_index),
        options.width,
        options.height,
    )?;

    for (requested_name, source_path) in sorted_sources {
        let loaded = load_export_sprite(&requested_name, &source_path, &options)?;
        let region = match atlas.insert(loaded.name.clone(), &loaded.image, options.extrude) {
            Ok(region) => region,
            Err(TextureAtlasError2D::OutOfSpace { .. }) if atlas.stats().region_count > 0 => {
                export_atlas_page(
                    output_dir,
                    &options,
                    page_index,
                    &atlas,
                    &mut manifest,
                    &mut output_files,
                )?;
                page_index += 1;
                atlas = DynamicTextureAtlas2D::new(
                    page_name(&options.output_prefix, page_index),
                    options.width,
                    options.height,
                )?;
                atlas.insert(loaded.name.clone(), &loaded.image, options.extrude)?
            }
            Err(error) => return Err(error),
        };
        let uv = atlas.uv_rect(&loaded.name).unwrap_or([0.0, 0.0, 0.0, 0.0]);
        manifest.regions.insert(
            loaded.name.clone(),
            SpriteAtlasRegionManifest2D {
                page: page_index,
                source: source_label(&source_path, options.source_root.as_deref()),
                x: region.x,
                y: region.y,
                w: region.width,
                h: region.height,
                original_w: loaded.original_width,
                original_h: loaded.original_height,
                offset_x: loaded.offset_x,
                offset_y: loaded.offset_y,
                extrude: region.extrude,
                uv,
            },
        );
        manifest.warnings.extend(loaded.warnings);
    }

    if atlas.stats().region_count > 0 {
        export_atlas_page(
            output_dir,
            &options,
            page_index,
            &atlas,
            &mut manifest,
            &mut output_files,
        )?;
    }

    let manifest_path = output_dir.join(format!("{}.spriteatlas.json", options.output_prefix));
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    output_files.push(manifest_path.clone());
    Ok(SpriteAtlasExportReport2D {
        manifest_path,
        output_files,
        manifest,
    })
}

impl Default for SpriteAtlasExportOptions2D {
    fn default() -> Self {
        Self {
            atlas_name: "Sprites".to_string(),
            width: 2048,
            height: 2048,
            extrude: 1,
            trim_transparent: true,
            power_of_two_pages: true,
            output_prefix: "Sprites".to_string(),
            source_root: None,
        }
    }
}

impl SpriteAtlasExportOptions2D {
    fn normalized(mut self) -> Result<Self, TextureAtlasError2D> {
        if self.power_of_two_pages {
            self.width = self.width.next_power_of_two();
            self.height = self.height.next_power_of_two();
        }
        if self.output_prefix.trim().is_empty() {
            self.output_prefix = sanitize_asset_name(&self.atlas_name);
        } else {
            self.output_prefix = sanitize_asset_name(&self.output_prefix);
        }
        if self.atlas_name.trim().is_empty() {
            self.atlas_name = self.output_prefix.clone();
        }
        if self.width == 0 || self.height == 0 {
            return Err(TextureAtlasError2D::InvalidAtlasSize {
                width: self.width,
                height: self.height,
            });
        }
        Ok(self)
    }
}

struct LoadedExportSprite2D {
    name: String,
    image: RgbaImage,
    original_width: u32,
    original_height: u32,
    offset_x: u32,
    offset_y: u32,
    warnings: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ExportAtlas2D {
    image: String,
    regions: BTreeMap<String, ExportRegion2D>,
}

#[derive(Serialize, Deserialize)]
struct ExportRegion2D {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

fn load_export_sprite(
    requested_name: &str,
    source_path: &Path,
    options: &SpriteAtlasExportOptions2D,
) -> Result<LoadedExportSprite2D, TextureAtlasError2D> {
    let original = image::open(source_path)?.to_rgba8();
    let original_width = original.width();
    let original_height = original.height();
    let mut warnings = Vec::new();
    let (image, offset_x, offset_y) = if options.trim_transparent {
        match trim_transparent(&original) {
            Some(trimmed) => trimmed,
            None => {
                warnings.push(format!(
                    "{} is fully transparent; exported as a 1x1 transparent sprite",
                    source_path.display()
                ));
                (RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])), 0, 0)
            }
        }
    } else {
        (original.clone(), 0, 0)
    };
    Ok(LoadedExportSprite2D {
        name: sanitize_asset_name(requested_name),
        image,
        original_width,
        original_height,
        offset_x,
        offset_y,
        warnings,
    })
}

fn export_atlas_page(
    output_dir: &Path,
    options: &SpriteAtlasExportOptions2D,
    page_index: usize,
    atlas: &DynamicTextureAtlas2D,
    manifest: &mut SpriteAtlasExportManifest2D,
    output_files: &mut Vec<PathBuf>,
) -> Result<(), TextureAtlasError2D> {
    let stem = page_name(&options.output_prefix, page_index);
    let png_path = output_dir.join(format!("{stem}.png"));
    let atlas_path = output_dir.join(format!("{stem}.atlas.json"));
    atlas.export(&png_path, &atlas_path)?;
    let stats = atlas.stats();
    manifest.pages.push(SpriteAtlasPageManifest2D {
        index: page_index,
        image: png_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| png_path.to_string_lossy().to_string()),
        atlas: atlas_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| atlas_path.to_string_lossy().to_string()),
        width: atlas.atlas().width,
        height: atlas.atlas().height,
        region_count: stats.region_count,
        occupancy: stats.occupancy,
    });
    output_files.push(png_path);
    output_files.push(atlas_path);
    Ok(())
}

fn trim_transparent(source: &RgbaImage) -> Option<(RgbaImage, u32, u32)> {
    let mut min_x = source.width();
    let mut min_y = source.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..source.height() {
        for x in 0..source.width() {
            if source.get_pixel(x, y).0[3] > 0 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if !found {
        return None;
    }
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let mut trimmed = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in 0..height {
        for x in 0..width {
            trimmed.put_pixel(x, y, *source.get_pixel(min_x + x, min_y + y));
        }
    }
    Some((trimmed, min_x, min_y))
}

fn source_label(path: &Path, root: Option<&Path>) -> String {
    root.and_then(|root| path.strip_prefix(root).ok())
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn page_name(prefix: &str, page_index: usize) -> String {
    format!("{}_{page_index:03}", sanitize_asset_name(prefix))
}

fn sanitize_asset_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "Sprite".to_string()
    } else {
        sanitized
    }
}

fn blit_with_extrusion(target: &mut RgbaImage, source: &RgbaImage, region: AtlasRegion2D) {
    let outer_x = region.x - region.extrude;
    let outer_y = region.y - region.extrude;
    let outer_width = region.width + region.extrude * 2;
    let outer_height = region.height + region.extrude * 2;
    let max_source_x = region.width - 1;
    let max_source_y = region.height - 1;

    for y in 0..outer_height {
        let source_y = y.saturating_sub(region.extrude).min(max_source_y);
        for x in 0..outer_width {
            let source_x = x.saturating_sub(region.extrude).min(max_source_x);
            target.put_pixel(
                outer_x + x,
                outer_y + y,
                *source.get_pixel(source_x, source_y),
            );
        }
    }
}

fn clear_rectangle(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32) {
    for pixel_y in y..y + height {
        for pixel_x in x..x + width {
            image.put_pixel(pixel_x, pixel_y, Rgba([0, 0, 0, 0]));
        }
    }
}

fn ensure_parent(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::asset_importers::AtlasImporter;

    fn solid(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(width, height, Rgba(color))
    }

    #[test]
    fn released_space_is_reused() {
        let mut atlas = DynamicTextureAtlas2D::new("sprites", 16, 8).unwrap();
        let first = atlas
            .insert("player", &solid(8, 8, [255, 0, 0, 255]), 0)
            .unwrap();
        atlas
            .insert("enemy", &solid(8, 8, [0, 255, 0, 255]), 0)
            .unwrap();
        assert!(matches!(
            atlas.insert("coin", &solid(1, 1, [255, 255, 0, 255]), 0),
            Err(TextureAtlasError2D::OutOfSpace { .. })
        ));

        assert_eq!(atlas.remove("player"), Some(first));
        let reused = atlas
            .insert("coin", &solid(8, 8, [255, 255, 0, 255]), 0)
            .unwrap();
        assert_eq!((reused.x, reused.y), (first.x, first.y));
        assert_eq!(atlas.stats().region_count, 2);
    }

    #[test]
    fn extrusion_repeats_edge_pixels() {
        let mut source = RgbaImage::new(2, 2);
        source.put_pixel(0, 0, Rgba([1, 0, 0, 255]));
        source.put_pixel(1, 0, Rgba([2, 0, 0, 255]));
        source.put_pixel(0, 1, Rgba([3, 0, 0, 255]));
        source.put_pixel(1, 1, Rgba([4, 0, 0, 255]));
        let mut atlas = DynamicTextureAtlas2D::new("sprites", 4, 4).unwrap();
        let region = atlas.insert("sprite", &source, 1).unwrap();

        assert_eq!(atlas.pixels().get_pixel(region.x - 1, region.y - 1).0[0], 1);
        assert_eq!(atlas.pixels().get_pixel(region.x + 2, region.y - 1).0[0], 2);
        assert_eq!(atlas.pixels().get_pixel(region.x - 1, region.y + 2).0[0], 3);
        assert_eq!(atlas.pixels().get_pixel(region.x + 2, region.y + 2).0[0], 4);
    }

    #[test]
    fn static_regions_reject_overlap() {
        let mut atlas = TextureAtlas2D {
            name: "manual".to_string(),
            width: 16,
            height: 16,
            regions: BTreeMap::new(),
        };
        assert!(atlas.add_region(
            "a",
            AtlasRegion2D {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
                extrude: 0
            }
        ));
        assert!(!atlas.add_region(
            "b",
            AtlasRegion2D {
                x: 4,
                y: 4,
                width: 8,
                height: 8,
                extrude: 0
            }
        ));
    }

    #[test]
    fn export_matches_existing_atlas_importer() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("miniforge-atlas-{unique}"));
        let png_path = directory.join("sprites_atlas.png");
        let json_path = directory.join("sprites_atlas.atlas.json");
        let mut atlas = DynamicTextureAtlas2D::new("sprites", 8, 8).unwrap();
        atlas
            .insert("player", &solid(2, 3, [255, 255, 255, 255]), 1)
            .unwrap();

        atlas.export(&png_path, &json_path).unwrap();
        let imported = AtlasImporter::load(&json_path).unwrap();
        assert_eq!(imported.image, "sprites_atlas.png");
        assert_eq!(imported.regions["player"].w, 2);
        assert_eq!(imported.regions["player"].h, 3);

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn paged_sprite_export_trims_writes_manifest_and_keeps_importer_sidecars() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("miniforge-paged-atlas-{unique}"));
        let source_dir = directory.join("sprites");
        let output_dir = directory.join("atlases");
        fs::create_dir_all(&source_dir).unwrap();

        let mut first = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
        first.put_pixel(3, 2, Rgba([255, 0, 0, 255]));
        first.put_pixel(4, 2, Rgba([255, 0, 0, 255]));
        first.save(source_dir.join("hero.png")).unwrap();
        solid(6, 6, [0, 255, 0, 255])
            .save(source_dir.join("enemy.png"))
            .unwrap();

        let report = export_sprite_atlas_pages_from_files(
            &[
                ("hero".to_string(), source_dir.join("hero.png")),
                ("enemy".to_string(), source_dir.join("enemy.png")),
            ],
            &output_dir,
            SpriteAtlasExportOptions2D {
                atlas_name: "GameplaySprites".to_string(),
                width: 8,
                height: 8,
                extrude: 1,
                output_prefix: "GameplaySprites".to_string(),
                source_root: Some(directory.clone()),
                power_of_two_pages: false,
                ..SpriteAtlasExportOptions2D::default()
            },
        )
        .unwrap();

        assert!(report.manifest_path.exists());
        assert!(report.manifest.pages.len() >= 2);
        assert_eq!(report.manifest.regions["hero"].original_w, 8);
        assert_eq!(report.manifest.regions["hero"].w, 2);
        assert_eq!(report.manifest.regions["hero"].offset_x, 3);
        assert!(
            report
                .output_files
                .iter()
                .any(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
        );
        assert!(
            report
                .output_files
                .iter()
                .any(|path| path.to_string_lossy().ends_with(".atlas.json"))
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
