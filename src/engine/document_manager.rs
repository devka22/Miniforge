use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::project_storage::{BackupPolicy, DEFAULT_BACKUP_GENERATIONS, ProjectStorage};
use crate::engine::tab_manager::TabManager;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentKind {
    Script,
    VisualGraph,
    Scene,
    Prefab,
    Ui,
    Material,
    Shader,
    Json,
    Text,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorDocument {
    pub path: PathBuf,
    pub kind: DocumentKind,
    pub text: String,
    pub dirty: bool,
    pub syntax_error: Option<String>,
    pub language: String,
    pub last_saved_backup: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseDocumentChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CloseDocumentOutcome {
    pub closed: bool,
    pub cancelled: bool,
    pub active: Option<PathBuf>,
    pub saved: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentManager {
    pub documents: BTreeMap<PathBuf, EditorDocument>,
    pub tabs: TabManager,
}

impl DocumentKind {
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str()
        {
            "luau" => Self::Script,
            "mfgraph" => Self::VisualGraph,
            "scene" => Self::Scene,
            "prefab" => Self::Prefab,
            "mfui" | "ui2d" => Self::Ui,
            "material" => Self::Material,
            "shader" => Self::Shader,
            "json" => Self::Json,
            "txt" | "md" => Self::Text,
            _ => Self::Unknown,
        }
    }
}

impl EditorDocument {
    pub fn from_disk(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let text = fs::read_to_string(&path)?;
        Ok(Self::from_text(path, text, false))
    }

    pub fn from_text(path: PathBuf, text: impl Into<String>, dirty: bool) -> Self {
        let kind = DocumentKind::from_path(&path);
        Self {
            language: language_for_kind(kind, &path),
            path,
            kind,
            text: text.into(),
            dirty,
            syntax_error: None,
            last_saved_backup: None,
        }
    }

    pub fn backup_path(&self) -> PathBuf {
        backup_path_for(&self.path)
    }
}

impl DocumentManager {
    pub fn open(&mut self, path: impl AsRef<Path>) -> io::Result<EditorDocument> {
        let path = path.as_ref().to_path_buf();
        let document = if let Some(document) = self.documents.get(&path) {
            document.clone()
        } else {
            EditorDocument::from_disk(&path)?
        };
        self.documents.insert(path.clone(), document.clone());
        self.tabs.open(path);
        Ok(document)
    }

    pub fn upsert(&mut self, document: EditorDocument) {
        self.tabs.open(document.path.clone());
        self.documents.insert(document.path.clone(), document);
    }

    pub fn active(&self) -> Option<&EditorDocument> {
        self.tabs
            .active
            .as_ref()
            .and_then(|path| self.documents.get(path))
    }

    pub fn active_mut(&mut self) -> Option<&mut EditorDocument> {
        let path = self.tabs.active.clone()?;
        self.documents.get_mut(&path)
    }

    pub fn get(&self, path: impl AsRef<Path>) -> Option<&EditorDocument> {
        self.documents.get(path.as_ref())
    }

    pub fn is_dirty(&self, path: impl AsRef<Path>) -> bool {
        self.documents
            .get(path.as_ref())
            .is_some_and(|document| document.dirty)
    }

    pub fn mark_dirty(&mut self, path: impl AsRef<Path>, dirty: bool) -> bool {
        let Some(document) = self.documents.get_mut(path.as_ref()) else {
            return false;
        };
        document.dirty = dirty;
        true
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> io::Result<PathBuf> {
        let path = path.as_ref().to_path_buf();
        let Some(document) = self.documents.get_mut(&path) else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Documento no abierto: {}", path.display()),
            ));
        };
        let existed = path.exists();
        if existed {
            let backup = document.backup_path();
            ProjectStorage::write_atomic_with_backup(
                &path,
                document.text.as_bytes(),
                BackupPolicy::new(&backup, DEFAULT_BACKUP_GENERATIONS),
            )
            .map_err(io::Error::from)?;
            document.last_saved_backup = Some(backup);
        } else {
            ProjectStorage::write_atomic(&path, document.text.as_bytes())
                .map_err(io::Error::from)?;
        }
        document.dirty = false;
        Ok(path)
    }

    pub fn close(
        &mut self,
        path: impl AsRef<Path>,
        choice: CloseDocumentChoice,
    ) -> io::Result<CloseDocumentOutcome> {
        let path = path.as_ref().to_path_buf();
        if !self.documents.contains_key(&path) && !self.tabs.tabs.contains(&path) {
            return Ok(CloseDocumentOutcome {
                active: self.tabs.active.clone(),
                ..Default::default()
            });
        }
        if choice == CloseDocumentChoice::Cancel {
            return Ok(CloseDocumentOutcome {
                cancelled: true,
                active: self.tabs.active.clone(),
                ..Default::default()
            });
        }
        let mut saved = false;
        if self.is_dirty(&path) {
            match choice {
                CloseDocumentChoice::Save => {
                    self.save(&path)?;
                    saved = true;
                }
                CloseDocumentChoice::Discard => {}
                CloseDocumentChoice::Cancel => unreachable!("cancel handled above"),
            }
        }
        self.documents.remove(&path);
        let active = self.tabs.close(&path);
        Ok(CloseDocumentOutcome {
            closed: true,
            cancelled: false,
            active,
            saved,
        })
    }

    pub fn close_active(
        &mut self,
        choice: CloseDocumentChoice,
    ) -> io::Result<CloseDocumentOutcome> {
        let Some(path) = self.tabs.active.clone() else {
            return Ok(CloseDocumentOutcome::default());
        };
        self.close(path, choice)
    }
}

fn language_for_kind(kind: DocumentKind, path: &Path) -> String {
    match kind {
        DocumentKind::Script => "luau".to_string(),
        DocumentKind::VisualGraph => "visual_graph".to_string(),
        DocumentKind::Scene => "scene_json".to_string(),
        DocumentKind::Prefab => "prefab_json".to_string(),
        DocumentKind::Ui => "ui_json".to_string(),
        DocumentKind::Material => "material_json".to_string(),
        DocumentKind::Shader => "shader".to_string(),
        DocumentKind::Json => "json".to_string(),
        DocumentKind::Text => "text".to_string(),
        DocumentKind::Unknown => path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("text")
            .to_string(),
    }
}

pub fn backup_path_for(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    path.with_file_name(format!("{filename}.bak"))
}

pub fn write_text_atomic(path: &Path, text: &str) -> io::Result<()> {
    ProjectStorage::write_atomic(path, text.as_bytes())
        .map(|_| ())
        .map_err(io::Error::from)
}
