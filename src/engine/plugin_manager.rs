use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
    pub manifest: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginValidation {
    pub valid: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginLoadPlan {
    #[serde(default)]
    pub load_order: Vec<String>,
    #[serde(default)]
    pub disabled_plugins: Vec<String>,
    #[serde(default)]
    pub blocked_plugins: Vec<PluginDependencyIssue>,
    #[serde(default)]
    pub hooks: BTreeMap<String, Vec<String>>,
    pub capabilities: PluginCapabilitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginDependencyIssue {
    pub plugin: String,
    pub dependency: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginCapabilitySummary {
    #[serde(default)]
    pub languages: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub systems: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub components: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub editor_panels: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub asset_importers: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub runtime_features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub services: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub render_backends: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub automation_tools: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct PluginManager {
    pub project_path: PathBuf,
    pub plugins: Vec<PluginInfo>,
}

impl PluginManager {
    pub fn new(project_path: impl AsRef<Path>) -> Self {
        Self {
            project_path: project_path.as_ref().to_path_buf(),
            plugins: Vec::new(),
        }
    }

    pub fn scan(&mut self) -> io::Result<usize> {
        self.plugins.clear();
        for root_name in ["plugins", "packages"] {
            let root = self.project_path.join(root_name);
            if !root.exists() {
                continue;
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let manifest_path = path.join("plugin.json");
                if !manifest_path.exists() {
                    continue;
                }
                let manifest = AssetTools::read_json(&manifest_path).unwrap_or(Value::Null);
                let enabled = manifest
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let name = manifest
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| {
                        path.file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("plugin")
                            .to_string()
                    });
                self.plugins.push(PluginInfo {
                    name,
                    path,
                    enabled,
                    manifest,
                });
            }
        }
        Ok(self.plugins.len())
    }

    pub fn emit_hook(&mut self, hook_name: &str) -> io::Result<usize> {
        self.scan()?;
        let mut count = 0;
        for plugin in &self.plugins {
            if !plugin.enabled {
                continue;
            }
            if manifest_declares_hook(&plugin.manifest, hook_name) {
                count += 1;
            }
        }
        Ok(count)
    }

    pub fn load_plan(&mut self) -> io::Result<PluginLoadPlan> {
        self.scan()?;
        let mut plan = PluginLoadPlan::default();
        let mut enabled = BTreeMap::<String, &PluginInfo>::new();
        for plugin in &self.plugins {
            if plugin.enabled {
                enabled.insert(plugin.name.clone(), plugin);
            } else {
                plan.disabled_plugins.push(plugin.name.clone());
            }
        }

        let mut resolved = BTreeSet::new();
        let mut blocked = BTreeSet::new();
        let mut remaining = enabled.keys().cloned().collect::<BTreeSet<_>>();
        loop {
            let mut progressed = false;
            for name in remaining.clone() {
                let Some(plugin) = enabled.get(&name) else {
                    remaining.remove(&name);
                    continue;
                };
                let deps = manifest_string_array(&plugin.manifest, "dependencies");
                let missing = deps.iter().find(|dep| !enabled.contains_key(*dep)).cloned();
                if let Some(dependency) = missing {
                    plan.blocked_plugins.push(PluginDependencyIssue {
                        plugin: name.clone(),
                        dependency,
                        reason: "dependency_missing_or_disabled".to_string(),
                    });
                    blocked.insert(name.clone());
                    remaining.remove(&name);
                    progressed = true;
                    continue;
                }
                let blocked_dependency = deps.iter().find(|dep| blocked.contains(*dep)).cloned();
                if let Some(dependency) = blocked_dependency {
                    plan.blocked_plugins.push(PluginDependencyIssue {
                        plugin: name.clone(),
                        dependency,
                        reason: "dependency_blocked".to_string(),
                    });
                    blocked.insert(name.clone());
                    remaining.remove(&name);
                    progressed = true;
                    continue;
                }
                if deps.iter().all(|dep| resolved.contains(dep)) {
                    plan.load_order.push(name.clone());
                    resolved.insert(name.clone());
                    remaining.remove(&name);
                    progressed = true;
                }
            }
            if remaining.is_empty() {
                break;
            }
            if !progressed {
                for name in remaining {
                    let Some(plugin) = enabled.get(&name) else {
                        continue;
                    };
                    let dependency = manifest_string_array(&plugin.manifest, "dependencies")
                        .into_iter()
                        .find(|dep| !resolved.contains(dep))
                        .unwrap_or_else(|| "<cycle>".to_string());
                    plan.blocked_plugins.push(PluginDependencyIssue {
                        plugin: name.clone(),
                        dependency,
                        reason: "dependency_cycle".to_string(),
                    });
                    blocked.insert(name);
                }
                break;
            }
        }

        for plugin_name in &plan.load_order {
            let Some(plugin) = enabled.get(plugin_name) else {
                continue;
            };
            for hook in manifest_hooks(&plugin.manifest) {
                plan.hooks
                    .entry(hook)
                    .or_default()
                    .push(plugin.name.clone());
            }
            plan.capabilities.add_plugin(plugin);
        }

        Ok(plan)
    }

    pub fn validate_all(&mut self, engine_version: &str) -> io::Result<Vec<PluginValidation>> {
        self.scan()?;
        Ok(self
            .plugins
            .iter()
            .map(|plugin| Self::validate_plugin(plugin, engine_version))
            .collect())
    }

    pub fn set_enabled(&mut self, plugin_name: &str, enabled: bool) -> io::Result<bool> {
        self.scan()?;
        let Some(plugin) = self
            .plugins
            .iter_mut()
            .find(|plugin| plugin.name == plugin_name)
        else {
            return Ok(false);
        };
        let manifest_path = plugin.path.join("plugin.json");
        let mut manifest = if plugin.manifest.is_object() {
            plugin.manifest.clone()
        } else {
            json!({})
        };
        if let Some(map) = manifest.as_object_mut() {
            map.insert("enabled".to_string(), json!(enabled));
        }
        AssetTools::write_json(&manifest_path, &manifest)?;
        plugin.enabled = enabled;
        plugin.manifest = manifest;
        Ok(true)
    }

    pub fn validate_plugin(plugin: &PluginInfo, engine_version: &str) -> PluginValidation {
        let mut warnings = Vec::new();
        for key in ["name", "version", "author", "enabled", "description"] {
            if plugin.manifest.get(key).is_none() {
                warnings.push(format!("{}: falta `{key}`", plugin.name));
            }
        }
        let min_version = plugin
            .manifest
            .get("min_engine_version")
            .and_then(Value::as_str)
            .unwrap_or("");
        if min_version.is_empty() {
            warnings.push(format!("{}: falta `min_engine_version`", plugin.name));
        } else if min_version > engine_version {
            warnings.push(format!(
                "{}: requiere MiniForge {min_version}, actual {engine_version}",
                plugin.name
            ));
        }
        if plugin
            .manifest
            .get("dependencies")
            .is_some_and(|value| !value.is_array())
        {
            warnings.push(format!("{}: dependencies debe ser array", plugin.name));
        }
        for key in [
            "languages",
            "systems",
            "components",
            "editor_panels",
            "asset_importers",
            "runtime_features",
            "services",
            "render_backends",
            "automation_tools",
        ] {
            if plugin
                .manifest
                .get(key)
                .is_some_and(|value| !value.is_array())
            {
                warnings.push(format!("{}: {key} debe ser array", plugin.name));
            }
        }
        PluginValidation {
            valid: warnings.is_empty(),
            warnings,
        }
    }
}

impl PluginCapabilitySummary {
    fn add_plugin(&mut self, plugin: &PluginInfo) {
        add_language_capability(&mut self.languages, plugin);
        add_capabilities(&mut self.languages, plugin, "languages");
        add_capabilities(&mut self.systems, plugin, "systems");
        add_capabilities(&mut self.components, plugin, "components");
        add_capabilities(&mut self.editor_panels, plugin, "editor_panels");
        add_capabilities(&mut self.asset_importers, plugin, "asset_importers");
        add_capabilities(&mut self.runtime_features, plugin, "runtime_features");
        add_capabilities(&mut self.services, plugin, "services");
        add_capabilities(&mut self.render_backends, plugin, "render_backends");
        add_capabilities(&mut self.automation_tools, plugin, "automation_tools");
    }

    pub fn total_contributions(&self) -> usize {
        [
            &self.systems,
            &self.components,
            &self.editor_panels,
            &self.asset_importers,
            &self.runtime_features,
            &self.services,
            &self.languages,
            &self.render_backends,
            &self.automation_tools,
        ]
        .iter()
        .map(|group| group.values().map(Vec::len).sum::<usize>())
        .sum()
    }
}

fn add_language_capability(target: &mut BTreeMap<String, Vec<String>>, plugin: &PluginInfo) {
    if let Some(language) = plugin.manifest.get("language").and_then(Value::as_str) {
        push_plugin_capability(target, language, plugin);
    }
}

fn add_capabilities(target: &mut BTreeMap<String, Vec<String>>, plugin: &PluginInfo, key: &str) {
    for value in manifest_string_array(&plugin.manifest, key) {
        push_plugin_capability(target, &value, plugin);
    }
}

fn push_plugin_capability(
    target: &mut BTreeMap<String, Vec<String>>,
    capability: &str,
    plugin: &PluginInfo,
) {
    let plugins = target.entry(capability.to_string()).or_default();
    if !plugins.contains(&plugin.name) {
        plugins.push(plugin.name.clone());
    }
}

fn manifest_declares_hook(manifest: &Value, hook_name: &str) -> bool {
    if let Some(hooks) = manifest.get("hooks").and_then(Value::as_array) {
        return hooks.iter().any(|hook| hook.as_str() == Some(hook_name));
    }
    if let Some(hooks) = manifest.get("hooks").and_then(Value::as_object) {
        return hooks.contains_key(hook_name);
    }
    false
}

fn manifest_hooks(manifest: &Value) -> Vec<String> {
    let mut hooks = Vec::new();
    if let Some(values) = manifest.get("hooks").and_then(Value::as_array) {
        hooks.extend(values.iter().filter_map(Value::as_str).map(str::to_string));
    }
    if let Some(values) = manifest.get("hooks").and_then(Value::as_object) {
        hooks.extend(values.keys().cloned());
    }
    hooks.sort();
    hooks.dedup();
    hooks
}

fn manifest_string_array(manifest: &Value, key: &str) -> Vec<String> {
    manifest
        .get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
