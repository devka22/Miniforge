use serde::{Deserialize, Serialize};

use crate::engine::developer_console::{ConsoleEntry, ConsoleSeverity, DeveloperConsole};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsoleCommand2D {
    pub name: String,
    pub usage: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConsoleCommandResult2D {
    Clear,
    ReloadAssets,
    ReloadScripts,
    ValidateProject,
    OpenScene(String),
    Play,
    Pause,
    Stop,
    BuildDebug,
    BuildRelease,
    Export,
    ShowFps(bool),
    Help(Vec<ConsoleCommand2D>),
    Unknown(String),
}

#[derive(Debug, Clone, Default)]
pub struct ConsolePanel2D {
    pub console: DeveloperConsole,
    pub copied_text: String,
}

impl ConsolePanel2D {
    pub fn commands() -> Vec<ConsoleCommand2D> {
        vec![
            command("help", "help", "Lista comandos internos."),
            command("clear", "clear", "Limpia la consola."),
            command("reload_assets", "reload_assets", "Reescanea AssetDatabase."),
            command("reload_scripts", "reload_scripts", "Recarga scripts Rhai."),
            command(
                "validate_project",
                "validate_project",
                "Ejecuta validadores.",
            ),
            command("open_scene", "open_scene <name>", "Abre una escena."),
            command("play", "play", "Inicia Play Mode."),
            command("pause", "pause", "Pausa Play Mode."),
            command("stop", "stop", "Detiene Play Mode."),
            command("build_debug", "build_debug", "Export debug."),
            command("build_release", "build_release", "Export release."),
            command("export", "export", "Exporta con perfil activo."),
            command("show_fps", "show_fps", "Muestra FPS."),
            command("hide_fps", "hide_fps", "Oculta FPS."),
        ]
    }

    pub fn execute(&mut self, line: &str) -> ConsoleCommandResult2D {
        let mut parts = line.split_whitespace();
        match parts.next().unwrap_or("") {
            "help" => ConsoleCommandResult2D::Help(Self::commands()),
            "clear" => {
                self.console.clear();
                ConsoleCommandResult2D::Clear
            }
            "reload_assets" => ConsoleCommandResult2D::ReloadAssets,
            "reload_scripts" => ConsoleCommandResult2D::ReloadScripts,
            "validate_project" => ConsoleCommandResult2D::ValidateProject,
            "open_scene" => {
                ConsoleCommandResult2D::OpenScene(parts.next().unwrap_or("").to_string())
            }
            "play" => ConsoleCommandResult2D::Play,
            "pause" => ConsoleCommandResult2D::Pause,
            "stop" => ConsoleCommandResult2D::Stop,
            "build_debug" => ConsoleCommandResult2D::BuildDebug,
            "build_release" => ConsoleCommandResult2D::BuildRelease,
            "export" => ConsoleCommandResult2D::Export,
            "show_fps" => ConsoleCommandResult2D::ShowFps(true),
            "hide_fps" => ConsoleCommandResult2D::ShowFps(false),
            other => ConsoleCommandResult2D::Unknown(other.to_string()),
        }
    }

    pub fn filtered(&self, query: &str, min_severity: ConsoleSeverity) -> Vec<ConsoleEntry> {
        self.console.search(query, min_severity)
    }

    pub fn copy_filtered(&mut self, query: &str, min_severity: ConsoleSeverity) -> String {
        self.copied_text = self
            .filtered(query, min_severity)
            .into_iter()
            .map(|entry| format!("{:?} [{}] {}", entry.severity, entry.channel, entry.message))
            .collect::<Vec<_>>()
            .join("\n");
        self.copied_text.clone()
    }
}

fn command(name: &str, usage: &str, description: &str) -> ConsoleCommand2D {
    ConsoleCommand2D {
        name: name.to_string(),
        usage: usage.to_string(),
        description: description.to_string(),
    }
}
