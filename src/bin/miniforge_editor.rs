use miniforge::editor_app::{editor_window_conf, run_editor_async};

#[macroquad::main(editor_window_conf)]
async fn main() {
    run_editor_async().await;
}
