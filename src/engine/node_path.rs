use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodePath {
    absolute: bool,
    segments: Vec<String>,
}

impl NodePath {
    pub fn root() -> Self {
        Self {
            absolute: true,
            segments: Vec::new(),
        }
    }

    pub fn current() -> Self {
        Self {
            absolute: false,
            segments: Vec::new(),
        }
    }

    pub fn new(absolute: bool, segments: Vec<String>) -> Self {
        Self { absolute, segments }.normalized()
    }

    pub fn parse(path: &str) -> Result<Self, NodePathError> {
        Self::from_str(path)
    }

    pub fn is_absolute(&self) -> bool {
        self.absolute
    }

    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    pub fn is_current(&self) -> bool {
        !self.absolute && self.segments.is_empty()
    }

    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            return None;
        }
        let mut segments = self.segments.clone();
        segments.pop();
        Some(Self {
            absolute: self.absolute,
            segments,
        })
    }

    pub fn join(&self, child: impl AsRef<str>) -> Self {
        let mut segments = self.segments.clone();
        segments.extend(parse_segments(child.as_ref()));
        Self {
            absolute: self.absolute,
            segments,
        }
        .normalized()
    }

    pub fn normalized(mut self) -> Self {
        let mut next = Vec::new();
        for segment in self.segments {
            match segment.as_str() {
                "" | "." => {}
                ".." if !next.is_empty() && next.last().is_some_and(|item| item != "..") => {
                    next.pop();
                }
                ".." if self.absolute => {}
                _ => next.push(segment),
            }
        }
        self.segments = next;
        self
    }

    pub fn to_slash_path(&self) -> String {
        if self.absolute && self.segments.is_empty() {
            return "/".to_string();
        }
        let body = self.segments.join("/");
        if self.absolute {
            format!("/{body}")
        } else if body.is_empty() {
            ".".to_string()
        } else {
            body
        }
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_slash_path())
    }
}

impl FromStr for NodePath {
    type Err = NodePathError;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        let path = path.trim();
        if path.is_empty() {
            return Err(NodePathError::Empty);
        }
        let absolute = path.starts_with('/');
        let body = if absolute { &path[1..] } else { path };
        Ok(Self {
            absolute,
            segments: parse_segments(body),
        }
        .normalized())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodePathError {
    Empty,
}

impl fmt::Display for NodePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("node path is empty"),
        }
    }
}

impl std::error::Error for NodePathError {}

fn parse_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| segment.trim().to_string())
        .collect()
}

pub fn node_path_segment(name: &str, entity_id: u64) -> String {
    let mut segment = name
        .trim()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' => '_',
            _ => ch,
        })
        .collect::<String>();
    if segment.is_empty() {
        segment = format!("Entity{entity_id}");
    }
    segment
}

#[cfg(test)]
mod tests {
    use super::NodePath;

    #[test]
    fn parses_and_normalizes_absolute_paths() {
        let path = NodePath::parse("/Root/Player/../Camera").unwrap();

        assert!(path.is_absolute());
        assert_eq!(path.to_string(), "/Root/Camera");
    }

    #[test]
    fn keeps_relative_parent_walks() {
        let path = NodePath::parse("../UI/HUD").unwrap();

        assert!(!path.is_absolute());
        assert_eq!(path.segments(), &["..", "UI", "HUD"]);
        assert_eq!(path.to_string(), "../UI/HUD");
    }
}
