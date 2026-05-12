from engine.component import ADVANCED_COMPONENT_TYPES


class EngineUpgradeManifest:
    """
    Machine-readable manifest for the 100+ engine upgrade batch.
    """

    COMPONENT_INTEGRATIONS = (
        "advanced component",
        "scene and prefab serialization",
        "editor registry entry",
        "inspector primitive field editing",
    )

    RUNTIME_UPGRADES = (
        "GameplaySystem runtime loop",
        "Lifetime expiration",
        "Cooldown ticking",
        "Timer completion events",
        "Status effect damage over time",
        "Status effect healing over time",
        "Stats health regeneration",
        "StateMachine timed transitions",
        "Blackboard-driven transitions",
        "Tween property sampling",
        "NavAgent destination movement",
        "NavAgent path rebuilding",
        "Interaction proximity activation",
        "Interaction input execution",
        "CharacterController2D movement",
        "CharacterController2D jump impulses",
        "Spawner interval spawning",
        "Spawner max-alive tracking",
        "AI target acquisition",
        "AI chase behavior",
        "AI attack behavior",
        "AI wander behavior",
        "DamageDealer cooldown tracking",
        "Health damage integration",
        "CameraFollow smoothing",
        "CameraFollow zoom control",
        "Gameplay profiler counters",
        "Runtime event emission for damage",
        "Runtime event emission for interactions",
        "Runtime event emission for timers",
    )

    API_UPGRADES = (
        "api.entities",
        "api.find_with_component",
        "api.get_component",
        "api.add_component",
        "api.remove_component",
        "api.create_unit",
        "api.destroy_by_id",
        "api.set_position",
        "api.translate",
        "api.move_to",
        "api.query_radius",
        "api.nearest",
        "api.damage",
        "api.heal",
        "api.health",
        "api.add_item",
        "api.remove_item",
        "api.item_count",
        "api.add_resource",
        "api.spend_resource",
        "api.set_blackboard",
        "api.get_blackboard",
        "api.start_cooldown",
        "api.cooldown_ready",
        "api.add_status_effect",
        "api.tween",
        "api.add_quest",
        "api.complete_quest",
        "api.on",
        "api.emit",
        "api.save_game_state",
        "api.load_game_state",
    )

    EDITOR_UPGRADES = (
        "dynamic advanced component menu",
        "TopDown Player preset",
        "Platformer Player preset",
        "Enemy AI preset",
        "Quest NPC preset",
        "Pickup Item preset",
        "Combat Projectile preset",
        "Spawner Enemy preset",
        "Checkpoint preset",
        "Interactable Door preset",
        "inspector quick TopDown preset button",
        "inspector quick Enemy preset button",
        "inspector quick NPC preset button",
        "inspector quick Projectile preset button",
        "inspector shows more components per entity",
        "UIElement padding",
        "UIElement border radius",
        "UIElement border color",
        "UIElement text alignment",
        "UIElement font size",
        "UIElement progress value",
        "UIElement max progress",
        "ProgressBar rendering",
        "stretch_width UI anchor",
        "stretch_height UI anchor",
        "stretch UI anchor",
        "UI focus helpers",
        "expanded input actions for abilities",
        "expanded input actions for inventory",
        "expanded input actions for running and secondary attack",
        "ActionRPG project template",
        "Survival project template",
        "template actions in navigator",
        "UI menu actions",
        "Visual scripting menu actions",
        "Plugin menu actions",
        "Asset dependency menu actions",
        "Command palette beta tools",
        "Scene View mouse gizmo dragging",
    )

    ASSET_PIPELINE_UPGRADES = (
        "persistent asset GUID metadata",
        "per-asset import settings",
        "sprite import filter toggle",
        "audio streaming toggle",
        "include_in_build toggle",
        "asset dependency graph",
        "reverse dependency lookup",
        "build report dependency listing",
        "build report import settings listing",
        "export respects include_in_build",
    )

    PLUGIN_UPGRADES = (
        "plugin.py hook loading",
        "on_editor_start hook",
        "on_scene_saved hook",
        "on_asset_imported hook",
        "plugin hook console command",
    )

    VALIDATION_UPGRADES = (
        "advanced numeric ranges",
        "advanced enum validation",
        "Saveable duplicate-key warnings",
        "AI target-id warnings",
        "NavAgent destination warnings",
        "Inventory capacity warnings",
        "Spawner capacity warnings",
        "Tween property errors",
    )

    VISUAL_SCRIPT_UPGRADES = (
        "AddComponent node",
        "SetBlackboard node",
        "EmitEvent node",
        "Damage node",
        "Heal node",
        "AddItem node",
        "StartCooldown node",
        "SetState node",
        "Tween node",
    )

    def all(self):
        upgrades = []

        for component_name in sorted(ADVANCED_COMPONENT_TYPES.keys()):
            for integration in self.COMPONENT_INTEGRATIONS:
                upgrades.append(f"{component_name}: {integration}")

        upgrades.extend(self.RUNTIME_UPGRADES)
        upgrades.extend(self.API_UPGRADES)
        upgrades.extend(self.EDITOR_UPGRADES)
        upgrades.extend(self.ASSET_PIPELINE_UPGRADES)
        upgrades.extend(self.PLUGIN_UPGRADES)
        upgrades.extend(self.VALIDATION_UPGRADES)
        upgrades.extend(self.VISUAL_SCRIPT_UPGRADES)
        return upgrades

    def count(self):
        return len(self.all())

    def summary(self):
        return {
            "count": self.count(),
            "advanced_components": len(ADVANCED_COMPONENT_TYPES),
            "runtime": len(self.RUNTIME_UPGRADES),
            "api": len(self.API_UPGRADES),
            "editor": len(self.EDITOR_UPGRADES),
            "asset_pipeline": len(self.ASSET_PIPELINE_UPGRADES),
            "plugins": len(self.PLUGIN_UPGRADES),
            "validation": len(self.VALIDATION_UPGRADES),
            "visual_scripting": len(self.VISUAL_SCRIPT_UPGRADES),
        }
