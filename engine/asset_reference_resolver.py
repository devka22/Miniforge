import os


class AssetReferenceResolver:
    """
    Resuelve referencias estables por GUID con fallback por nombre/path.
    """

    def __init__(self, game):
        self.game = game

    def asset_by_guid(self, guid):
        if not guid or not hasattr(self.game, "asset_database"):
            return None

        return self.game.asset_database.find_by_id(guid)

    def sprite_name(self, sprite_guid=None, fallback_name=None):
        asset = self.asset_by_guid(sprite_guid)

        if asset and asset.get("type") == "Sprite":
            return asset.get("name")

        return fallback_name

    def prefab_path(self, prefab_guid=None, fallback_path=None):
        asset = self.asset_by_guid(prefab_guid)

        if asset and asset.get("type") == "Prefab":
            return asset.get("path")

        if fallback_path and os.path.exists(fallback_path):
            return fallback_path

        return fallback_path

    def attach_sprite_reference(self, entity, asset):
        if not asset:
            return

        entity.sprite_name = asset.get("name")
        entity.sprite_guid = asset.get("id")

        sprite_renderer = (
            entity.get_component("SpriteRenderer")
            if hasattr(entity, "get_component")
            else None
        )

        if sprite_renderer:
            sprite_renderer.sprite_name = asset.get("name")
            sprite_renderer.sprite_guid = asset.get("id")
