use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputActionInfo {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub devices: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct InputMap {
    pub path: PathBuf,
    pub bindings: BTreeMap<String, Vec<String>>,
    pub actions: BTreeMap<String, InputActionInfo>,
}

impl InputMap {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut input = Self {
            path,
            bindings: default_bindings(),
            actions: default_actions(),
        };
        input.load()?;
        input.ensure_default_actions();
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
            if let Some(actions) = value.get("actions").and_then(Value::as_object) {
                for (name, action) in actions {
                    let info = InputActionInfo {
                        name: name.clone(),
                        display_name: action
                            .get("display_name")
                            .and_then(Value::as_str)
                            .unwrap_or(name)
                            .to_string(),
                        category: action
                            .get("category")
                            .and_then(Value::as_str)
                            .unwrap_or("Gameplay")
                            .to_string(),
                        devices: action
                            .get("devices")
                            .and_then(Value::as_array)
                            .map(|items| string_array(items))
                            .unwrap_or_else(|| vec!["keyboard".to_string()]),
                        description: action
                            .get("description")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    };
                    self.actions.insert(name.clone(), info);
                }
            }
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> io::Result<()> {
        let actions = self
            .actions
            .iter()
            .map(|(name, action)| {
                (
                    name.clone(),
                    json!({
                        "display_name": action.display_name,
                        "category": action.category,
                        "devices": action.devices,
                        "description": action.description,
                    }),
                )
            })
            .collect::<BTreeMap<_, _>>();
        AssetTools::write_json(
            &self.path,
            &json!({"bindings": self.bindings, "actions": actions}),
        )
    }

    pub fn set_binding(&mut self, action: &str, keys: Vec<String>) -> io::Result<()> {
        self.bindings.insert(action.to_string(), keys);
        self.actions
            .entry(action.to_string())
            .or_insert_with(|| input_action(action, action, "Gameplay", &["keyboard"], ""));
        self.save()
    }

    pub fn set_action_binding(
        &mut self,
        action: &str,
        binding_index: usize,
        binding: impl Into<String>,
    ) -> io::Result<()> {
        let bindings = self.bindings.entry(action.to_string()).or_default();
        if binding_index >= bindings.len() {
            bindings.push(binding.into());
        } else {
            bindings[binding_index] = binding.into();
        }
        self.save()
    }

    pub fn add_binding(&mut self, action: &str, binding: impl Into<String>) -> io::Result<()> {
        let binding = binding.into();
        let bindings = self.bindings.entry(action.to_string()).or_default();
        if !bindings.contains(&binding) {
            bindings.push(binding);
        }
        self.save()
    }

    pub fn remove_binding(&mut self, action: &str, binding: &str) -> io::Result<()> {
        if let Some(bindings) = self.bindings.get_mut(action) {
            bindings.retain(|existing| existing != binding);
        }
        self.save()
    }

    pub fn action_infos(&self) -> Vec<InputActionInfo> {
        self.actions.values().cloned().collect()
    }

    pub fn ensure_default_actions(&mut self) {
        for (name, info) in default_actions() {
            self.actions.entry(name.clone()).or_insert(info);
        }
        for (name, bindings) in default_bindings() {
            self.bindings.entry(name).or_insert(bindings);
        }
    }
}

fn default_bindings() -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        (
            "Move".to_string(),
            vec![
                "keyboard:wasd".to_string(),
                "keyboard:arrows".to_string(),
                "gamepad:left_stick".to_string(),
            ],
        ),
        (
            "Attack".to_string(),
            vec![
                "mouse_left".to_string(),
                "gamepad:right_trigger".to_string(),
            ],
        ),
        (
            "Jump".to_string(),
            vec!["space".to_string(), "gamepad:south".to_string()],
        ),
        (
            "Interact".to_string(),
            vec!["e".to_string(), "gamepad:west".to_string()],
        ),
        (
            "Pause".to_string(),
            vec!["escape".to_string(), "gamepad:start".to_string()],
        ),
        (
            "Select".to_string(),
            vec!["mouse_left".to_string(), "gamepad:south".to_string()],
        ),
        (
            "Command".to_string(),
            vec![
                "mouse_right".to_string(),
                "gamepad:right_shoulder".to_string(),
            ],
        ),
        (
            "CameraPan".to_string(),
            vec![
                "mouse_middle".to_string(),
                "gamepad:right_stick".to_string(),
            ],
        ),
        ("select".to_string(), vec!["mouse_left".to_string()]),
        ("command".to_string(), vec!["mouse_right".to_string()]),
        ("pan".to_string(), vec!["mouse_middle".to_string()]),
        (
            "save".to_string(),
            vec!["ctrl+s".to_string(), "cmd+s".to_string()],
        ),
        ("play".to_string(), vec!["f5".to_string()]),
    ])
}

fn default_actions() -> BTreeMap<String, InputActionInfo> {
    BTreeMap::from([
        (
            "Move".to_string(),
            input_action(
                "Move",
                "Move",
                "Gameplay",
                &["keyboard", "gamepad"],
                "Directional movement vector.",
            ),
        ),
        (
            "Attack".to_string(),
            input_action(
                "Attack",
                "Attack",
                "Gameplay",
                &["mouse", "gamepad"],
                "Primary attack or RTS attack order.",
            ),
        ),
        (
            "Jump".to_string(),
            input_action(
                "Jump",
                "Jump",
                "Gameplay",
                &["keyboard", "gamepad"],
                "Platformer jump action.",
            ),
        ),
        (
            "Interact".to_string(),
            input_action(
                "Interact",
                "Interact",
                "Gameplay",
                &["keyboard", "gamepad"],
                "Talk, pickup, activate or inspect.",
            ),
        ),
        (
            "Pause".to_string(),
            input_action(
                "Pause",
                "Pause",
                "System",
                &["keyboard", "gamepad"],
                "Pause gameplay or open pause menu.",
            ),
        ),
        (
            "Select".to_string(),
            input_action(
                "Select",
                "Select",
                "RTS",
                &["mouse", "gamepad"],
                "Select units or confirm UI.",
            ),
        ),
        (
            "Command".to_string(),
            input_action(
                "Command",
                "Command",
                "RTS",
                &["mouse", "gamepad"],
                "Move, attack-move, gather and contextual commands.",
            ),
        ),
        (
            "CameraPan".to_string(),
            input_action(
                "CameraPan",
                "Camera Pan",
                "Camera",
                &["mouse", "gamepad"],
                "Pan the editor/runtime camera.",
            ),
        ),
    ])
}

fn input_action(
    name: &str,
    display_name: &str,
    category: &str,
    devices: &[&str],
    description: &str,
) -> InputActionInfo {
    InputActionInfo {
        name: name.to_string(),
        display_name: display_name.to_string(),
        category: category.to_string(),
        devices: devices.iter().map(|device| (*device).to_string()).collect(),
        description: description.to_string(),
    }
}

fn string_array(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect()
}
