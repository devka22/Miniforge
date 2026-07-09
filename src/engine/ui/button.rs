#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonRole {
    #[default]
    Action,
    Toggle,
    Radio,
    Menu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVisualState {
    #[default]
    Idle,
    Hovered,
    Pressed,
    Disabled,
    Hidden,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub rect: (f64, f64, f64, f64),
    pub enabled: bool,
    pub visible: bool,
    pub active: bool,
    pub role: ButtonRole,
    pub command: Option<String>,
    pub tooltip: Option<String>,
    pub shortcut: Option<String>,
}

impl Button {
    pub fn new(label: &str, rect: (f64, f64, f64, f64)) -> Self {
        Self {
            id: stable_id(label),
            label: label.to_string(),
            rect,
            enabled: true,
            visible: true,
            active: false,
            role: ButtonRole::Action,
            command: None,
            tooltip: None,
            shortcut: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_role(mut self, role: ButtonRole) -> Self {
        self.role = role;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Hit testing shared by every frontend. Negative dimensions are normalized and
    /// invalid/hidden controls never capture input.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        Self::hit_test(self.rect, x, y, self.enabled, self.visible)
    }

    pub fn hit_test(
        rect: (f64, f64, f64, f64),
        x: f64,
        y: f64,
        enabled: bool,
        visible: bool,
    ) -> bool {
        let Some((left, top, right, bottom)) = normalized_bounds(rect) else {
            return false;
        };
        enabled
            && visible
            && x.is_finite()
            && y.is_finite()
            && x >= left
            && y >= top
            && x <= right
            && y <= bottom
    }

    pub fn visual_state(&self, hovered: bool, pressed: bool) -> ButtonVisualState {
        Self::resolve_visual_state(self.visible, self.enabled, hovered, pressed)
    }

    pub fn resolve_visual_state(
        visible: bool,
        enabled: bool,
        hovered: bool,
        pressed: bool,
    ) -> ButtonVisualState {
        if !visible {
            ButtonVisualState::Hidden
        } else if !enabled {
            ButtonVisualState::Disabled
        } else if hovered && pressed {
            ButtonVisualState::Pressed
        } else if hovered {
            ButtonVisualState::Hovered
        } else {
            ButtonVisualState::Idle
        }
    }

    pub fn command_at(&self, x: f64, y: f64) -> Option<&str> {
        self.contains(x, y)
            .then_some(self.command.as_deref())
            .flatten()
    }

    pub fn accessibility_label(&self) -> String {
        match self.shortcut.as_deref() {
            Some(shortcut) if !shortcut.is_empty() => format!("{} ({shortcut})", self.label),
            _ => self.label.clone(),
        }
    }
}

fn normalized_bounds(rect: (f64, f64, f64, f64)) -> Option<(f64, f64, f64, f64)> {
    let (x, y, width, height) = rect;
    if ![x, y, width, height].into_iter().all(f64::is_finite) {
        return None;
    }
    let right = x + width;
    let bottom = y + height;
    Some((x.min(right), y.min(bottom), x.max(right), y.max(bottom)))
}

fn stable_id(label: &str) -> String {
    let id = label
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if id.is_empty() {
        "button".to_string()
    } else {
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_testing_normalizes_rectangles_and_respects_state() {
        let button = Button::new("Create Actor", (20.0, 20.0, -10.0, -10.0));
        assert!(button.contains(15.0, 15.0));
        assert!(!button.clone().enabled(false).contains(15.0, 15.0));
        assert!(!button.visible(false).contains(15.0, 15.0));
    }

    #[test]
    fn command_and_accessibility_metadata_are_available_to_frontends() {
        let button = Button::new("Save", (0.0, 0.0, 20.0, 20.0))
            .with_command("save_project")
            .with_shortcut("Cmd+S");
        assert_eq!(button.command_at(4.0, 4.0), Some("save_project"));
        assert_eq!(button.accessibility_label(), "Save (Cmd+S)");
    }
}
