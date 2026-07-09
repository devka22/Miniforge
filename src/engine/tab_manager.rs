use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabManager {
    pub tabs: Vec<PathBuf>,
    pub active: Option<PathBuf>,
    pub closed_tabs: Vec<PathBuf>,
    pub max_closed_tabs: usize,
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            closed_tabs: Vec::new(),
            max_closed_tabs: 16,
        }
    }

    pub fn open(&mut self, path: impl Into<PathBuf>) -> PathBuf {
        let path = path.into();
        if !self.tabs.contains(&path) {
            self.tabs.push(path.clone());
        }
        self.closed_tabs.retain(|closed| closed != &path);
        self.active = Some(path.clone());
        path
    }

    pub fn activate(&mut self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        let tab = self.tabs.iter().find(|tab| tab.as_path() == path)?.clone();
        self.active = Some(tab.clone());
        Some(tab)
    }

    pub fn activate_relative(&mut self, direction: i32) -> Option<PathBuf> {
        if self.tabs.is_empty() {
            self.active = None;
            return None;
        }
        let current = self
            .active
            .as_ref()
            .and_then(|path| self.tabs.iter().position(|tab| tab == path))
            .unwrap_or(0);
        let len = self.tabs.len() as i32;
        let next = (current as i32 + direction).rem_euclid(len) as usize;
        let path = self.tabs[next].clone();
        self.active = Some(path.clone());
        Some(path)
    }

    pub fn close(&mut self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        let index = self.tabs.iter().position(|tab| tab.as_path() == path)?;
        let closed = self.tabs.remove(index);
        self.closed_tabs.push(closed);
        let max_closed = self.max_closed_tabs.max(1);
        if self.closed_tabs.len() > max_closed {
            let overflow = self.closed_tabs.len() - max_closed;
            self.closed_tabs.drain(0..overflow);
        }

        if self.active.as_deref() == Some(path) {
            self.active = self
                .tabs
                .get(
                    index
                        .saturating_sub(1)
                        .min(self.tabs.len().saturating_sub(1)),
                )
                .cloned();
        }
        self.active.clone()
    }

    pub fn reopen_last_closed(&mut self) -> Option<PathBuf> {
        let path = self.closed_tabs.pop()?;
        Some(self.open(path))
    }

    pub fn sync_from_paths(&mut self, tabs: &[PathBuf], active: Option<PathBuf>) {
        self.tabs = tabs.to_vec();
        self.active = active.filter(|path| self.tabs.contains(path));
    }
}
