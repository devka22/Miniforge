use crate::engine::asset_database::AssetDatabase;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default)]
pub struct AssetReferenceResolver;

impl AssetReferenceResolver {
    pub fn resolve_entity(entity: &mut GameObject, database: &AssetDatabase) {
        if entity.sprite_guid.is_some() {
            return;
        }
        if let Some(sprite_name) = &entity.sprite_name {
            for record in database.assets.values() {
                if record.name == *sprite_name {
                    entity.sprite_guid = Some(record.guid.clone());
                    entity.sync_to_components();
                    break;
                }
            }
        }
    }
}
