use std::collections::VecDeque;

use crate::engine::formation::Formation;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub enum RTSCommand {
    Move {
        unit_id: u64,
        target: (f64, f64),
    },
    Stop {
        unit_id: u64,
    },
    Hold {
        unit_id: u64,
    },
    Patrol {
        unit_id: u64,
        target: (f64, f64),
    },
    AttackMove {
        unit_id: u64,
        target: (f64, f64),
    },
    Gather {
        unit_id: u64,
        target_id: u64,
    },
    FormationMove {
        unit_ids: Vec<u64>,
        target: (f64, f64),
        formation: String,
        spacing: f64,
    },
}

#[derive(Debug, Clone, Default)]
pub struct RTSCommandQueue {
    pub queue: VecDeque<RTSCommand>,
}

impl RTSCommandQueue {
    pub fn push(&mut self, command: RTSCommand) {
        self.queue.push_back(command);
    }

    pub fn update(&mut self, entities: &mut [GameObject]) {
        while let Some(command) = self.queue.pop_front() {
            match command {
                RTSCommand::Move { unit_id, target } => {
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "MOVE".to_string();
                        unit.path = vec![target];
                    }
                }
                RTSCommand::Stop { unit_id } => {
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "STOP".to_string();
                        unit.path.clear();
                    }
                }
                RTSCommand::Hold { unit_id } => {
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "HOLD".to_string();
                        unit.path.clear();
                    }
                }
                RTSCommand::Patrol { unit_id, target } => {
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "PATROL".to_string();
                        unit.patrol_points = vec![(unit.x, unit.y), target];
                        unit.patrol_index = 0;
                        unit.path = vec![target];
                    }
                }
                RTSCommand::AttackMove { unit_id, target } => {
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "ATTACK_MOVE".to_string();
                        unit.attack_move_target = Some(target);
                        unit.path = vec![target];
                    }
                }
                RTSCommand::Gather { unit_id, target_id } => {
                    let target_position = entities
                        .iter()
                        .find(|entity| entity.id == target_id)
                        .map(|entity| (entity.x, entity.y));
                    if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id) {
                        unit.command = "GATHER".to_string();
                        unit.gather_target_id = Some(target_id);
                        if let Some(worker) = unit.get_component_mut("Worker") {
                            worker.set("gather_target_id", serde_json::json!(target_id));
                        }
                        if let Some(target) = target_position {
                            unit.path = vec![target];
                        }
                    }
                }
                RTSCommand::FormationMove {
                    unit_ids,
                    target,
                    formation,
                    spacing,
                } => {
                    let positions =
                        Formation::positions(&formation, unit_ids.len(), target, spacing);
                    for (unit_id, position) in unit_ids.into_iter().zip(positions) {
                        if let Some(unit) = entities.iter_mut().find(|entity| entity.id == unit_id)
                        {
                            unit.command = "FORMATION_MOVE".to_string();
                            unit.state = "MOVING".to_string();
                            unit.path = vec![position];
                        }
                    }
                }
            }
        }
    }
}
