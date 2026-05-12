use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct EditorHistory {
    pub undo: Vec<(String, Vec<GameObject>)>,
    pub redo: Vec<(String, Vec<GameObject>)>,
}

impl EditorHistory {
    pub fn take_snapshot(&mut self, label: &str, entities: &[GameObject]) {
        self.undo.push((label.to_string(), entities.to_vec()));
        self.redo.clear();
    }

    pub fn undo(&mut self, current: &mut Vec<GameObject>) -> Option<String> {
        let (label, snapshot) = self.undo.pop()?;
        self.redo.push((label.clone(), current.clone()));
        *current = snapshot;
        Some(label)
    }

    pub fn redo(&mut self, current: &mut Vec<GameObject>) -> Option<String> {
        let (label, snapshot) = self.redo.pop()?;
        self.undo.push((label.clone(), current.clone()));
        *current = snapshot;
        Some(label)
    }
}
