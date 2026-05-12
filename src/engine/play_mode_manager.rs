use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct PlayModeManager {
    pub snapshot: Option<Vec<GameObject>>,
}

impl PlayModeManager {
    pub fn enter_play_mode(&mut self, entities: &[GameObject], mode: &mut String) {
        self.snapshot = Some(entities.to_vec());
        *mode = "PLAY".to_string();
    }

    pub fn exit_play_mode(&mut self, entities: &mut Vec<GameObject>, mode: &mut String) {
        if let Some(snapshot) = self.snapshot.take() {
            *entities = snapshot;
        }
        *mode = "EDITOR".to_string();
    }

    pub fn toggle(&mut self, entities: &mut Vec<GameObject>, mode: &mut String) {
        if mode == "PLAY" {
            self.exit_play_mode(entities, mode);
        } else {
            self.enter_play_mode(entities, mode);
        }
    }
}
