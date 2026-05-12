use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ResourceManager {
    pub root: PathBuf,
    pub images: BTreeMap<String, PathBuf>,
    pub audio: BTreeMap<String, PathBuf>,
    pub data: BTreeMap<String, PathBuf>,
}

impl ResourceManager {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            images: BTreeMap::new(),
            audio: BTreeMap::new(),
            data: BTreeMap::new(),
        }
    }

    pub fn set_root(&mut self, root: impl AsRef<Path>) {
        self.root = root.as_ref().to_path_buf();
    }

    pub fn scan_all(&mut self) -> io::Result<()> {
        self.scan_sprites()?;
        self.scan_audio()?;
        self.scan_data()?;
        Ok(())
    }

    pub fn scan_sprites(&mut self) -> io::Result<()> {
        self.scan_into(
            "sprites",
            &["png", "jpg", "jpeg", "bmp", "gif", "webp"],
            ResourceKind::Image,
        )
    }

    pub fn scan_audio(&mut self) -> io::Result<()> {
        self.scan_into("audio", &["wav", "mp3", "ogg"], ResourceKind::Audio)
    }

    pub fn scan_data(&mut self) -> io::Result<()> {
        self.scan_into("data", &["json", "txt", "csv"], ResourceKind::Data)
    }

    fn scan_into(
        &mut self,
        folder: &str,
        extensions: &[&str],
        kind: ResourceKind,
    ) -> io::Result<()> {
        let start = self.root.join(folder);
        if !start.exists() {
            return Ok(());
        }
        for path in walk_files(&start)? {
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !extensions.iter().any(|allowed| *allowed == ext) {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("asset")
                .to_string();
            let rel = path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf();
            match kind {
                ResourceKind::Image => {
                    self.images.insert(name, rel);
                }
                ResourceKind::Audio => {
                    self.audio.insert(name, rel);
                }
                ResourceKind::Data => {
                    self.data.insert(name, rel);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum ResourceKind {
    Image,
    Audio,
    Data,
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
