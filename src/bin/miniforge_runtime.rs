use miniforge::editor_app::{run_exported_runtime_player, runtime_player_window_conf};

#[macroquad::main(runtime_player_window_conf)]
async fn main() {
    run_exported_runtime_player().await;
}
