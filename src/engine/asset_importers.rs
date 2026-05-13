//! Import helpers for sprite sheets, texture atlases, and lightweight audio waveform previews.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSlice {
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSheetMetadata {
    pub source: String,
    pub image_width: u32,
    pub image_height: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub margin: u32,
    pub padding: u32,
    pub slices: Vec<SpriteSlice>,
}

#[derive(Debug, Default)]
pub struct SpriteSheetImporter;

impl SpriteSheetImporter {
    pub fn build_metadata(
        image_path: &Path,
        cell_width: u32,
        cell_height: u32,
        margin: u32,
        padding: u32,
    ) -> io::Result<SpriteSheetMetadata> {
        let (iw, ih) = image_dimensions(image_path)?;
        if cell_width == 0 || cell_height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cell_width/cell_height deben ser > 0",
            ));
        }
        let step_x = cell_width + margin + padding;
        let step_y = cell_height + margin + padding;
        let mut slices = Vec::new();
        let mut index = 0usize;
        let mut y = margin;
        while y + cell_height <= ih {
            let mut x = margin;
            while x + cell_width <= iw {
                slices.push(SpriteSlice {
                    index,
                    x,
                    y,
                    width: cell_width,
                    height: cell_height,
                });
                index += 1;
                x += step_x;
            }
            y += step_y;
        }
        Ok(SpriteSheetMetadata {
            source: image_path.to_string_lossy().to_string(),
            image_width: iw,
            image_height: ih,
            cell_width,
            cell_height,
            margin,
            padding,
            slices,
        })
    }

    pub fn write_sidecar(image_path: &Path, meta: &SpriteSheetMetadata) -> io::Result<PathBuf> {
        let stem = image_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("sheet");
        let sidecar = image_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}.spritesheet.json"));
        AssetTools::write_json(&sidecar, &serde_json::to_value(meta).unwrap_or(json!({})))?;
        Ok(sidecar)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AtlasRegion {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AtlasFile {
    pub image: String,
    pub regions: std::collections::BTreeMap<String, AtlasRegion>,
}

#[derive(Debug, Default)]
pub struct AtlasImporter;

impl AtlasImporter {
    pub fn load(atlas_json: &Path) -> io::Result<AtlasFile> {
        let v = AssetTools::read_json(atlas_json)?;
        serde_json::from_value(v).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn validate_image(project_root: &Path, atlas: &AtlasFile) -> io::Result<PathBuf> {
        let p = project_root.join(&atlas.image);
        if !p.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Atlas image missing: {}", atlas.image),
            ));
        }
        Ok(p)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WaveformCache {
    pub cache_dir: PathBuf,
}

impl WaveformCache {
    pub fn new(project_root: &Path) -> Self {
        Self {
            cache_dir: project_root.join(".miniforge_cache").join("waveforms"),
        }
    }

    fn key_for(path: &Path) -> u64 {
        let mut h = DefaultHasher::new();
        path.to_string_lossy().hash(&mut h);
        h.finish()
    }

    pub fn peaks_for_wav(&self, wav_path: &Path, bucket_count: usize) -> io::Result<Vec<f32>> {
        fs::create_dir_all(&self.cache_dir)?;
        let cache_file = self
            .cache_dir
            .join(format!("wf_{:016x}.json", Self::key_for(wav_path)));
        if cache_file.exists() {
            if let Ok(v) = AssetTools::read_json(&cache_file) {
                if let Some(arr) = v.get("peaks").and_then(|p| p.as_array()) {
                    let peaks: Vec<f32> = arr
                        .iter()
                        .filter_map(|x| x.as_f64().map(|f| f as f32))
                        .collect();
                    if peaks.len() == bucket_count {
                        return Ok(peaks);
                    }
                }
            }
        }
        let peaks = compute_wav_peaks(wav_path, bucket_count)?;
        AssetTools::write_json(
            &cache_file,
            &json!({"path": wav_path.to_string_lossy(), "peaks": peaks}),
        )?;
        Ok(peaks)
    }
}

fn read_i16_le(data: &[u8], off: usize) -> Option<i16> {
    let bytes: [u8; 2] = data.get(off..off + 2)?.try_into().ok()?;
    Some(i16::from_le_bytes(bytes))
}

fn compute_wav_peaks(path: &Path, buckets: usize) -> io::Result<Vec<f32>> {
    let mut file = fs::File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    if buf.len() < 44 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Solo se soporta WAV PCM simple para preview",
        ));
    }
    let mut offset = 12usize;
    let mut channels = 1u16;
    let mut bits = 16u16;
    let mut data_len = 0usize;
    let mut data_off = 0usize;
    while offset + 8 <= buf.len() {
        let chunk_id = &buf[offset..offset + 4];
        let size = u32::from_le_bytes(buf[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        if chunk_id == b"fmt " {
            channels = u16::from_le_bytes(buf[offset + 2..offset + 4].try_into().unwrap());
            bits = u16::from_le_bytes(buf[offset + 14..offset + 16].try_into().unwrap());
        } else if chunk_id == b"data" {
            data_len = size;
            data_off = offset;
            break;
        }
        offset += size;
    }
    if data_len == 0 || bits != 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Waveform preview requiere WAV PCM 16-bit",
        ));
    }
    let sample_bytes = data_len;
    let total_samples = sample_bytes / ((bits / 8) as usize) / (channels as usize);
    let bucket_samples = (total_samples / buckets.max(1)).max(1);
    let mut peaks = vec![0.0f32; buckets];
    let mut i = 0usize;
    let mut b = 0usize;
    while b < buckets && i < total_samples {
        let start = i;
        let end = (i + bucket_samples).min(total_samples);
        let mut acc = 0.0f32;
        let mut count = 0usize;
        let mut j = start;
        while j < end && data_off + j * 2 * (channels as usize) + 1 < buf.len() {
            let off = data_off + j * 2 * (channels as usize);
            if let Some(v) = read_i16_le(&buf, off) {
                acc += (v as f32).abs();
                count += 1;
            }
            j += 1;
        }
        peaks[b] = if count > 0 {
            acc / (count as f32 * 32768.0)
        } else {
            0.0
        };
        i = end;
        b += 1;
    }
    Ok(peaks)
}

fn image_dimensions(path: &Path) -> io::Result<(u32, u32)> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" => read_png_dimensions(path),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "SpriteSheetImporter: dimensiones solo desde PNG (IHDR). Convierte a PNG o amplía el importador.",
        )),
    }
}

fn read_png_dimensions(path: &Path) -> io::Result<(u32, u32)> {
    let mut f = fs::File::open(path)?;
    let mut sig = [0u8; 8];
    f.read_exact(&mut sig)?;
    if sig != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cabecera PNG inválida",
        ));
    }
    let mut len_buf = [0u8; 4];
    f.read_exact(&mut len_buf)?;
    let _chunk_len = u32::from_be_bytes(len_buf);
    let mut typ = [0u8; 4];
    f.read_exact(&mut typ)?;
    if typ != *b"IHDR" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Primer chunk PNG debe ser IHDR",
        ));
    }
    let mut hdr = [0u8; 13];
    f.read_exact(&mut hdr)?;
    let w = u32::from_be_bytes(hdr[0..4].try_into().unwrap());
    let h = u32::from_be_bytes(hdr[4..8].try_into().unwrap());
    Ok((w, h))
}
