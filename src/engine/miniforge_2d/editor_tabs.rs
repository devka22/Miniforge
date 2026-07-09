use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocumentKind2D {
    Scene,
    LuauScript,
    VisualGraph,
    Prefab,
    Material,
    UiDocument,
    Timeline,
    JsonConfig,
    Log,
    BuildOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorTab2D {
    pub id: String,
    pub title: String,
    pub path: String,
    pub kind: DocumentKind2D,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorTabGroup2D {
    pub id: String,
    pub title: String,
    pub tab_ids: Vec<String>,
    pub active: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorTabSession2D {
    pub tabs: Vec<EditorTab2D>,
    pub active: Option<String>,
    pub closed: Vec<EditorTab2D>,
}

impl EditorTabSession2D {
    pub fn open(&mut self, path: impl Into<String>) -> String {
        let path = path.into();
        let id = path.clone();
        if !self.tabs.iter().any(|tab| tab.id == id) {
            self.tabs.push(EditorTab2D {
                id: id.clone(),
                title: title_for_path(&path),
                kind: kind_for_path(&path),
                path,
                dirty: false,
            });
        }
        self.active = Some(id.clone());
        id
    }

    pub fn open_in_group(&mut self, path: impl Into<String>, group_id: &str) -> String {
        let id = self.open(path);
        let target_kind = kind_for_group(group_id);
        if let Some(kind) = target_kind
            && let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id)
        {
            tab.kind = kind;
        }
        id
    }

    pub fn close(&mut self, id: &str, force: bool) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        if self.tabs[index].dirty && !force {
            return false;
        }
        let closed = self.tabs.remove(index);
        self.closed.push(closed);
        if self.active.as_deref() == Some(id) {
            self.active = self.tabs.last().map(|tab| tab.id.clone());
        }
        true
    }

    pub fn reopen_last_closed(&mut self) -> Option<String> {
        let tab = self.closed.pop()?;
        let id = tab.id.clone();
        self.tabs.push(tab);
        self.active = Some(id.clone());
        Some(id)
    }

    pub fn reorder(&mut self, id: &str, new_index: usize) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return false;
        };
        let tab = self.tabs.remove(index);
        self.tabs.insert(new_index.min(self.tabs.len()), tab);
        true
    }

    pub fn mark_dirty(&mut self, id: &str, dirty: bool) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) else {
            return false;
        };
        tab.dirty = dirty;
        true
    }

    pub fn grouped_tabs(&self) -> Vec<EditorTabGroup2D> {
        let mut groups = vec![
            EditorTabGroup2D {
                id: "scenes".to_string(),
                title: "Scenes".to_string(),
                tab_ids: Vec::new(),
                active: None,
            },
            EditorTabGroup2D {
                id: "scripts".to_string(),
                title: "Scripts".to_string(),
                tab_ids: Vec::new(),
                active: None,
            },
            EditorTabGroup2D {
                id: "assets".to_string(),
                title: "Assets".to_string(),
                tab_ids: Vec::new(),
                active: None,
            },
            EditorTabGroup2D {
                id: "output".to_string(),
                title: "Output".to_string(),
                tab_ids: Vec::new(),
                active: None,
            },
        ];
        for tab in &self.tabs {
            let group_id = group_for_kind(&tab.kind);
            let Some(group) = groups.iter_mut().find(|group| group.id == group_id) else {
                continue;
            };
            group.tab_ids.push(tab.id.clone());
            if self.active.as_deref() == Some(tab.id.as_str()) {
                group.active = Some(tab.id.clone());
            }
        }
        groups
    }

    pub fn tabs_for_group(&self, group_id: &str) -> Vec<&EditorTab2D> {
        self.tabs
            .iter()
            .filter(|tab| group_for_kind(&tab.kind) == group_id)
            .collect()
    }

    pub fn save_session(&self, path: impl AsRef<Path>) -> io::Result<()> {
        AssetTools::write_json(path, &serde_json::to_value(self).unwrap_or_default())
    }

    pub fn load_session(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let value = AssetTools::read_json(path)?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }
}

pub fn group_for_kind(kind: &DocumentKind2D) -> &'static str {
    match kind {
        DocumentKind2D::Scene | DocumentKind2D::Prefab => "scenes",
        DocumentKind2D::LuauScript | DocumentKind2D::VisualGraph => "scripts",
        DocumentKind2D::Material
        | DocumentKind2D::UiDocument
        | DocumentKind2D::Timeline
        | DocumentKind2D::JsonConfig => "assets",
        DocumentKind2D::Log | DocumentKind2D::BuildOutput => "output",
    }
}

fn kind_for_group(group_id: &str) -> Option<DocumentKind2D> {
    match group_id {
        "scenes" => Some(DocumentKind2D::Scene),
        "scripts" => Some(DocumentKind2D::LuauScript),
        "assets" => Some(DocumentKind2D::JsonConfig),
        "output" => Some(DocumentKind2D::BuildOutput),
        _ => None,
    }
}

pub fn kind_for_path(path: &str) -> DocumentKind2D {
    match std::path::Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
    {
        "scene" => DocumentKind2D::Scene,
        "luau" => DocumentKind2D::LuauScript,
        "mfgraph" => DocumentKind2D::VisualGraph,
        "prefab" => DocumentKind2D::Prefab,
        "mat" | "material" => DocumentKind2D::Material,
        "mfui" | "ui2d" => DocumentKind2D::UiDocument,
        "mftime" | "seq2d" => DocumentKind2D::Timeline,
        "log" => DocumentKind2D::Log,
        "txt" if path.contains("build") => DocumentKind2D::BuildOutput,
        _ => DocumentKind2D::JsonConfig,
    }
}

fn title_for_path(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_string()
}
