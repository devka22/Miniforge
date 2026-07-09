//! Advanced, engine-agnostic UI preparation for MiniForge interfaces.
//!
//! This layer complements the authoring structures in `miniforge_2d::ui_framework`
//! with production concerns: responsive breakpoints, view-model bindings,
//! localization fallbacks and accessibility diagnostics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::engine::miniforge_2d::ui_framework::{
    UiCanvas2D, UiResolvedWidget2D, UiStyle2D, UiWidget2D, is_interactive_widget_type,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiBreakpoint2D {
    pub name: String,
    pub min_width: f32,
    pub max_width: Option<f32>,
    pub scale: f32,
    pub safe_area: [f32; 4],
    pub minimum_touch_target: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiBindingContext2D {
    #[serde(default)]
    pub state: Value,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub translations: BTreeMap<String, BTreeMap<String, String>>,
}

impl Default for UiBindingContext2D {
    fn default() -> Self {
        Self {
            state: Value::Null,
            locale: default_locale(),
            translations: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiBindingReport2D {
    pub applied: usize,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UiAuditSeverity2D {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiAuditIssue2D {
    pub severity: UiAuditSeverity2D,
    pub code: String,
    pub widget_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UiAccessibilityReport2D {
    pub score: u8,
    pub interactive_widgets: usize,
    pub labelled_widgets: usize,
    pub focus_order: Vec<String>,
    pub issues: Vec<UiAuditIssue2D>,
}

impl UiAccessibilityReport2D {
    pub fn errors(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == UiAuditSeverity2D::Error)
            .count()
    }

    pub fn warnings(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == UiAuditSeverity2D::Warning)
            .count()
    }

    pub fn production_ready(&self) -> bool {
        self.errors() == 0 && self.score >= 80
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiResolvedInterface2D {
    pub breakpoint: String,
    pub viewport: [f32; 2],
    pub content_rect: [f32; 4],
    pub widgets: Vec<UiResolvedWidget2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiPreparedInterface2D {
    pub canvas: UiCanvas2D,
    pub layout: UiResolvedInterface2D,
    pub bindings: UiBindingReport2D,
    pub accessibility: UiAccessibilityReport2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiAdvancedInterface2D {
    pub breakpoints: Vec<UiBreakpoint2D>,
    pub bindings: UiBindingContext2D,
}

impl Default for UiAdvancedInterface2D {
    fn default() -> Self {
        Self {
            breakpoints: standard_breakpoints(),
            bindings: UiBindingContext2D::default(),
        }
    }
}

impl UiAdvancedInterface2D {
    pub fn select_breakpoint(&self, width: f32) -> UiBreakpoint2D {
        self.breakpoints
            .iter()
            .filter(|profile| {
                width >= profile.min_width
                    && profile.max_width.is_none_or(|maximum| width <= maximum)
            })
            .max_by(|left, right| left.min_width.total_cmp(&right.min_width))
            .or_else(|| {
                self.breakpoints.iter().min_by(|left, right| {
                    (width - left.min_width)
                        .abs()
                        .total_cmp(&(width - right.min_width).abs())
                })
            })
            .cloned()
            .unwrap_or_else(default_desktop_breakpoint)
    }

    pub fn prepare(&self, canvas: &UiCanvas2D, viewport: (f32, f32)) -> UiPreparedInterface2D {
        let (canvas, bindings) = apply_ui_bindings(canvas, &self.bindings);
        let breakpoint = self.select_breakpoint(viewport.0);
        let layout = resolve_responsive_layout(&canvas, viewport, &breakpoint);
        let accessibility = audit_accessibility(&canvas, &layout, breakpoint.minimum_touch_target);
        UiPreparedInterface2D {
            canvas,
            layout,
            bindings,
            accessibility,
        }
    }
}

pub fn standard_breakpoints() -> Vec<UiBreakpoint2D> {
    vec![
        UiBreakpoint2D {
            name: "mobile".to_string(),
            min_width: 0.0,
            max_width: Some(599.0),
            scale: 0.82,
            safe_area: [16.0, 20.0, 16.0, 20.0],
            minimum_touch_target: 48.0,
        },
        UiBreakpoint2D {
            name: "tablet".to_string(),
            min_width: 600.0,
            max_width: Some(1023.0),
            scale: 0.92,
            safe_area: [24.0, 20.0, 24.0, 20.0],
            minimum_touch_target: 46.0,
        },
        UiBreakpoint2D {
            name: "desktop".to_string(),
            min_width: 1024.0,
            max_width: Some(1919.0),
            scale: 1.0,
            safe_area: [24.0, 20.0, 24.0, 20.0],
            minimum_touch_target: 44.0,
        },
        UiBreakpoint2D {
            name: "ultrawide".to_string(),
            min_width: 1920.0,
            max_width: None,
            scale: 1.12,
            safe_area: [48.0, 32.0, 48.0, 32.0],
            minimum_touch_target: 44.0,
        },
    ]
}

pub fn apply_ui_bindings(
    canvas: &UiCanvas2D,
    context: &UiBindingContext2D,
) -> (UiCanvas2D, UiBindingReport2D) {
    let mut canvas = canvas.clone();
    let mut report = UiBindingReport2D::default();
    for widget in &mut canvas.widgets {
        apply_widget_bindings(widget, context, &mut report);
    }
    (canvas, report)
}

pub fn resolve_responsive_layout(
    canvas: &UiCanvas2D,
    viewport: (f32, f32),
    breakpoint: &UiBreakpoint2D,
) -> UiResolvedInterface2D {
    let [left, top, right, bottom] = breakpoint.safe_area;
    let content_width = (viewport.0 - left - right).max(1.0);
    let content_height = (viewport.1 - top - bottom).max(1.0);
    let mut scaled = canvas.clone();
    for widget in &mut scaled.widgets {
        scale_widget(widget, breakpoint.scale);
    }
    let mut widgets = scaled.resolve_layout((content_width, content_height));
    for widget in &mut widgets {
        widget.rect.x += left;
        widget.rect.y += top;
    }
    UiResolvedInterface2D {
        breakpoint: breakpoint.name.clone(),
        viewport: [viewport.0, viewport.1],
        content_rect: [left, top, content_width, content_height],
        widgets,
    }
}

pub fn audit_accessibility(
    canvas: &UiCanvas2D,
    layout: &UiResolvedInterface2D,
    minimum_touch_target: f32,
) -> UiAccessibilityReport2D {
    let mut report = UiAccessibilityReport2D::default();
    let resolved = layout
        .widgets
        .iter()
        .map(|widget| (widget.id.as_str(), widget))
        .collect::<BTreeMap<_, _>>();

    if !canvas.validate_widget_ids() {
        report.issues.push(issue(
            UiAuditSeverity2D::Error,
            "duplicate_widget_id",
            canvas.name.clone(),
            "Widget ids must be unique inside a canvas.",
        ));
    }
    for navigation_issue in canvas.validate_navigation_links() {
        report.issues.push(issue(
            UiAuditSeverity2D::Error,
            "invalid_focus_target",
            canvas.name.clone(),
            navigation_issue,
        ));
    }

    for widget in canvas.flatten_widgets() {
        if widget.properties.get("visible").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let interactive =
            is_interactive_widget_type(&widget.widget_type) || !widget.callbacks.is_empty();
        let label = accessible_label(widget);
        if label.is_some() {
            report.labelled_widgets += 1;
        }
        if interactive {
            report.interactive_widgets += 1;
            report.focus_order.push(widget.id.clone());
            if label.is_none() {
                report.issues.push(issue(
                    UiAuditSeverity2D::Error,
                    "missing_accessible_name",
                    widget.id.clone(),
                    "Interactive widget needs accessible_name, text, label, title or alt_text.",
                ));
            }
            if let Some(resolved) = resolved.get(widget.id.as_str())
                && (resolved.rect.width < minimum_touch_target
                    || resolved.rect.height < minimum_touch_target)
            {
                report.issues.push(issue(
                    UiAuditSeverity2D::Warning,
                    "touch_target_too_small",
                    widget.id.clone(),
                    format!(
                        "Interactive target is {:.0}x{:.0}; recommended minimum is {:.0}x{:.0}.",
                        resolved.rect.width,
                        resolved.rect.height,
                        minimum_touch_target,
                        minimum_touch_target
                    ),
                ));
            }
        }

        let style = resolved_style(canvas, widget);
        if let (Some(foreground), Some(background)) = (style.foreground, style.background) {
            let contrast = contrast_ratio(foreground, background);
            if contrast < 4.5 {
                report.issues.push(issue(
                    UiAuditSeverity2D::Warning,
                    "low_color_contrast",
                    widget.id.clone(),
                    format!("Text contrast is {contrast:.2}:1; target is at least 4.5:1."),
                ));
            }
        }
    }

    for animation in &canvas.animations {
        if canvas.find_widget(&animation.target_widget).is_none() {
            report.issues.push(issue(
                UiAuditSeverity2D::Error,
                "animation_target_missing",
                animation.target_widget.clone(),
                format!("Animation {} points to a missing widget.", animation.name),
            ));
        } else if animation.duration > 5.0 {
            report.issues.push(issue(
                UiAuditSeverity2D::Info,
                "long_ui_animation",
                animation.target_widget.clone(),
                "Long UI animation should expose a reduced-motion alternative.",
            ));
        }
    }

    let penalty = report.errors() * 20 + report.warnings() * 5;
    report.score = 100usize.saturating_sub(penalty).min(100) as u8;
    report
}

fn apply_widget_bindings(
    widget: &mut UiWidget2D,
    context: &UiBindingContext2D,
    report: &mut UiBindingReport2D,
) {
    let bindings = widget.bindings.clone();
    for binding in bindings {
        let value = resolve_binding_value(&binding.source_path, context)
            .unwrap_or_else(|| binding.fallback.clone());
        if value.is_null() && binding.fallback.is_null() {
            report.unresolved.push(format!(
                "{}.{} <- {}",
                widget.id, binding.property, binding.source_path
            ));
            continue;
        }
        set_json_path(&mut widget.properties, &binding.property, value);
        report.applied += 1;
    }
    for child in &mut widget.children {
        apply_widget_bindings(child, context, report);
    }
}

fn resolve_binding_value(expression: &str, context: &UiBindingContext2D) -> Option<Value> {
    let mut parts = expression.split('|');
    let source = parts.next()?.trim();
    let mut value = if let Some(key) = source.strip_prefix("@i18n:") {
        context
            .translations
            .get(&context.locale)
            .and_then(|locale| locale.get(key))
            .or_else(|| {
                context
                    .translations
                    .get("en")
                    .and_then(|locale| locale.get(key))
            })
            .map(|value| json!(value))?
    } else if let Some(literal) = source.strip_prefix("@literal:") {
        json!(literal)
    } else {
        resolve_json_path(&context.state, source)?.clone()
    };
    for filter in parts.map(str::trim) {
        value = apply_binding_filter(value, filter);
    }
    Some(value)
}

fn apply_binding_filter(value: Value, filter: &str) -> Value {
    match filter {
        "percent" => value
            .as_f64()
            .map(|number| json!(format!("{:.0}%", number * 100.0)))
            .unwrap_or(value),
        "round" => value
            .as_f64()
            .map(|number| json!(number.round() as i64))
            .unwrap_or(value),
        "uppercase" => value
            .as_str()
            .map(|text| json!(text.to_uppercase()))
            .unwrap_or(value),
        "lowercase" => value
            .as_str()
            .map(|text| json!(text.to_lowercase()))
            .unwrap_or(value),
        "visible" => json!(value.as_bool().unwrap_or(!value.is_null())),
        _ => value,
    }
}

fn resolve_json_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut value = root;
    for segment in path.trim_start_matches("state.").split('.') {
        if segment.is_empty() {
            continue;
        }
        value = match value {
            Value::Object(map) => map.get(segment)?,
            Value::Array(values) => values.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(value)
}

fn set_json_path(root: &mut Value, path: &str, value: Value) {
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let mut current = root;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            current
                .as_object_mut()
                .expect("binding target must be an object")
                .insert(segment.to_string(), value);
            return;
        }
        let map = current
            .as_object_mut()
            .expect("binding target must be an object");
        current = map
            .entry(segment.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
    }
}

fn scale_widget(widget: &mut UiWidget2D, scale: f32) {
    widget.rect.x *= scale;
    widget.rect.y *= scale;
    widget.rect.width *= scale;
    widget.rect.height *= scale;
    for child in &mut widget.children {
        scale_widget(child, scale);
    }
}

fn accessible_label(widget: &UiWidget2D) -> Option<&str> {
    ["accessible_name", "text", "label", "title", "alt_text"]
        .into_iter()
        .find_map(|key| widget.properties.get(key).and_then(Value::as_str))
        .filter(|label| !label.trim().is_empty())
}

fn resolved_style(canvas: &UiCanvas2D, widget: &UiWidget2D) -> UiStyle2D {
    let mut style = widget
        .style
        .style_id
        .as_ref()
        .and_then(|id| canvas.theme.styles.get(id))
        .cloned()
        .unwrap_or_default();
    style.background = widget.style.background.or(style.background);
    style.foreground = widget.style.foreground.or(style.foreground);
    style.font_size = widget.style.font_size.or(style.font_size);
    style.padding = widget.style.padding.or(style.padding);
    style.radius = widget.style.radius.or(style.radius);
    style
}

fn contrast_ratio(foreground: [u8; 4], background: [u8; 4]) -> f32 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn relative_luminance(color: [u8; 4]) -> f32 {
    let channel = |value: u8| {
        let value = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
}

fn issue(
    severity: UiAuditSeverity2D,
    code: &str,
    widget_id: String,
    message: impl Into<String>,
) -> UiAuditIssue2D {
    UiAuditIssue2D {
        severity,
        code: code.to_string(),
        widget_id,
        message: message.into(),
    }
}

fn default_locale() -> String {
    "en".to_string()
}

fn default_desktop_breakpoint() -> UiBreakpoint2D {
    UiBreakpoint2D {
        name: "desktop".to_string(),
        min_width: 0.0,
        max_width: None,
        scale: 1.0,
        safe_area: [24.0, 20.0, 24.0, 20.0],
        minimum_touch_target: 44.0,
    }
}
