pub mod core;
pub mod engine;
pub mod entities;
pub mod input;
pub mod map;
pub mod pathfinding;
pub mod render;
pub mod runtime;
pub mod systems;

pub use core::game::Game;
pub use engine::version::{ENGINE_VERSION, version_label};
