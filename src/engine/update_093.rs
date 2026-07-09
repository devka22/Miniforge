use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Engine093Capability {
    pub system: String,
    pub feature: String,
    pub impact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Engine093UpgradePlan {
    pub version: String,
    pub codename: String,
    pub capabilities: Vec<Engine093Capability>,
    pub next_pass_focus: Vec<String>,
}

impl Engine093UpgradePlan {
    pub fn current() -> Self {
        Self {
            version: crate::engine::version::ENGINE_VERSION.to_string(),
            codename: crate::engine::version::ENGINE_CODENAME.to_string(),
            capabilities: vec![
                cap(
                    "Core",
                    "component bundle helpers",
                    "fewer panics and faster entity setup",
                ),
                cap(
                    "Input",
                    "edge states and axis helpers",
                    "menus/gameplay can react to press/release cleanly",
                ),
                cap(
                    "Systems",
                    "scheduler frame reports",
                    "profiling and budget warnings per system",
                ),
                cap(
                    "Runtime",
                    "configurable headless runner",
                    "tests/tools can boot multiple deterministic steps",
                ),
                cap(
                    "Render",
                    "frame render stats",
                    "visibility and draw-call pressure are measurable",
                ),
                cap(
                    "Pathfinding",
                    "path quality reports",
                    "AI can inspect detours and smoothing wins",
                ),
                cap(
                    "UI",
                    "screen manager stack",
                    "standard game screens are reusable",
                ),
                cap(
                    "Packaging",
                    "standalone launchers",
                    "games can run without source access",
                ),
                cap(
                    "Backend",
                    "system readiness audit",
                    "each subsystem has next-pass actions",
                ),
                cap(
                    "Docs",
                    "0.9.3.4 release map",
                    "the upgrade is visible to users and tooling",
                ),
            ],
            next_pass_focus: vec![
                "connect SystemReadinessReport to an editor dashboard".to_string(),
                "drive ScreenManager2D from runtime input contexts".to_string(),
                "turn scheduler budget warnings into profiler markers".to_string(),
                "add packaged app bundles for macOS/Windows release profiles".to_string(),
                "promote render/pathfinding stats into in-game debug overlays".to_string(),
            ],
        }
    }

    pub fn systems(&self) -> Vec<String> {
        let mut systems = self
            .capabilities
            .iter()
            .map(|capability| capability.system.clone())
            .collect::<Vec<_>>();
        systems.sort();
        systems.dedup();
        systems
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|error| {
            json!({
                "version": self.version,
                "serialization_error": error.to_string(),
            })
        })
    }
}

impl Default for Engine093UpgradePlan {
    fn default() -> Self {
        Self::current()
    }
}

fn cap(system: &str, feature: &str, impact: &str) -> Engine093Capability {
    Engine093Capability {
        system: system.to_string(),
        feature: feature.to_string(),
        impact: impact.to_string(),
    }
}
