use crate::entities::game_object::GameObject;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MovementFrameReport {
    pub updated: usize,
    pub skipped: usize,
    pub sanitized: usize,
    pub parallel: bool,
}

#[derive(Debug, Clone, Default)]
pub struct MovementSystem;

impl MovementSystem {
    pub fn update_entities(&self, entities: &mut [GameObject], dt: f64) {
        let _ = self.update_entities_with_report(entities, dt);
    }

    pub fn update_entities_with_report(
        &self,
        entities: &mut [GameObject],
        dt: f64,
    ) -> MovementFrameReport {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        let parallel = entities.len() >= 256;
        let update = |entity: &mut GameObject| -> (bool, bool) {
            if !entity.enabled || !entity.active {
                return (false, false);
            }
            let invalid = !entity.x.is_finite()
                || !entity.y.is_finite()
                || !entity.speed.is_finite()
                || !entity.rotation.is_finite();
            if invalid {
                entity.x = finite_or(entity.x, 0.0);
                entity.y = finite_or(entity.y, 0.0);
                entity.speed = finite_or(entity.speed, 0.0).max(0.0);
                entity.rotation = finite_or(entity.rotation, 0.0);
            }
            entity.update_movement(dt);
            entity.sync_to_components();
            (true, invalid)
        };

        let outcomes: Vec<_> = if parallel {
            entities.par_iter_mut().map(update).collect()
        } else {
            entities.iter_mut().map(update).collect()
        };
        let updated = outcomes.iter().filter(|(updated, _)| *updated).count();
        MovementFrameReport {
            updated,
            skipped: entities.len().saturating_sub(updated),
            sanitized: outcomes.iter().filter(|(_, sanitized)| *sanitized).count(),
            parallel,
        }
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}
