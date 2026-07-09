use serde::{Deserialize, Serialize};

use crate::engine::forge_ai::context::AiProjectContext;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiOptimizationSuggestion {
    pub system: String,
    pub severity: String,
    pub message: String,
    pub evidence: String,
    pub safe_to_auto_apply: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AiOptimizer;

impl AiOptimizer {
    pub fn analyze(context: &AiProjectContext) -> Vec<AiOptimizationSuggestion> {
        let mut suggestions = Vec::new();
        let visible = context
            .entities
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
            .count();
        if visible > 500 {
            suggestions.push(suggestion(
                "Graphics",
                "Warning",
                "High visible entity count; sprite batching and culling should be verified.",
                format!("{visible} visible entities"),
                false,
            ));
        }
        if context.physics_summary.collider_count > 250 {
            suggestions.push(suggestion(
                "Physics",
                "Suggestion",
                "Collider-heavy scene should use layer masks and static collider chunks.",
                format!("{} colliders", context.physics_summary.collider_count),
                false,
            ));
        }
        let light_count = context.component_count("Light2D") + context.component_count("Light3D");
        if light_count > 0 {
            suggestions.push(suggestion(
                "Lighting",
                "Suggestion",
                "Use per-view light limits and cached shadow casters for dynamic lighting.",
                format!("{light_count} light components"),
                false,
            ));
        }
        suggestions.push(suggestion(
            "RenderBackend",
            "Suggestion",
            "Keep Macroquad/OpenGL compatibility enabled and prefer WGPU Metal on macOS when experimental renderer is active.",
            "RenderBackendConfig exposes OpenGL compatibility and Metal optimization flags.",
            false,
        ));
        suggestions
    }
}

fn suggestion(
    system: &str,
    severity: &str,
    message: impl Into<String>,
    evidence: impl Into<String>,
    safe_to_auto_apply: bool,
) -> AiOptimizationSuggestion {
    AiOptimizationSuggestion {
        system: system.to_string(),
        severity: severity.to_string(),
        message: message.into(),
        evidence: evidence.into(),
        safe_to_auto_apply,
    }
}
