use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Anchor2D {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct UiRect2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCallback2D {
    pub event: String,
    pub graph: Option<String>,
    pub function: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UiStyle2D {
    pub style_id: Option<String>,
    pub background: Option<[u8; 4]>,
    pub foreground: Option<[u8; 4]>,
    pub font_size: Option<f32>,
    pub padding: Option<[f32; 4]>,
    pub radius: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiBinding2D {
    pub property: String,
    pub source_path: String,
    pub fallback: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiNavigation2D {
    pub up: Option<String>,
    pub down: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiWidget2D {
    pub id: String,
    pub widget_type: String,
    pub rect: UiRect2D,
    pub anchors: Anchor2D,
    #[serde(default)]
    pub children: Vec<UiWidget2D>,
    #[serde(default)]
    pub callbacks: Vec<UiCallback2D>,
    #[serde(default)]
    pub properties: Value,
    #[serde(default)]
    pub style: UiStyle2D,
    #[serde(default)]
    pub bindings: Vec<UiBinding2D>,
    #[serde(default)]
    pub navigation: UiNavigation2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiCanvas2D {
    pub name: String,
    pub viewport_width: f32,
    pub viewport_height: f32,
    #[serde(default)]
    pub widgets: Vec<UiWidget2D>,
    #[serde(default)]
    pub theme: UiTheme2D,
    #[serde(default)]
    pub animations: Vec<UiAnimation2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiTheme2D {
    pub name: String,
    #[serde(default)]
    pub styles: BTreeMap<String, UiStyle2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiAnimation2D {
    pub name: String,
    pub target_widget: String,
    pub duration: f32,
    #[serde(default)]
    pub keyframes: Vec<UiAnimationKey2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiAnimationKey2D {
    pub time: f32,
    pub property: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiResolvedWidget2D {
    pub id: String,
    pub widget_type: String,
    pub rect: UiRect2D,
    #[serde(default)]
    pub clip_rect: Option<UiRect2D>,
    pub interactive: bool,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum UiScreenKind2D {
    MainMenu,
    Pause,
    Settings,
    Hud,
    Inventory,
    Dialogue,
    GameOver,
    LevelSelect,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UIScreen2D {
    pub id: String,
    pub kind: UiScreenKind2D,
    pub canvas: UiCanvas2D,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub modal: bool,
    #[serde(default)]
    pub blocks_gameplay: bool,
    #[serde(default)]
    pub close_on_cancel: bool,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub input_context: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ScreenManager2D {
    #[serde(default)]
    pub screens: BTreeMap<String, UIScreen2D>,
    #[serde(default)]
    pub active_stack: Vec<String>,
    #[serde(default)]
    pub last_closed: Vec<String>,
}

impl Anchor2D {
    pub const TOP_LEFT: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
    };
    pub const TOP_RIGHT: Self = Self {
        min_x: 1.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 0.0,
    };
    pub const CENTER: Self = Self {
        min_x: 0.5,
        min_y: 0.5,
        max_x: 0.5,
        max_y: 0.5,
    };
    pub const BOTTOM_LEFT: Self = Self {
        min_x: 0.0,
        min_y: 1.0,
        max_x: 0.0,
        max_y: 1.0,
    };
    pub const BOTTOM_RIGHT: Self = Self {
        min_x: 1.0,
        min_y: 1.0,
        max_x: 1.0,
        max_y: 1.0,
    };
    pub const BOTTOM_CENTER: Self = Self {
        min_x: 0.5,
        min_y: 1.0,
        max_x: 0.5,
        max_y: 1.0,
    };
    pub const FILL: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 1.0,
        max_y: 1.0,
    };
}

impl UIScreen2D {
    pub fn new(id: impl Into<String>, kind: UiScreenKind2D, canvas: UiCanvas2D) -> Self {
        Self {
            id: id.into(),
            kind,
            canvas,
            visible: false,
            modal: false,
            blocks_gameplay: false,
            close_on_cancel: true,
            persistent: false,
            sort_order: 0,
            input_context: None,
        }
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn modal(mut self, blocks_gameplay: bool) -> Self {
        self.modal = true;
        self.blocks_gameplay = blocks_gameplay;
        self
    }

    pub fn persistent(mut self) -> Self {
        self.persistent = true;
        self.close_on_cancel = false;
        self
    }

    pub fn sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = sort_order;
        self
    }

    pub fn input_context(mut self, input_context: impl Into<String>) -> Self {
        self.input_context = Some(input_context.into());
        self
    }
}

impl ScreenManager2D {
    pub fn standard(game_title: &str) -> Self {
        let mut manager = Self::default();
        for screen in [
            hud_screen(),
            main_menu_screen(game_title),
            pause_screen(),
            settings_screen(),
            inventory_screen(),
            dialogue_screen(),
            game_over_screen(),
            level_select_screen(),
        ] {
            manager.register_screen(screen);
        }
        manager.open_screen("HUDScreen");
        manager.open_screen("MainMenuScreen");
        manager
    }

    pub fn register_screen(&mut self, screen: UIScreen2D) -> Option<UIScreen2D> {
        let id = screen.id.clone();
        if screen.visible && !self.active_stack.iter().any(|active| active == &id) {
            self.active_stack.push(id.clone());
        }
        self.screens.insert(id, screen)
    }

    pub fn open_screen(&mut self, id: &str) -> bool {
        let Some(screen) = self.screens.get_mut(id) else {
            return false;
        };
        screen.visible = true;
        if !self.active_stack.iter().any(|active| active == id) {
            self.active_stack.push(id.to_string());
        }
        true
    }

    pub fn close_screen(&mut self, id: &str) -> bool {
        let Some(screen) = self.screens.get_mut(id) else {
            return false;
        };
        if screen.persistent {
            return false;
        }
        screen.visible = false;
        self.active_stack.retain(|active| active != id);
        self.last_closed.push(id.to_string());
        true
    }

    pub fn toggle_screen(&mut self, id: &str) -> bool {
        if self.screens.get(id).map(|screen| screen.visible) == Some(true) {
            self.close_screen(id)
        } else {
            self.open_screen(id)
        }
    }

    pub fn active_screens(&self) -> Vec<&UIScreen2D> {
        let mut seen = BTreeSet::new();
        let mut active = Vec::new();
        for id in &self.active_stack {
            if let Some(screen) = self.screens.get(id)
                && screen.visible
            {
                seen.insert(id.clone());
                active.push(screen);
            }
        }
        let mut passive = self
            .screens
            .values()
            .filter(|screen| screen.visible && !seen.contains(&screen.id))
            .collect::<Vec<_>>();
        passive.sort_by_key(|screen| screen.sort_order);
        active.extend(passive);
        active.sort_by_key(|screen| screen.sort_order);
        active
    }

    pub fn active_canvases(&self) -> Vec<&UiCanvas2D> {
        self.active_screens()
            .into_iter()
            .map(|screen| &screen.canvas)
            .collect()
    }

    pub fn top_screen(&self) -> Option<&UIScreen2D> {
        self.active_stack
            .iter()
            .rev()
            .filter_map(|id| self.screens.get(id))
            .find(|screen| screen.visible)
            .or_else(|| {
                self.screens
                    .values()
                    .filter(|screen| screen.visible)
                    .max_by_key(|screen| screen.sort_order)
            })
    }

    pub fn gameplay_blocked(&self) -> bool {
        self.active_screens()
            .into_iter()
            .any(|screen| screen.blocks_gameplay)
    }

    pub fn command_for_widget(&self, screen_id: &str, widget_id: &str) -> Option<String> {
        self.screens
            .get(screen_id)
            .and_then(|screen| screen.canvas.command_for_widget(widget_id))
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for (id, screen) in &self.screens {
            if id != &screen.id {
                issues.push(format!("screen key {id} does not match id {}", screen.id));
            }
            if !screen.canvas.validate_widget_ids() {
                issues.push(format!("screen {} has duplicate widget ids", screen.id));
            }
            for issue in screen.canvas.validate_navigation_links() {
                issues.push(format!("screen {} navigation: {issue}", screen.id));
            }
        }
        for id in &self.active_stack {
            if !self.screens.contains_key(id) {
                issues.push(format!("active screen missing: {id}"));
            }
        }
        issues
    }
}

impl UiCanvas2D {
    pub fn validate_widget_ids(&self) -> bool {
        let mut ids = BTreeSet::new();
        self.widgets
            .iter()
            .all(|widget| collect_widget_id(widget, &mut ids))
    }

    pub fn find_widget(&self, id: &str) -> Option<&UiWidget2D> {
        self.widgets
            .iter()
            .find_map(|widget| find_widget(widget, id))
    }

    pub fn find_widget_mut(&mut self, id: &str) -> Option<&mut UiWidget2D> {
        self.widgets
            .iter_mut()
            .find_map(|widget| find_widget_mut(widget, id))
    }

    pub fn flatten_widgets(&self) -> Vec<&UiWidget2D> {
        let mut widgets = Vec::new();
        for widget in &self.widgets {
            flatten_widget(widget, &mut widgets);
        }
        widgets
    }

    pub fn resolve_layout(&self, viewport: (f32, f32)) -> Vec<UiResolvedWidget2D> {
        let root_rect = UiRect2D {
            x: 0.0,
            y: 0.0,
            width: viewport.0,
            height: viewport.1,
        };
        let mut resolved = Vec::new();
        for widget in &self.widgets {
            resolve_widget_layout(widget, root_rect, None, 0, &mut resolved);
        }
        resolved
    }

    pub fn binding_paths(&self) -> Vec<String> {
        let mut paths = self
            .flatten_widgets()
            .into_iter()
            .flat_map(|widget| {
                widget
                    .bindings
                    .iter()
                    .map(|binding| binding.source_path.clone())
            })
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        paths
    }

    pub fn focused_neighbor(&self, widget_id: &str, direction: &str) -> Option<&str> {
        let widget = self.find_widget(widget_id)?;
        match direction {
            "up" => widget.navigation.up.as_deref(),
            "down" => widget.navigation.down.as_deref(),
            "left" => widget.navigation.left.as_deref(),
            "right" => widget.navigation.right.as_deref(),
            _ => None,
        }
    }

    pub fn widget_type_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for widget in self.flatten_widgets() {
            *counts.entry(widget.widget_type.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn callbacks_for_event(&self, event: &str) -> Vec<&UiCallback2D> {
        self.flatten_widgets()
            .into_iter()
            .flat_map(|widget| &widget.callbacks)
            .filter(|callback| callback.event.eq_ignore_ascii_case(event))
            .collect()
    }

    pub fn validate_navigation_links(&self) -> Vec<String> {
        let ids = self
            .flatten_widgets()
            .into_iter()
            .map(|widget| widget.id.clone())
            .collect::<BTreeSet<_>>();
        let mut issues = Vec::new();
        for widget in self.flatten_widgets() {
            for (direction, target) in [
                ("up", widget.navigation.up.as_deref()),
                ("down", widget.navigation.down.as_deref()),
                ("left", widget.navigation.left.as_deref()),
                ("right", widget.navigation.right.as_deref()),
            ] {
                if let Some(target) = target
                    && !ids.contains(target)
                {
                    issues.push(format!("{}.{} -> {}", widget.id, direction, target));
                }
            }
        }
        issues
    }

    pub fn command_for_widget(&self, id: &str) -> Option<String> {
        self.find_widget(id).and_then(widget_command)
    }
}

impl Default for UiTheme2D {
    fn default() -> Self {
        Self {
            name: "MiniForge Dark".to_string(),
            styles: BTreeMap::from([
                (
                    "panel".to_string(),
                    UiStyle2D {
                        style_id: Some("panel".to_string()),
                        background: Some([20, 24, 32, 230]),
                        foreground: Some([230, 238, 248, 255]),
                        font_size: Some(14.0),
                        padding: Some([8.0, 8.0, 8.0, 8.0]),
                        radius: Some(6.0),
                    },
                ),
                (
                    "button".to_string(),
                    UiStyle2D {
                        style_id: Some("button".to_string()),
                        background: Some([36, 48, 66, 255]),
                        foreground: Some([242, 248, 255, 255]),
                        font_size: Some(14.0),
                        padding: Some([10.0, 8.0, 10.0, 8.0]),
                        radius: Some(4.0),
                    },
                ),
            ]),
        }
    }
}

pub fn supported_widget_types() -> Vec<&'static str> {
    vec![
        "Canvas",
        "SafeArea",
        "Panel",
        "Label",
        "Text",
        "RichText",
        "Button",
        "MenuButton",
        "IconButton",
        "Image",
        "NineSlice",
        "ProgressBar",
        "Slider",
        "Checkbox",
        "TextInput",
        "InputField",
        "Dropdown",
        "TabView",
        "VerticalBox",
        "HorizontalBox",
        "GridPanel",
        "ScrollBox",
        "Spacer",
        "Border",
        "MenuStack",
        "MiniMap",
        "DialogueBox",
        "InventorySlot",
        "InventoryGrid",
        "AbilityBar",
        "Tooltip",
        "RadialMenu",
    ]
}

pub fn supported_screen_types() -> Vec<&'static str> {
    vec![
        "UIScreen",
        "ScreenManager",
        "MainMenuScreen",
        "PauseScreen",
        "SettingsScreen",
        "HUDScreen",
        "InventoryScreen",
        "DialogueScreen",
        "GameOverScreen",
        "LevelSelectScreen",
    ]
}

pub fn supported_ui_events() -> Vec<&'static str> {
    vec![
        "OnClick",
        "OnHover",
        "OnPressed",
        "OnReleased",
        "OnValueChanged",
        "OnTextChanged",
        "OnFocus",
        "OnUnfocus",
        "OnNavigate",
        "OnSubmit",
        "OnCancel",
    ]
}

pub fn main_menu_canvas(game_title: &str) -> UiCanvas2D {
    game_menu_canvas(
        "MainMenu",
        game_title,
        &[
            ("ContinueButton", "Continue", "continue_game"),
            ("NewGameButton", "New Game", "new_game"),
            ("SettingsButton", "Settings", "open_settings"),
            ("QuitButton", "Quit", "quit_game"),
        ],
    )
}

pub fn pause_menu_canvas() -> UiCanvas2D {
    game_menu_canvas(
        "PauseMenu",
        "Paused",
        &[
            ("ResumeButton", "Resume", "resume_game"),
            ("RestartButton", "Restart", "restart_scene"),
            ("SettingsButton", "Settings", "open_settings"),
            ("MainMenuButton", "Main Menu", "open_main_menu"),
        ],
    )
}

pub fn settings_menu_canvas() -> UiCanvas2D {
    settings_controls_canvas("SettingsMenu")
}

pub fn game_menu_canvas(name: &str, title: &str, buttons: &[(&str, &str, &str)]) -> UiCanvas2D {
    let button_widgets = buttons
        .iter()
        .enumerate()
        .map(|(index, (id, label, command))| {
            let up = index
                .checked_sub(1)
                .and_then(|prev| buttons.get(prev))
                .map(|(id, _, _)| (*id).to_string());
            let down = buttons.get(index + 1).map(|(id, _, _)| (*id).to_string());
            UiWidget2D {
                id: (*id).to_string(),
                widget_type: "MenuButton".to_string(),
                rect: UiRect2D {
                    x: 64.0,
                    y: 128.0 + index as f32 * 56.0,
                    width: 280.0,
                    height: 44.0,
                },
                anchors: Anchor2D::TOP_LEFT,
                children: Vec::new(),
                callbacks: vec![UiCallback2D {
                    event: "OnClick".to_string(),
                    graph: None,
                    function: Some((*command).to_string()),
                    payload: json!({"command": command}),
                }],
                properties: json!({"text": label, "command": command}),
                style: UiStyle2D {
                    style_id: Some("menu_button".to_string()),
                    ..Default::default()
                },
                bindings: Vec::new(),
                navigation: UiNavigation2D {
                    up,
                    down,
                    ..Default::default()
                },
            }
        })
        .collect::<Vec<_>>();

    UiCanvas2D {
        name: name.to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        theme: UiTheme2D {
            name: "MiniForge Menu".to_string(),
            styles: BTreeMap::from([
                (
                    "menu_panel".to_string(),
                    UiStyle2D {
                        style_id: Some("menu_panel".to_string()),
                        background: Some([14, 18, 24, 238]),
                        foreground: Some([238, 244, 250, 255]),
                        font_size: Some(18.0),
                        padding: Some([24.0, 24.0, 24.0, 24.0]),
                        radius: Some(8.0),
                    },
                ),
                (
                    "menu_button".to_string(),
                    UiStyle2D {
                        style_id: Some("menu_button".to_string()),
                        background: Some([38, 50, 66, 255]),
                        foreground: Some([244, 248, 252, 255]),
                        font_size: Some(16.0),
                        padding: Some([14.0, 10.0, 14.0, 10.0]),
                        radius: Some(6.0),
                    },
                ),
            ]),
        },
        animations: vec![UiAnimation2D {
            name: "MenuFadeIn".to_string(),
            target_widget: "MenuPanel".to_string(),
            duration: 0.25,
            keyframes: vec![
                UiAnimationKey2D {
                    time: 0.0,
                    property: "opacity".to_string(),
                    value: json!(0.0),
                },
                UiAnimationKey2D {
                    time: 0.25,
                    property: "opacity".to_string(),
                    value: json!(1.0),
                },
            ],
        }],
        widgets: vec![UiWidget2D {
            id: "RootCanvas".to_string(),
            widget_type: "Canvas".to_string(),
            rect: UiRect2D {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            anchors: Anchor2D::FILL,
            callbacks: Vec::new(),
            properties: json!({"screen": name}),
            style: UiStyle2D::default(),
            bindings: Vec::new(),
            navigation: UiNavigation2D::default(),
            children: vec![UiWidget2D {
                id: "MenuPanel".to_string(),
                widget_type: "MenuStack".to_string(),
                rect: UiRect2D {
                    x: -220.0,
                    y: -220.0,
                    width: 440.0,
                    height: 440.0,
                },
                anchors: Anchor2D::CENTER,
                callbacks: Vec::new(),
                properties: json!({"title": title, "layout": "vertical"}),
                style: UiStyle2D {
                    style_id: Some("menu_panel".to_string()),
                    ..Default::default()
                },
                bindings: Vec::new(),
                navigation: UiNavigation2D::default(),
                children: {
                    let mut children = vec![UiWidget2D {
                        id: "TitleText".to_string(),
                        widget_type: "Text".to_string(),
                        rect: UiRect2D {
                            x: 64.0,
                            y: 48.0,
                            width: 312.0,
                            height: 48.0,
                        },
                        anchors: Anchor2D::TOP_LEFT,
                        children: Vec::new(),
                        callbacks: Vec::new(),
                        properties: json!({"text": title, "role": "title"}),
                        style: UiStyle2D {
                            style_id: Some("menu_panel".to_string()),
                            font_size: Some(28.0),
                            ..Default::default()
                        },
                        bindings: Vec::new(),
                        navigation: UiNavigation2D::default(),
                    }];
                    children.extend(button_widgets);
                    children
                },
            }],
        }],
    }
}

pub fn minimal_ui_canvas() -> UiCanvas2D {
    UiCanvas2D {
        name: "HUD_Main".to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        theme: UiTheme2D::default(),
        animations: vec![UiAnimation2D {
            name: "HealthPulse".to_string(),
            target_widget: "HealthBar".to_string(),
            duration: 0.35,
            keyframes: vec![
                UiAnimationKey2D {
                    time: 0.0,
                    property: "scale".to_string(),
                    value: json!(1.0),
                },
                UiAnimationKey2D {
                    time: 0.18,
                    property: "scale".to_string(),
                    value: json!(1.05),
                },
                UiAnimationKey2D {
                    time: 0.35,
                    property: "scale".to_string(),
                    value: json!(1.0),
                },
            ],
        }],
        widgets: vec![UiWidget2D {
            id: "RootCanvas".to_string(),
            widget_type: "Canvas".to_string(),
            rect: UiRect2D {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            anchors: Anchor2D::FILL,
            callbacks: Vec::new(),
            properties: json!({}),
            style: UiStyle2D::default(),
            bindings: Vec::new(),
            navigation: UiNavigation2D::default(),
            children: vec![
                UiWidget2D {
                    id: "HealthPanel".to_string(),
                    widget_type: "Panel".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 24.0,
                        width: 260.0,
                        height: 56.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    callbacks: Vec::new(),
                    properties: json!({"background": [24, 28, 34, 220]}),
                    style: UiStyle2D {
                        style_id: Some("panel".to_string()),
                        ..Default::default()
                    },
                    bindings: Vec::new(),
                    navigation: UiNavigation2D::default(),
                    children: vec![UiWidget2D {
                        id: "HealthBar".to_string(),
                        widget_type: "ProgressBar".to_string(),
                        rect: UiRect2D {
                            x: 16.0,
                            y: 16.0,
                            width: 228.0,
                            height: 20.0,
                        },
                        anchors: Anchor2D::TOP_LEFT,
                        children: Vec::new(),
                        callbacks: Vec::new(),
                        properties: json!({"value": 1.0, "fill": [220, 60, 72, 255]}),
                        style: UiStyle2D::default(),
                        bindings: vec![UiBinding2D {
                            property: "value".to_string(),
                            source_path: "player.health_percent".to_string(),
                            fallback: json!(1.0),
                        }],
                        navigation: UiNavigation2D::default(),
                    }],
                },
                UiWidget2D {
                    id: "StartButton".to_string(),
                    widget_type: "Button".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 96.0,
                        width: 160.0,
                        height: 40.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    children: Vec::new(),
                    callbacks: vec![UiCallback2D {
                        event: "click".to_string(),
                        graph: Some("scripts/visual_graphs/StartGame.mfgraph".to_string()),
                        function: None,
                        payload: json!({"command": "start"}),
                    }],
                    properties: json!({"text": "Start"}),
                    style: UiStyle2D {
                        style_id: Some("button".to_string()),
                        ..Default::default()
                    },
                    bindings: Vec::new(),
                    navigation: UiNavigation2D {
                        down: Some("CoinText".to_string()),
                        ..Default::default()
                    },
                },
                UiWidget2D {
                    id: "CoinText".to_string(),
                    widget_type: "Text".to_string(),
                    rect: UiRect2D {
                        x: 24.0,
                        y: 148.0,
                        width: 160.0,
                        height: 32.0,
                    },
                    anchors: Anchor2D::TOP_LEFT,
                    children: Vec::new(),
                    callbacks: Vec::new(),
                    properties: json!({"text": "Coins: 0", "binding": "player.coins"}),
                    style: UiStyle2D::default(),
                    bindings: vec![UiBinding2D {
                        property: "text".to_string(),
                        source_path: "player.coins".to_string(),
                        fallback: json!(0),
                    }],
                    navigation: UiNavigation2D {
                        up: Some("StartButton".to_string()),
                        ..Default::default()
                    },
                },
            ],
        }],
    }
}

pub fn hud_screen_canvas() -> UiCanvas2D {
    let mut canvas = minimal_ui_canvas();
    canvas.name = "HUDScreen".to_string();
    if let Some(root) = canvas.widgets.first_mut() {
        root.properties = json!({"screen": "HUDScreen", "layer": "hud"});
        root.children.push(ui_widget(
            "ObjectiveLabel",
            "Label",
            UiRect2D {
                x: 24.0,
                y: 188.0,
                width: 380.0,
                height: 28.0,
            },
            Anchor2D::TOP_LEFT,
            json!({"text": "Objective: survive and explore", "role": "objective"}),
            "default",
        ));
        root.children.push(ui_widget(
            "PortraitImage",
            "Image",
            UiRect2D {
                x: -112.0,
                y: 24.0,
                width: 88.0,
                height: 88.0,
            },
            Anchor2D::TOP_RIGHT,
            json!({"sprite": "assets/ui/player_portrait.png", "fit": "contain"}),
            "panel",
        ));
    }
    canvas
}

pub fn inventory_screen_canvas() -> UiCanvas2D {
    root_canvas(
        "InventoryScreen",
        vec![ui_widget(
            "InventoryPanel",
            "Panel",
            UiRect2D {
                x: -260.0,
                y: -210.0,
                width: 520.0,
                height: 420.0,
            },
            Anchor2D::CENTER,
            json!({"title": "Inventory"}),
            "panel",
        )
        .with_children(vec![
            ui_widget(
                "InventoryTitle",
                "Label",
                UiRect2D {
                    x: 24.0,
                    y: 22.0,
                    width: 240.0,
                    height: 34.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"text": "Inventory", "role": "title"}),
                "panel",
            ),
            ui_widget(
                "InventoryGrid",
                "InventoryGrid",
                UiRect2D {
                    x: 24.0,
                    y: 74.0,
                    width: 360.0,
                    height: 288.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"columns": 6, "rows": 4, "slot_size": 52, "binding": "player.inventory"}),
                "panel",
            ),
            button_widget(
                "CloseInventoryButton",
                "Close",
                "close_inventory",
                UiRect2D {
                    x: 396.0,
                    y: 338.0,
                    width: 96.0,
                    height: 42.0,
                },
                Anchor2D::TOP_LEFT,
            ),
        ])],
    )
}

pub fn survival_hud_canvas() -> UiCanvas2D {
    let mut rows = Vec::new();
    let specs = [
        ("Health", "player.health.percent", 1.0, [220, 60, 72, 255]),
        ("Hunger", "player.needs.hunger", 100.0, [214, 151, 54, 255]),
        ("Thirst", "player.needs.thirst", 100.0, [55, 158, 207, 255]),
        ("Energy", "player.needs.energy", 100.0, [65, 184, 108, 255]),
        (
            "Stamina",
            "player.needs.stamina",
            100.0,
            [136, 101, 187, 255],
        ),
    ];
    for (index, (label, path, max, color)) in specs.into_iter().enumerate() {
        let y = 18.0 + index as f32 * 42.0;
        rows.push(ui_widget(
            &format!("{label}Label"),
            "Label",
            UiRect2D {
                x: 16.0,
                y,
                width: 72.0,
                height: 20.0,
            },
            Anchor2D::TOP_LEFT,
            json!({"text": label}),
            "panel",
        ));
        let mut bar = ui_widget(
            &format!("{label}Bar"),
            "ProgressBar",
            UiRect2D {
                x: 92.0,
                y,
                width: 172.0,
                height: 18.0,
            },
            Anchor2D::TOP_LEFT,
            json!({"value": max, "max": max, "fill": color}),
            "panel",
        );
        bar.bindings.push(UiBinding2D {
            property: "value".to_string(),
            source_path: path.to_string(),
            fallback: json!(max),
        });
        rows.push(bar);
    }
    root_canvas(
        "SurvivalHUD",
        vec![
            ui_widget(
                "SurvivalStatusPanel",
                "Panel",
                UiRect2D {
                    x: 24.0,
                    y: 24.0,
                    width: 282.0,
                    height: 228.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"title": "Status"}),
                "panel",
            )
            .with_children(rows),
        ],
    )
}

pub fn dialogue_screen_canvas() -> UiCanvas2D {
    root_canvas(
        "DialogueScreen",
        vec![ui_widget(
            "DialoguePanel",
            "DialogueBox",
            UiRect2D {
                x: -420.0,
                y: -176.0,
                width: 840.0,
                height: 156.0,
            },
            Anchor2D::BOTTOM_CENTER,
            json!({"speaker": "Guide", "text": "Dialogue text goes here.", "binding": "dialogue.current"}),
            "panel",
        )
        .with_children(vec![
            ui_widget(
                "SpeakerLabel",
                "Label",
                UiRect2D {
                    x: 24.0,
                    y: 18.0,
                    width: 240.0,
                    height: 28.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"text": "Guide", "binding": "dialogue.speaker"}),
                "panel",
            ),
            ui_widget(
                "DialogueText",
                "RichText",
                UiRect2D {
                    x: 24.0,
                    y: 56.0,
                    width: 640.0,
                    height: 72.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"text": "Dialogue text goes here.", "binding": "dialogue.text"}),
                "default",
            ),
            button_widget(
                "DialogueNextButton",
                "Next",
                "dialogue_next",
                UiRect2D {
                    x: 704.0,
                    y: 92.0,
                    width: 104.0,
                    height: 40.0,
                },
                Anchor2D::TOP_LEFT,
            ),
        ])],
    )
}

pub fn game_over_screen_canvas() -> UiCanvas2D {
    game_menu_canvas(
        "GameOverScreen",
        "Game Over",
        &[
            ("RetryButton", "Retry", "restart_scene"),
            ("LevelSelectButton", "Level Select", "open_level_select"),
            ("MainMenuButton", "Main Menu", "open_main_menu"),
        ],
    )
}

pub fn level_select_screen_canvas() -> UiCanvas2D {
    root_canvas(
        "LevelSelectScreen",
        vec![
            ui_widget(
                "LevelSelectPanel",
                "Panel",
                UiRect2D {
                    x: -300.0,
                    y: -220.0,
                    width: 600.0,
                    height: 440.0,
                },
                Anchor2D::CENTER,
                json!({"title": "Level Select"}),
                "panel",
            )
            .with_children(vec![
                ui_widget(
                    "LevelSelectTitle",
                    "Label",
                    UiRect2D {
                        x: 32.0,
                        y: 28.0,
                        width: 300.0,
                        height: 38.0,
                    },
                    Anchor2D::TOP_LEFT,
                    json!({"text": "Level Select", "role": "title"}),
                    "panel",
                ),
                button_widget(
                    "LevelOneButton",
                    "Level 1",
                    "load_level_1",
                    UiRect2D {
                        x: 44.0,
                        y: 108.0,
                        width: 150.0,
                        height: 58.0,
                    },
                    Anchor2D::TOP_LEFT,
                ),
                button_widget(
                    "LevelTwoButton",
                    "Level 2",
                    "load_level_2",
                    UiRect2D {
                        x: 224.0,
                        y: 108.0,
                        width: 150.0,
                        height: 58.0,
                    },
                    Anchor2D::TOP_LEFT,
                ),
                button_widget(
                    "LevelThreeButton",
                    "Level 3",
                    "load_level_3",
                    UiRect2D {
                        x: 404.0,
                        y: 108.0,
                        width: 150.0,
                        height: 58.0,
                    },
                    Anchor2D::TOP_LEFT,
                ),
                button_widget(
                    "BackButton",
                    "Back",
                    "back",
                    UiRect2D {
                        x: 424.0,
                        y: 360.0,
                        width: 130.0,
                        height: 44.0,
                    },
                    Anchor2D::TOP_LEFT,
                ),
            ]),
        ],
    )
}

pub fn settings_screen_canvas() -> UiCanvas2D {
    settings_controls_canvas("SettingsScreen")
}

pub fn main_menu_screen(game_title: &str) -> UIScreen2D {
    UIScreen2D::new(
        "MainMenuScreen",
        UiScreenKind2D::MainMenu,
        main_menu_canvas(game_title),
    )
    .visible(true)
    .modal(true)
    .sort_order(100)
    .input_context("menu")
}

pub fn pause_screen() -> UIScreen2D {
    UIScreen2D::new("PauseScreen", UiScreenKind2D::Pause, pause_menu_canvas())
        .modal(true)
        .sort_order(200)
        .input_context("menu")
}

pub fn settings_screen() -> UIScreen2D {
    UIScreen2D::new(
        "SettingsScreen",
        UiScreenKind2D::Settings,
        settings_screen_canvas(),
    )
    .modal(true)
    .sort_order(300)
    .input_context("menu")
}

pub fn hud_screen() -> UIScreen2D {
    UIScreen2D::new("HUDScreen", UiScreenKind2D::Hud, hud_screen_canvas())
        .visible(true)
        .persistent()
        .sort_order(0)
        .input_context("gameplay")
}

pub fn inventory_screen() -> UIScreen2D {
    UIScreen2D::new(
        "InventoryScreen",
        UiScreenKind2D::Inventory,
        inventory_screen_canvas(),
    )
    .modal(true)
    .sort_order(160)
    .input_context("inventory")
}

pub fn dialogue_screen() -> UIScreen2D {
    UIScreen2D::new(
        "DialogueScreen",
        UiScreenKind2D::Dialogue,
        dialogue_screen_canvas(),
    )
    .modal(true)
    .sort_order(180)
    .input_context("dialogue")
}

pub fn game_over_screen() -> UIScreen2D {
    UIScreen2D::new(
        "GameOverScreen",
        UiScreenKind2D::GameOver,
        game_over_screen_canvas(),
    )
    .modal(true)
    .sort_order(400)
    .input_context("menu")
}

pub fn level_select_screen() -> UIScreen2D {
    UIScreen2D::new(
        "LevelSelectScreen",
        UiScreenKind2D::LevelSelect,
        level_select_screen_canvas(),
    )
    .modal(true)
    .sort_order(350)
    .input_context("menu")
}

pub fn standard_screen_manager(game_title: &str) -> ScreenManager2D {
    ScreenManager2D::standard(game_title)
}

pub fn canonical_widget_type(widget_type: &str) -> &str {
    match widget_type {
        "Label" => "Text",
        "TextInput" => "InputField",
        other => other,
    }
}

pub fn is_interactive_widget_type(widget_type: &str) -> bool {
    matches!(
        canonical_widget_type(widget_type),
        "Button"
            | "MenuButton"
            | "IconButton"
            | "Slider"
            | "Checkbox"
            | "InputField"
            | "Dropdown"
            | "InventorySlot"
            | "RadialMenu"
    )
}

fn settings_controls_canvas(name: &str) -> UiCanvas2D {
    root_canvas(
        name,
        vec![ui_widget(
            "SettingsPanel",
            "Panel",
            UiRect2D {
                x: -260.0,
                y: -210.0,
                width: 520.0,
                height: 420.0,
            },
            Anchor2D::CENTER,
            json!({"title": "Settings"}),
            "panel",
        )
        .with_children(vec![
            ui_widget(
                "SettingsTitle",
                "Label",
                UiRect2D {
                    x: 28.0,
                    y: 26.0,
                    width: 240.0,
                    height: 38.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"text": "Settings", "role": "title"}),
                "panel",
            ),
            ui_widget(
                "MasterVolumeSlider",
                "Slider",
                UiRect2D {
                    x: 28.0,
                    y: 98.0,
                    width: 360.0,
                    height: 28.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"label": "Master Volume", "value": 0.8, "min": 0.0, "max": 1.0, "command": "set_master_volume"}),
                "button",
            ),
            ui_widget(
                "FullscreenCheckbox",
                "Checkbox",
                UiRect2D {
                    x: 28.0,
                    y: 154.0,
                    width: 280.0,
                    height: 30.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"text": "Fullscreen", "checked": false, "command": "toggle_fullscreen"}),
                "button",
            ),
            ui_widget(
                "PlayerNameInput",
                "TextInput",
                UiRect2D {
                    x: 28.0,
                    y: 212.0,
                    width: 360.0,
                    height: 38.0,
                },
                Anchor2D::TOP_LEFT,
                json!({"placeholder": "Player name", "text": "", "command": "set_player_name"}),
                "button",
            ),
            button_widget(
                "BackButton",
                "Back",
                "back",
                UiRect2D {
                    x: 368.0,
                    y: 340.0,
                    width: 112.0,
                    height: 44.0,
                },
                Anchor2D::TOP_LEFT,
            ),
        ])],
    )
}

trait UiWidgetBuilder2D {
    fn with_children(self, children: Vec<UiWidget2D>) -> Self;
}

impl UiWidgetBuilder2D for UiWidget2D {
    fn with_children(mut self, children: Vec<UiWidget2D>) -> Self {
        self.children = children;
        self
    }
}

fn root_canvas(name: &str, children: Vec<UiWidget2D>) -> UiCanvas2D {
    UiCanvas2D {
        name: name.to_string(),
        viewport_width: 1280.0,
        viewport_height: 720.0,
        theme: UiTheme2D::default(),
        animations: Vec::new(),
        widgets: vec![
            ui_widget(
                "RootCanvas",
                "Canvas",
                UiRect2D {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
                Anchor2D::FILL,
                json!({"screen": name}),
                "default",
            )
            .with_children(children),
        ],
    }
}

fn ui_widget(
    id: &str,
    widget_type: &str,
    rect: UiRect2D,
    anchors: Anchor2D,
    properties: Value,
    style_id: &str,
) -> UiWidget2D {
    UiWidget2D {
        id: id.to_string(),
        widget_type: widget_type.to_string(),
        rect,
        anchors,
        children: Vec::new(),
        callbacks: Vec::new(),
        properties,
        style: UiStyle2D {
            style_id: Some(style_id.to_string()),
            ..Default::default()
        },
        bindings: Vec::new(),
        navigation: UiNavigation2D::default(),
    }
}

fn button_widget(
    id: &str,
    label: &str,
    command: &str,
    rect: UiRect2D,
    anchors: Anchor2D,
) -> UiWidget2D {
    let mut widget = ui_widget(
        id,
        "Button",
        rect,
        anchors,
        json!({"text": label, "command": command}),
        "button",
    );
    widget.callbacks.push(command_callback(command));
    widget
}

fn command_callback(command: &str) -> UiCallback2D {
    UiCallback2D {
        event: "OnClick".to_string(),
        graph: None,
        function: Some(command.to_string()),
        payload: json!({"command": command}),
    }
}

fn widget_command(widget: &UiWidget2D) -> Option<String> {
    widget
        .properties
        .get("command")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            widget
                .callbacks
                .iter()
                .find(|callback| {
                    callback.event.eq_ignore_ascii_case("click")
                        || callback.event.eq_ignore_ascii_case("OnClick")
                        || callback.event.eq_ignore_ascii_case("OnPressed")
                })
                .and_then(|callback| {
                    callback
                        .payload
                        .get("command")
                        .and_then(Value::as_str)
                        .or(callback.function.as_deref())
                        .or(callback.graph.as_deref())
                        .map(ToString::to_string)
                })
        })
}

fn collect_widget_id(widget: &UiWidget2D, ids: &mut BTreeSet<String>) -> bool {
    if !ids.insert(widget.id.clone()) {
        return false;
    }
    widget
        .children
        .iter()
        .all(|child| collect_widget_id(child, ids))
}

fn find_widget<'a>(widget: &'a UiWidget2D, id: &str) -> Option<&'a UiWidget2D> {
    if widget.id == id {
        return Some(widget);
    }
    widget
        .children
        .iter()
        .find_map(|child| find_widget(child, id))
}

fn find_widget_mut<'a>(widget: &'a mut UiWidget2D, id: &str) -> Option<&'a mut UiWidget2D> {
    if widget.id == id {
        return Some(widget);
    }
    widget
        .children
        .iter_mut()
        .find_map(|child| find_widget_mut(child, id))
}

fn flatten_widget<'a>(widget: &'a UiWidget2D, widgets: &mut Vec<&'a UiWidget2D>) {
    widgets.push(widget);
    for child in &widget.children {
        flatten_widget(child, widgets);
    }
}

fn resolve_widget_layout(
    widget: &UiWidget2D,
    parent: UiRect2D,
    inherited_clip: Option<UiRect2D>,
    depth: usize,
    resolved: &mut Vec<UiResolvedWidget2D>,
) {
    if widget.properties.get("visible").and_then(Value::as_bool) == Some(false) {
        return;
    }
    let anchor_x = parent.x + parent.width * widget.anchors.min_x;
    let anchor_y = parent.y + parent.height * widget.anchors.min_y;
    let stretch_w = parent.width * (widget.anchors.max_x - widget.anchors.min_x);
    let stretch_h = parent.height * (widget.anchors.max_y - widget.anchors.min_y);
    let canvas_stretch = widget.widget_type == "Canvas";
    let rect = UiRect2D {
        x: anchor_x + widget.rect.x,
        y: anchor_y + widget.rect.y,
        width: if stretch_w > 0.0 {
            stretch_w
                + if canvas_stretch {
                    widget.rect.width.min(0.0)
                } else {
                    widget.rect.width
                }
        } else {
            widget.rect.width
        },
        height: if stretch_h > 0.0 {
            stretch_h
                + if canvas_stretch {
                    widget.rect.height.min(0.0)
                } else {
                    widget.rect.height
                }
        } else {
            widget.rect.height
        },
    };
    if !ui_rect_is_finite(rect) || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let clip_rect = inherited_clip.and_then(|clip| intersect_ui_rect(clip, rect));
    if inherited_clip.is_some() && clip_rect.is_none() {
        return;
    }
    resolved.push(UiResolvedWidget2D {
        id: widget.id.clone(),
        widget_type: widget.widget_type.clone(),
        rect,
        clip_rect,
        interactive: is_interactive_widget_type(&widget.widget_type)
            || widget.callbacks.iter().any(|callback| {
                callback.event.eq_ignore_ascii_case("click")
                    || callback.event.eq_ignore_ascii_case("OnClick")
                    || callback.event.eq_ignore_ascii_case("OnPressed")
            }),
        depth,
    });
    let clips_children = widget.widget_type.eq_ignore_ascii_case("ScrollBox")
        || widget
            .properties
            .get("clip_children")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let child_clip = if clips_children {
        match inherited_clip {
            Some(clip) => intersect_ui_rect(clip, rect),
            None => Some(rect),
        }
    } else {
        inherited_clip
    };
    let scroll_x = finite_property_f32(&widget.properties, "scroll_x").max(0.0);
    let scroll_y = finite_property_f32(&widget.properties, "scroll_y").max(0.0);
    let child_parent = UiRect2D {
        x: rect.x - scroll_x,
        y: rect.y - scroll_y,
        ..rect
    };
    for child in &widget.children {
        resolve_widget_layout(child, child_parent, child_clip, depth + 1, resolved);
    }
}

fn finite_property_f32(properties: &Value, key: &str) -> f32 {
    properties
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0) as f32
}

fn ui_rect_is_finite(rect: UiRect2D) -> bool {
    [rect.x, rect.y, rect.width, rect.height]
        .into_iter()
        .all(f32::is_finite)
}

fn intersect_ui_rect(left: UiRect2D, right: UiRect2D) -> Option<UiRect2D> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom_edge = (left.y + left.height).min(right.y + right.height);
    (right_edge > x && bottom_edge > y).then_some(UiRect2D {
        x,
        y,
        width: right_edge - x,
        height: bottom_edge - y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget(id: &str, widget_type: &str, rect: UiRect2D) -> UiWidget2D {
        UiWidget2D {
            id: id.to_string(),
            widget_type: widget_type.to_string(),
            rect,
            anchors: Anchor2D::TOP_LEFT,
            children: Vec::new(),
            callbacks: Vec::new(),
            properties: json!({}),
            style: UiStyle2D::default(),
            bindings: Vec::new(),
            navigation: UiNavigation2D::default(),
        }
    }

    #[test]
    fn scroll_layout_offsets_and_clips_children() {
        let mut scroll = widget(
            "feed",
            "ScrollBox",
            UiRect2D {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 60.0,
            },
        );
        scroll.properties = json!({"scroll_y": 40.0});
        scroll.children.push(widget(
            "entry",
            "Button",
            UiRect2D {
                x: 5.0,
                y: 70.0,
                width: 90.0,
                height: 40.0,
            },
        ));
        scroll.children.push(widget(
            "hidden",
            "Label",
            UiRect2D {
                x: 5.0,
                y: 140.0,
                width: 90.0,
                height: 20.0,
            },
        ));
        let canvas = UiCanvas2D {
            name: "Scroll".to_string(),
            viewport_width: 320.0,
            viewport_height: 180.0,
            widgets: vec![scroll],
            theme: UiTheme2D::default(),
            animations: Vec::new(),
        };

        let layout = canvas.resolve_layout((320.0, 180.0));
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[1].id, "entry");
        assert_eq!(layout[1].rect.y, 50.0);
        assert_eq!(
            layout[1].clip_rect,
            Some(UiRect2D {
                x: 15.0,
                y: 50.0,
                width: 90.0,
                height: 30.0,
            })
        );
    }

    #[test]
    fn invisible_and_invalid_widgets_are_not_resolved() {
        let mut hidden = widget(
            "hidden",
            "Panel",
            UiRect2D {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        );
        hidden.properties = json!({"visible": false});
        hidden.children.push(widget(
            "hidden_child",
            "Button",
            UiRect2D {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height: 20.0,
            },
        ));
        let invalid = widget(
            "invalid",
            "Label",
            UiRect2D {
                x: f32::NAN,
                y: 0.0,
                width: 40.0,
                height: 20.0,
            },
        );
        let canvas = UiCanvas2D {
            name: "Visibility".to_string(),
            viewport_width: 320.0,
            viewport_height: 180.0,
            widgets: vec![hidden, invalid],
            theme: UiTheme2D::default(),
            animations: Vec::new(),
        };

        assert!(canvas.resolve_layout((320.0, 180.0)).is_empty());
    }
}
