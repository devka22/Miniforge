//! Shared, renderer-agnostic editor UI state.
//!
//! Macroquad, egui and future frontends consume these models instead of keeping
//! separate copies of menu, toolbar and interaction state.

pub mod button;
pub mod menu_bar;
pub mod toolbar;

pub use button::{Button, ButtonRole, ButtonVisualState};
pub use menu_bar::{Menu, MenuBar, MenuItem};
pub use toolbar::{EditorTool, ToolDescriptor, Toolbar};
