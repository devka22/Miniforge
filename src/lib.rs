#![recursion_limit = "256"]

pub mod core;
#[cfg(feature = "editor")]
pub mod editor_app;
#[cfg(feature = "editor_ffi")]
pub mod editor_ffi;
pub mod engine;
pub mod entities;
pub mod input;
pub mod map;
pub mod pathfinding;
pub mod render;
pub mod runtime;
pub mod systems;

#[cfg(feature = "editor")]
pub use core::game::Game;
pub use engine::version::{
    DEVELOPMENT_VERSION, ENGINE_VERSION, development_version_label, version_label,
};
pub use engine::world::RuntimeWorld;
pub use runtime::EngineRuntime;
