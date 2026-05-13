use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct PlayModeManager {
    pub snapshot: Option<Vec<GameObject>>,
    pub enter_count: usize,
    pub frame_count: usize,
    pub last_exit_reason: String,
    /// Frames simulados en la última sesión de Play (actualizado al salir).
    pub last_session_frames: usize,
    /// Entidades capturadas en el snapshot al entrar a Play.
    pub last_session_entity_count: usize,
}

impl PlayModeManager {
    pub fn enter_play_mode(&mut self, entities: &[GameObject], mode: &mut String) {
        self.snapshot = Some(entities.to_vec());
        self.enter_count += 1;
        self.frame_count = 0;
        self.last_session_entity_count = entities.len();
        self.last_exit_reason.clear();
        *mode = "PLAY".to_string();
    }

    pub fn exit_play_mode(
        &mut self,
        entities: &mut Vec<GameObject>,
        mode: &mut String,
        reason: &str,
    ) {
        self.last_session_frames = self.frame_count;
        if let Some(snapshot) = self.snapshot.take() {
            *entities = snapshot;
        }
        self.last_exit_reason = reason.to_string();
        *mode = "EDITOR".to_string();
        self.frame_count = 0;
    }

    pub fn toggle(&mut self, entities: &mut Vec<GameObject>, mode: &mut String) {
        if mode == "PLAY" {
            self.exit_play_mode(entities, mode, "toggle");
        } else {
            self.enter_play_mode(entities, mode);
        }
    }

    pub fn tick_frame(&mut self) {
        if self.snapshot.is_some() {
            self.frame_count += 1;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.snapshot.is_some()
    }
}
