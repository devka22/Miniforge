use std::io;
use std::path::Path;

use crate::core::game::Game;

pub fn run(project_path: impl AsRef<Path>) -> io::Result<Game> {
    let mut game = Game::from_project(project_path, true)?;
    game.run_headless_once(1.0 / 60.0);
    Ok(game)
}
