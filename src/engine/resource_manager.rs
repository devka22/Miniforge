use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct ResourceManager {
    pub root: PathBuf,
    pub images: BTreeMap<String, PathBuf>,
    pub audio: BTreeMap<String, PathBuf>,
    pub data: BTreeMap<String, PathBuf>,
    pub catalog: BTreeMap<String, ResourceEntry>,
    pub cache: BTreeMap<String, ResourceEntry>,
    pub load_requests: BTreeMap<String, ResourceLoadRequest>,
}

impl ResourceManager {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            images: BTreeMap::new(),
            audio: BTreeMap::new(),
            data: BTreeMap::new(),
            catalog: BTreeMap::new(),
            cache: BTreeMap::new(),
            load_requests: BTreeMap::new(),
        }
    }

    pub fn set_root(&mut self, root: impl AsRef<Path>) {
        self.root = root.as_ref().to_path_buf();
    }

    pub fn scan_all(&mut self) -> io::Result<()> {
        self.clear();
        self.scan_into(
            "sprites",
            &["png", "jpg", "jpeg", "bmp", "gif", "webp"],
            ResourceKind::Image,
        )?;
        self.scan_into("audio", &["wav", "mp3", "ogg", "json"], ResourceKind::Audio)?;
        self.scan_into("data", &["json", "txt", "csv"], ResourceKind::Data)?;
        self.scan_into("prefabs", &["prefab"], ResourceKind::Prefab)?;
        self.scan_into("ui", &["mfui", "ui2d", "json"], ResourceKind::UiDocument)?;
        Ok(())
    }

    pub fn scan_sprites(&mut self) -> io::Result<()> {
        self.clear_kind(ResourceKind::Image);
        self.scan_into(
            "sprites",
            &["png", "jpg", "jpeg", "bmp", "gif", "webp"],
            ResourceKind::Image,
        )
    }

    pub fn scan_audio(&mut self) -> io::Result<()> {
        self.clear_kind(ResourceKind::Audio);
        self.scan_into("audio", &["wav", "mp3", "ogg", "json"], ResourceKind::Audio)
    }

    pub fn scan_data(&mut self) -> io::Result<()> {
        self.clear_kind(ResourceKind::Data);
        self.scan_into("data", &["json", "txt", "csv"], ResourceKind::Data)
    }

    pub fn scan_project_resources(project_path: impl AsRef<Path>) -> io::Result<Self> {
        let project_path = project_path.as_ref();
        let paths = AssetTools::get_project_paths(project_path);
        let mut manager = Self::new(project_path);
        manager.scan_into_path(
            &paths.sprites,
            &["png", "jpg", "jpeg", "bmp", "gif", "webp"],
            ResourceKind::Image,
            project_path,
        )?;
        manager.scan_into_path(
            &paths.audio,
            &["wav", "mp3", "ogg", "json"],
            ResourceKind::Audio,
            project_path,
        )?;
        manager.scan_into_path(
            &paths.data,
            &["json", "txt", "csv", "ron", "toml"],
            ResourceKind::Data,
            project_path,
        )?;
        manager.scan_into_path(
            &paths.prefabs,
            &["prefab"],
            ResourceKind::Prefab,
            project_path,
        )?;
        manager.scan_into_path(
            &paths.scripts,
            &["luau"],
            ResourceKind::Script,
            project_path,
        )?;
        manager.scan_into_path(
            &paths.visual_graphs,
            &["mfgraph"],
            ResourceKind::VisualGraph,
            project_path,
        )?;
        manager.scan_into_path(&paths.scenes, &["scene"], ResourceKind::Scene, project_path)?;
        manager.scan_into_path(
            &paths.assets.join("ui"),
            &["mfui", "json"],
            ResourceKind::UiDocument,
            project_path,
        )?;
        Ok(manager)
    }

    pub fn find(&self, kind: ResourceKind, name: &str) -> Option<&ResourceEntry> {
        self.catalog
            .values()
            .find(|entry| entry.kind == kind && entry.name == name)
    }

    pub fn entries_by_kind(&self, kind: ResourceKind) -> Vec<&ResourceEntry> {
        self.catalog
            .values()
            .filter(|entry| entry.kind == kind)
            .collect()
    }

    pub fn report(&self) -> ResourceReport {
        let mut counts = BTreeMap::new();
        let mut total_bytes = 0;
        let mut by_name = BTreeMap::<String, Vec<String>>::new();
        for entry in self.catalog.values() {
            *counts.entry(entry.kind.label().to_string()).or_insert(0) += 1;
            total_bytes += entry.size_bytes;
            by_name
                .entry(format!("{}:{}", entry.kind.label(), entry.name))
                .or_default()
                .push(entry.relative_path.to_string_lossy().to_string());
        }
        let duplicates = by_name
            .into_iter()
            .filter_map(|(key, mut paths)| {
                if paths.len() < 2 {
                    return None;
                }
                paths.sort();
                Some(ResourceDuplicate { key, paths })
            })
            .collect();
        ResourceReport {
            total_files: self.catalog.len(),
            total_bytes,
            counts,
            duplicates,
        }
    }

    pub fn request_load(
        &mut self,
        id: impl Into<String>,
        type_hint: impl Into<String>,
        cache_mode: ResourceCacheMode,
    ) -> ResourceLoadRequest {
        let id = id.into();
        let request = ResourceLoadRequest {
            id: id.clone(),
            type_hint: type_hint.into(),
            cache_mode,
            status: ResourceLoadStatus::Queued,
            progress: 0.0,
            message: "queued".to_string(),
        };
        self.load_requests.insert(id, request.clone());
        request
    }

    pub fn request_load_by_kind(
        &mut self,
        kind: ResourceKind,
        name: &str,
        cache_mode: ResourceCacheMode,
    ) -> Option<ResourceLoadRequest> {
        let entry = self.find(kind, name)?.clone();
        Some(self.request_load(entry.id, kind.label(), cache_mode))
    }

    pub fn process_load_queue(&mut self) -> Vec<ResourceLoadRequest> {
        let ids = self.load_requests.keys().cloned().collect::<Vec<_>>();
        let mut processed = Vec::new();
        for id in ids {
            let Some(mut request) = self.load_requests.get(&id).cloned() else {
                continue;
            };
            if request.status == ResourceLoadStatus::Loaded {
                processed.push(request);
                continue;
            }
            if request.cache_mode == ResourceCacheMode::Reuse && self.cache.contains_key(&id) {
                request.status = ResourceLoadStatus::Loaded;
                request.progress = 1.0;
                request.message = "cache_hit".to_string();
            } else if let Some(entry) = self.catalog.get(&id).cloned() {
                if request.cache_mode != ResourceCacheMode::Ignore {
                    self.cache.insert(id.clone(), entry);
                }
                request.status = ResourceLoadStatus::Loaded;
                request.progress = 1.0;
                request.message = "loaded".to_string();
            } else {
                request.status = ResourceLoadStatus::Missing;
                request.progress = 1.0;
                request.message = "missing_resource".to_string();
            }
            self.load_requests.insert(id, request.clone());
            processed.push(request);
        }
        processed
    }

    pub fn load_status(&self, id: &str) -> Option<ResourceLoadStatus> {
        self.load_requests.get(id).map(|request| request.status)
    }

    pub fn cached_entries_by_kind(&self, kind: ResourceKind) -> Vec<&ResourceEntry> {
        self.cache
            .values()
            .filter(|entry| entry.kind == kind)
            .collect()
    }

    fn scan_into(
        &mut self,
        folder: &str,
        extensions: &[&str],
        kind: ResourceKind,
    ) -> io::Result<()> {
        let start = self.root.join(folder);
        let base = self.root.clone();
        self.scan_into_path(&start, extensions, kind, &base)
    }

    fn scan_into_path(
        &mut self,
        start: &Path,
        extensions: &[&str],
        kind: ResourceKind,
        base: &Path,
    ) -> io::Result<()> {
        if !start.exists() {
            return Ok(());
        }
        for path in walk_files(start)? {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !extensions.iter().any(|allowed| *allowed == extension) {
                continue;
            }
            self.insert_resource(path, extension, kind, base);
        }
        Ok(())
    }

    fn insert_resource(
        &mut self,
        path: PathBuf,
        extension: String,
        kind: ResourceKind,
        base: &Path,
    ) {
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("asset")
            .to_string();
        let relative_path = path.strip_prefix(base).unwrap_or(&path).to_path_buf();
        let size_bytes = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        let id = format!("{}:{}", kind.label(), relative_path.to_string_lossy());
        let entry = ResourceEntry {
            id: id.clone(),
            name: name.clone(),
            kind,
            relative_path: relative_path.clone(),
            extension,
            labels: kind.default_labels(&relative_path),
            size_bytes,
        };
        match kind {
            ResourceKind::Image => {
                self.images.insert(name, relative_path);
            }
            ResourceKind::Audio => {
                self.audio.insert(name, relative_path);
            }
            ResourceKind::Data => {
                self.data.insert(name, relative_path);
            }
            _ => {}
        }
        self.catalog.insert(id, entry);
    }

    fn clear(&mut self) {
        self.images.clear();
        self.audio.clear();
        self.data.clear();
        self.catalog.clear();
        self.cache.clear();
        self.load_requests.clear();
    }

    fn clear_kind(&mut self, kind: ResourceKind) {
        match kind {
            ResourceKind::Image => self.images.clear(),
            ResourceKind::Audio => self.audio.clear(),
            ResourceKind::Data => self.data.clear(),
            _ => {}
        }
        self.catalog.retain(|_, entry| entry.kind != kind);
        self.cache.retain(|_, entry| entry.kind != kind);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceCacheMode {
    Ignore,
    Reuse,
    Replace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceLoadStatus {
    Queued,
    Loaded,
    Missing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLoadRequest {
    pub id: String,
    pub type_hint: String,
    pub cache_mode: ResourceCacheMode,
    pub status: ResourceLoadStatus,
    pub progress: f32,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    Image,
    Audio,
    Data,
    Script,
    VisualGraph,
    Prefab,
    Scene,
    UiDocument,
}

impl ResourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Data => "data",
            Self::Script => "script",
            Self::VisualGraph => "visual_graph",
            Self::Prefab => "prefab",
            Self::Scene => "scene",
            Self::UiDocument => "ui_document",
        }
    }

    fn default_labels(self, path: &Path) -> Vec<String> {
        let mut labels = vec![self.label().to_string()];
        let text = path.to_string_lossy().to_ascii_lowercase();
        for label in ["sprite", "ui", "rts", "player", "enemy", "level", "audio"] {
            if text.contains(label) && !labels.iter().any(|item| item == label) {
                labels.push(label.to_string());
            }
        }
        labels
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceEntry {
    pub id: String,
    pub name: String,
    pub kind: ResourceKind,
    pub relative_path: PathBuf,
    pub extension: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceReport {
    pub total_files: usize,
    pub total_bytes: u64,
    #[serde(default)]
    pub counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub duplicates: Vec<ResourceDuplicate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceDuplicate {
    pub key: String,
    pub paths: Vec<String>,
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
    files.sort();
    Ok(files)
}
