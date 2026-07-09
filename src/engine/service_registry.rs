use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::event_bus::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngineServiceKind {
    Editor,
    Runtime,
    Asset,
    Scene,
    Script,
    Graph,
    Ui,
    Render,
    Physics,
    Audio,
    Gameplay,
    Validation,
    Export,
    Plugin,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceHealth {
    Healthy,
    Degraded,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineServiceDescriptor {
    pub name: String,
    pub kind: EngineServiceKind,
    pub startup_order: usize,
    pub enabled: bool,
    pub critical: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub handles_events: Vec<String>,
    pub health: ServiceHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceMessage {
    pub service: String,
    pub event: String,
    pub accepted: bool,
    pub payload_summary: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EngineServiceRegistry {
    #[serde(default)]
    pub services: BTreeMap<String, EngineServiceDescriptor>,
    #[serde(default)]
    pub routes: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub messages: Vec<ServiceMessage>,
}

impl EngineServiceRegistry {
    pub fn default_miniforge_2d() -> Self {
        let mut registry = Self::default();
        for service in [
            service(
                "DiagnosticsService",
                EngineServiceKind::Diagnostics,
                0,
                true,
                &[],
                &[
                    "EngineStarted",
                    "FrameStats",
                    "ScriptPanic",
                    "ValidationFailed",
                ],
            ),
            service(
                "AssetService",
                EngineServiceKind::Asset,
                10,
                true,
                &[],
                &[
                    "AssetImported",
                    "AssetDeleted",
                    "ScriptOpened",
                    "GraphOpened",
                    "SceneOpened",
                ],
            ),
            service(
                "ProjectService",
                EngineServiceKind::Editor,
                15,
                true,
                &["AssetService"],
                &["ProjectOpened", "ProjectSaved", "SettingsChanged"],
            ),
            service(
                "SceneService",
                EngineServiceKind::Scene,
                20,
                true,
                &["AssetService"],
                &["SceneOpened", "SceneSaved", "ActorSpawned", "ActorDeleted"],
            ),
            service(
                "ScriptService",
                EngineServiceKind::Script,
                30,
                true,
                &["AssetService"],
                &["ScriptOpened", "ScriptSaved", "ScriptClosed", "ScriptPanic"],
            ),
            service(
                "GraphService",
                EngineServiceKind::Graph,
                35,
                true,
                &["ScriptService"],
                &["GraphOpened", "GraphSaved", "GraphCompiled"],
            ),
            service(
                "UiService",
                EngineServiceKind::Ui,
                40,
                true,
                &["AssetService", "GraphService"],
                &[
                    "UiDocumentOpened",
                    "UiDocumentSaved",
                    "WidgetChanged",
                    "GraphCompiled",
                ],
            ),
            service(
                "PhysicsService",
                EngineServiceKind::Physics,
                50,
                true,
                &["SceneService"],
                &["ColliderChanged", "TilemapChanged", "PlayStarted"],
            ),
            service(
                "RenderService",
                EngineServiceKind::Render,
                60,
                true,
                &["AssetService", "SceneService"],
                &[
                    "RenderBackendChanged",
                    "TilemapChanged",
                    "ParticlePresetChanged",
                    "UiDocumentSaved",
                    "PlayStarted",
                ],
            ),
            service(
                "AudioService",
                EngineServiceKind::Audio,
                70,
                false,
                &["AssetService"],
                &["AudioImported", "PlayStarted", "PlayStopped"],
            ),
            service(
                "GameplayService",
                EngineServiceKind::Gameplay,
                80,
                true,
                &["SceneService", "ScriptService", "GraphService"],
                &[
                    "AbilityActivated",
                    "GameplayEffectApplied",
                    "PlayStarted",
                    "PlayStopped",
                ],
            ),
            service(
                "PluginService",
                EngineServiceKind::Plugin,
                90,
                false,
                &["ProjectService"],
                &["PluginEnabled", "PluginDisabled", "PluginReloadRequested"],
            ),
            service(
                "ValidationService",
                EngineServiceKind::Validation,
                100,
                true,
                &["AssetService", "SceneService", "ScriptService"],
                &[
                    "ValidateProject",
                    "AssetImported",
                    "ScriptSaved",
                    "GraphSaved",
                ],
            ),
            service(
                "ExportService",
                EngineServiceKind::Export,
                120,
                true,
                &["ValidationService", "AssetService", "ProjectService"],
                &["ExportRequested", "BuildProfileChanged"],
            ),
        ] {
            registry.register(service);
        }
        registry
    }

    pub fn register(&mut self, mut descriptor: EngineServiceDescriptor) {
        if !descriptor.enabled {
            descriptor.health = ServiceHealth::Disabled;
        }
        let name = descriptor.name.clone();
        for event in &descriptor.handles_events {
            self.routes
                .entry(event.clone())
                .or_default()
                .insert(name.clone());
        }
        self.services.insert(name, descriptor);
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        let Some(service) = self.services.get_mut(name) else {
            return false;
        };
        service.enabled = enabled;
        service.health = if enabled {
            ServiceHealth::Healthy
        } else {
            ServiceHealth::Disabled
        };
        true
    }

    pub fn set_health(&mut self, name: &str, health: ServiceHealth) -> bool {
        let Some(service) = self.services.get_mut(name) else {
            return false;
        };
        service.health = health;
        if health == ServiceHealth::Disabled {
            service.enabled = false;
        }
        true
    }

    pub fn startup_order(&self) -> Vec<String> {
        let mut services: Vec<_> = self
            .services
            .values()
            .filter(|service| service.enabled)
            .collect();
        services.sort_by_key(|service| (service.startup_order, service.name.clone()));
        services
            .into_iter()
            .map(|service| service.name.clone())
            .collect()
    }

    pub fn dispatch_event(&mut self, event: &str, payload: &Value) -> Vec<ServiceMessage> {
        let mut recipients = self.routes.get(event).cloned().unwrap_or_default();
        if let Some(wildcards) = self.routes.get("*") {
            recipients.extend(wildcards.iter().cloned());
        }

        let mut delivered = Vec::new();
        for service_name in recipients {
            let Some(service) = self.services.get(&service_name) else {
                continue;
            };
            let accepted = service.enabled
                && matches!(
                    service.health,
                    ServiceHealth::Healthy | ServiceHealth::Degraded
                );
            let message = ServiceMessage {
                service: service_name,
                event: event.to_string(),
                accepted,
                payload_summary: payload_summary(payload),
            };
            self.messages.push(message.clone());
            delivered.push(message);
        }
        delivered
    }

    pub fn dispatch_bus_events(&mut self, bus: &mut EventBus) -> Vec<ServiceMessage> {
        let mut delivered = Vec::new();
        for (event, payload) in bus.drain() {
            delivered.extend(self.dispatch_event(&event, &payload));
        }
        delivered
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for service in self.services.values() {
            if !service.enabled {
                continue;
            }
            for dependency in &service.dependencies {
                match self.services.get(dependency) {
                    Some(dep) if dep.enabled => {}
                    Some(_) => issues.push(format!(
                        "{} depende de {}, pero esta desactivado",
                        service.name, dependency
                    )),
                    None => issues.push(format!(
                        "{} depende de {}, pero no existe",
                        service.name, dependency
                    )),
                }
            }
            if service.critical && service.health == ServiceHealth::Failed {
                issues.push(format!("{} es critico y esta fallando", service.name));
            }
        }
        issues
    }

    pub fn health_summary(&self) -> BTreeMap<ServiceHealth, usize> {
        let mut summary = BTreeMap::new();
        for service in self.services.values() {
            *summary.entry(service.health).or_insert(0) += 1;
        }
        summary
    }

    pub fn manifest(&self) -> Value {
        json!({
            "services": self.services,
            "routes": self.routes,
            "startup_order": self.startup_order(),
            "validation": self.validate(),
            "health": self.health_summary(),
        })
    }
}

fn service(
    name: &str,
    kind: EngineServiceKind,
    startup_order: usize,
    critical: bool,
    dependencies: &[&str],
    events: &[&str],
) -> EngineServiceDescriptor {
    EngineServiceDescriptor {
        name: name.to_string(),
        kind,
        startup_order,
        enabled: true,
        critical,
        dependencies: dependencies.iter().map(|item| item.to_string()).collect(),
        handles_events: events.iter().map(|item| item.to_string()).collect(),
        health: ServiceHealth::Healthy,
    }
}

fn payload_summary(payload: &Value) -> String {
    match payload {
        Value::Null => "null".to_string(),
        Value::Bool(value) => format!("bool:{value}"),
        Value::Number(value) => format!("number:{value}"),
        Value::String(value) => format!("string:{}", value.chars().take(48).collect::<String>()),
        Value::Array(values) => format!("array:{}", values.len()),
        Value::Object(map) => {
            let keys = map.keys().take(4).cloned().collect::<Vec<_>>().join(",");
            format!("object:{}:{keys}", map.len())
        }
    }
}
