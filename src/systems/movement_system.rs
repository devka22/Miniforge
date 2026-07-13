use crate::entities::game_object::GameObject;
use rayon::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MovementFrameReport {
    pub updated: usize,
    pub skipped: usize,
    pub sanitized: usize,
    pub component_syncs: usize,
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
        let update = |entity: &mut GameObject| -> (bool, bool, bool) {
            if !entity.enabled || !entity.active {
                return (false, false, false);
            }
            let before = (
                entity.x,
                entity.y,
                entity.rotation,
                entity.scale_x,
                entity.scale_y,
                entity.speed,
            );
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
            let after = (
                entity.x,
                entity.y,
                entity.rotation,
                entity.scale_x,
                entity.scale_y,
                entity.speed,
            );
            let needs_sync = invalid || before != after;
            if needs_sync {
                entity.sync_runtime_motion_to_components();
            }
            (true, invalid, needs_sync)
        };

        let (updated, sanitized, component_syncs) = if parallel {
            entities
                .par_iter_mut()
                .map(update)
                .map(|(updated, sanitized, synced)| {
                    (
                        usize::from(updated),
                        usize::from(sanitized),
                        usize::from(synced),
                    )
                })
                .reduce(
                    || (0, 0, 0),
                    |left, right| (left.0 + right.0, left.1 + right.1, left.2 + right.2),
                )
        } else {
            entities.iter_mut().map(update).fold(
                (0, 0, 0),
                |(updated, sanitized, synced), (did_update, did_sanitize, did_sync)| {
                    (
                        updated + usize::from(did_update),
                        sanitized + usize::from(did_sanitize),
                        synced + usize::from(did_sync),
                    )
                },
            )
        };
        MovementFrameReport {
            updated,
            skipped: entities.len().saturating_sub(updated),
            sanitized,
            component_syncs,
            parallel,
        }
    }
}

fn finite_or(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::MovementSystem;
    use crate::entities::game_object::GameObject;

    #[test]
    fn report_counts_active_skipped_and_sanitized_entities_without_frame_storage() {
        let mut moving = GameObject::new_unit(0.0, 0.0, Some("Moving".to_string()));
        moving.path = vec![(10.0, 0.0)];
        let mut invalid = GameObject::new_unit(f64::NAN, 0.0, Some("Invalid".to_string()));
        invalid.path = vec![(10.0, 0.0)];
        let mut disabled = GameObject::new_unit(0.0, 0.0, Some("Disabled".to_string()));
        disabled.enabled = false;
        let mut entities = vec![moving, invalid, disabled];

        let report = MovementSystem.update_entities_with_report(&mut entities, 1.0 / 60.0);

        assert_eq!(report.updated, 2);
        assert_eq!(report.skipped, 1);
        assert_eq!(report.sanitized, 1);
        assert_eq!(report.component_syncs, 2);
        assert!(!report.parallel);
        assert!(entities[1].x.is_finite());
    }

    #[test]
    fn large_scenes_use_parallel_reduction() {
        let mut entities = (0..256)
            .map(|index| GameObject::new(index as f64, 0.0, None))
            .collect::<Vec<_>>();

        let report = MovementSystem.update_entities_with_report(&mut entities, 0.0);

        assert_eq!(report.updated, 256);
        assert_eq!(report.component_syncs, 0);
        assert!(report.parallel);
    }
}
