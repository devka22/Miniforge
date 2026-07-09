#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorTool {
    Select,
    Move,
    Rotate,
    Scale,
    Pivot,
    Collision,
    Paint,
}

impl EditorTool {
    pub const ALL: [Self; 7] = [
        Self::Select,
        Self::Move,
        Self::Rotate,
        Self::Scale,
        Self::Pivot,
        Self::Collision,
        Self::Paint,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Pivot => "Pivot",
            Self::Collision => "Collision",
            Self::Paint => "Paint",
        }
    }

    pub fn shortcut(self) -> u8 {
        match self {
            Self::Select => 1,
            Self::Move => 2,
            Self::Rotate => 3,
            Self::Scale => 4,
            Self::Paint => 5,
            Self::Pivot => 6,
            Self::Collision => 7,
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|tool| tool.label().eq_ignore_ascii_case(label.trim()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub tool: EditorTool,
    pub label: String,
    pub tooltip: String,
    pub shortcut: u8,
    pub width_hint: u16,
}

impl ToolDescriptor {
    fn for_tool(tool: EditorTool) -> Self {
        let purpose = match tool {
            EditorTool::Select => "Selecciona objetos y elementos de UI",
            EditorTool::Move => "Mueve la selección con snap y guías",
            EditorTool::Rotate => "Rota la selección",
            EditorTool::Scale => "Escala la selección",
            EditorTool::Pivot => "Edita el pivote de la selección",
            EditorTool::Collision => "Edita colisiones y vértices",
            EditorTool::Paint => "Pinta el tilemap activo",
        };
        let width_hint = match tool {
            EditorTool::Select => 60,
            EditorTool::Move => 52,
            EditorTool::Rotate => 58,
            EditorTool::Scale => 56,
            EditorTool::Pivot => 50,
            EditorTool::Collision => 70,
            EditorTool::Paint => 56,
        };
        Self {
            tool,
            label: tool.label().to_string(),
            tooltip: format!("{purpose} · {}", tool.shortcut()),
            shortcut: tool.shortcut(),
            width_hint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolbar {
    active_tool: EditorTool,
    pub tools: Vec<ToolDescriptor>,
    recent_tools: Vec<EditorTool>,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            active_tool: EditorTool::Select,
            tools: EditorTool::ALL
                .into_iter()
                .map(ToolDescriptor::for_tool)
                .collect(),
            recent_tools: vec![EditorTool::Select],
        }
    }
}

impl Toolbar {
    pub fn active_tool(&self) -> EditorTool {
        self.active_tool
    }

    pub fn descriptors(&self) -> &[ToolDescriptor] {
        &self.tools
    }

    pub fn activate(&mut self, tool: EditorTool) -> bool {
        if !self.tools.iter().any(|descriptor| descriptor.tool == tool) {
            return false;
        }
        if self.active_tool == tool {
            return true;
        }
        self.active_tool = tool;
        self.recent_tools.retain(|recent| *recent != tool);
        self.recent_tools.insert(0, tool);
        self.recent_tools.truncate(4);
        true
    }

    pub fn set_tool(&mut self, tool: &str) -> bool {
        EditorTool::from_label(tool).is_some_and(|tool| self.activate(tool))
    }

    pub fn activate_shortcut(&mut self, shortcut: u8) -> Option<EditorTool> {
        let tool = self
            .tools
            .iter()
            .find(|descriptor| descriptor.shortcut == shortcut)?
            .tool;
        self.activate(tool).then_some(tool)
    }

    pub fn cycle(&mut self, delta: isize) -> EditorTool {
        let current = self
            .tools
            .iter()
            .position(|descriptor| descriptor.tool == self.active_tool)
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(self.tools.len() as isize) as usize;
        let tool = self.tools[next].tool;
        self.activate(tool);
        tool
    }

    pub fn recent_tools(&self) -> &[EditorTool] {
        &self.recent_tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_uses_typed_tools_and_tracks_workflow_history() {
        let mut toolbar = Toolbar::default();
        assert!(toolbar.set_tool("move"));
        assert_eq!(toolbar.active_tool(), EditorTool::Move);
        assert_eq!(toolbar.activate_shortcut(5), Some(EditorTool::Paint));
        assert_eq!(
            toolbar.recent_tools()[..2],
            [EditorTool::Paint, EditorTool::Move]
        );
    }

    #[test]
    fn cycling_wraps_without_invalid_active_tools() {
        let mut toolbar = Toolbar::default();
        assert_eq!(toolbar.cycle(-1), EditorTool::Paint);
        assert_eq!(toolbar.cycle(1), EditorTool::Select);
    }
}
