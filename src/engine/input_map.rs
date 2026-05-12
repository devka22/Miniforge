use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct InputMap {
    pub path: PathBuf,
    pub bindings: BTreeMap<String, Vec<String>>,
}

impl InputMap {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut input = Self {
            path,
            bindings: BTreeMap::from([
                ("select".to_string(), vec!["mouse_left".to_string()]),
                ("command".to_string(), vec!["mouse_right".to_string()]),
                ("pan".to_string(), vec!["mouse_middle".to_string()]),
                (
                    "save".to_string(),
                    vec!["ctrl+s".to_string(), "cmd+s".to_string()],
                ),
                ("play".to_string(), vec!["f5".to_string()]),
            ]),
        };
        input.load()?;
        Ok(input)
    }

    pub fn load(&mut self) -> io::Result<()> {
        if self.path.exists() {
            let value = AssetTools::read_json(&self.path)?;
            if let Some(bindings) = value
                .get("bindings")
                .or(Some(&value))
                .and_then(Value::as_object)
            {
                for (action, keys) in bindings {
                    let keys = keys
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|item| item.as_str().map(ToString::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    self.bindings.insert(action.clone(), keys);
                }
            }
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        AssetTools::write_json(&self.path, &json!({"bindings": self.bindings}))
    }

    pub fn set_binding(&mut self, action: &str, keys: Vec<String>) -> io::Result<()> {
        self.bindings.insert(action.to_string(), keys);
        self.save()
    }
}
