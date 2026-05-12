import os

from engine.asset_tools import AssetTools


class ProjectTemplates:
    """
    Plantillas de proyecto para arrancar juegos sin editar el motor.
    """

    @staticmethod
    def create(game, template_name):
        template = str(template_name or "Empty").lower()

        AssetTools.ensure_project_folders(game.project_path)

        if template == "empty":
            return ProjectTemplates.empty(game)

        if template == "rts":
            return ProjectTemplates.rts(game)

        if template == "topdown":
            return ProjectTemplates.topdown(game)

        if template == "platformer":
            return ProjectTemplates.platformer(game)

        if template in ("actionrpg", "action_rpg", "action rpg"):
            return ProjectTemplates.action_rpg(game)

        if template == "survival":
            return ProjectTemplates.survival(game)

        game.console.log(f"Plantilla desconocida: {template_name}", "WARNING")
        return False

    @staticmethod
    def empty(game):
        scene = AssetTools.create_scene(game.project_path, "EmptyScene")
        game.console.log(f"Plantilla Empty creada: {scene}", "ENGINE")
        return True

    @staticmethod
    def rts(game):
        AssetTools.create_script(game.project_path, "CameraController")
        AssetTools.create_system(game.project_path, "EconomySystem")
        AssetTools.create_prefab(game.project_path, "Worker")
        AssetTools.create_scene(game.project_path, "RTS_Map")
        game.console.log("Plantilla RTS creada", "ENGINE")
        return True

    @staticmethod
    def topdown(game):
        script = AssetTools.create_script(game.project_path, "PlayerController")
        data_folder = AssetTools.get_project_paths(game.project_path)["data"]
        AssetTools.create_json(game.project_path, data_folder, "InputBindings")
        AssetTools.create_scene(game.project_path, "TopDown_Level")
        game.console.log(f"Plantilla TopDown creada: {os.path.basename(script)}", "ENGINE")
        return True

    @staticmethod
    def platformer(game):
        AssetTools.create_component(game.project_path, "PlatformerMotor")
        AssetTools.create_script(game.project_path, "JumpController")
        AssetTools.create_scene(game.project_path, "Platformer_Level")
        game.console.log("Plantilla Platformer creada", "ENGINE")
        return True

    @staticmethod
    def action_rpg(game):
        data_folder = AssetTools.get_project_paths(game.project_path)["data"]
        AssetTools.create_script(game.project_path, "PlayerCombat")
        AssetTools.create_script(game.project_path, "EnemyBrain")
        AssetTools.create_system(game.project_path, "QuestRuntime")
        AssetTools.create_system(game.project_path, "LootRuntime")
        AssetTools.create_prefab(game.project_path, "Player")
        AssetTools.create_prefab(game.project_path, "Enemy")
        AssetTools.create_prefab(game.project_path, "QuestNPC")
        AssetTools.create_json(game.project_path, data_folder, "Items")
        AssetTools.create_json(game.project_path, data_folder, "Quests")
        AssetTools.create_scene(game.project_path, "ActionRPG_Level")
        game.console.log("Plantilla ActionRPG creada", "ENGINE")
        return True

    @staticmethod
    def survival(game):
        data_folder = AssetTools.get_project_paths(game.project_path)["data"]
        AssetTools.create_script(game.project_path, "SurvivalPlayer")
        AssetTools.create_system(game.project_path, "DayNightSystem")
        AssetTools.create_system(game.project_path, "CraftingSystem")
        AssetTools.create_prefab(game.project_path, "ResourceNode")
        AssetTools.create_prefab(game.project_path, "Campfire")
        AssetTools.create_json(game.project_path, data_folder, "CraftingRecipes")
        AssetTools.create_json(game.project_path, data_folder, "BiomeRules")
        AssetTools.create_scene(game.project_path, "Survival_Map")
        game.console.log("Plantilla Survival creada", "ENGINE")
        return True
