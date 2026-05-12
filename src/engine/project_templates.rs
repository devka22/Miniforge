use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::engine::asset_tools::AssetTools;
use crate::engine::component::default_component;
use crate::entities::game_object::GameObject;

pub struct ProjectTemplates;

impl ProjectTemplates {
    pub fn create(
        project_path: impl AsRef<Path>,
        template_name: &str,
    ) -> io::Result<Vec<std::path::PathBuf>> {
        AssetTools::ensure_project_folders(&project_path)?;
        let template = template_name.to_lowercase().replace([' ', '-'], "_");
        match template.as_str() {
            "empty" => Self::empty(project_path),
            "rts" => Self::rts(project_path),
            "topdown" | "top_down" => Self::topdown(project_path),
            "platformer" => Self::platformer(project_path),
            "actionrpg" | "action_rpg" => Self::action_rpg(project_path),
            "survival" => Self::survival(project_path),
            _ => Self::empty(project_path),
        }
    }

    pub fn empty(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        Ok(vec![AssetTools::create_scene(project_path, "EmptyScene")?])
    }

    pub fn rts(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        let paths = AssetTools::get_project_paths(&project_path);
        let scene = AssetTools::create_scene(&project_path, "RTS_Map")?;
        AssetTools::write_json(&scene, &Self::rts_scene_data("RTS_Map"))?;
        Ok(vec![
            AssetTools::create_visual_graph(&project_path, "CameraController")?,
            AssetTools::create_visual_graph(&project_path, "SelectionMarquee")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "EconomySystem")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "ProductionSystem")?,
            AssetTools::create_prefab(&project_path, "Worker")?,
            AssetTools::create_prefab(&project_path, "Soldier")?,
            AssetTools::create_prefab(&project_path, "CommandCenter")?,
            AssetTools::create_prefab(&project_path, "Barracks")?,
            AssetTools::create_prefab(&project_path, "ResourceNode")?,
            AssetTools::create_json_file(
                paths.data.join("RTSBalance.json"),
                &json!({
                    "resources": ["Gold", "Wood", "Supply"],
                    "units": {
                        "Worker": {"build_time": 3.0, "cost": {"Gold": 50.0}},
                        "Soldier": {"build_time": 5.0, "cost": {"Gold": 80.0, "Wood": 20.0}}
                    },
                    "buildings": {
                        "CommandCenter": {"build_time": 10.0, "cost": {"Gold": 350.0, "Wood": 150.0}},
                        "Barracks": {"build_time": 8.0, "cost": {"Gold": 150.0, "Wood": 80.0}}
                    }
                }),
                true,
            )?,
            scene,
        ])
    }

    pub fn topdown(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        let paths = AssetTools::get_project_paths(&project_path);
        Ok(vec![
            AssetTools::create_visual_graph(&project_path, "PlayerController")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "InputBindings")?,
            AssetTools::create_scene(&project_path, "TopDown_Level")?,
        ])
    }

    pub fn platformer(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        Ok(vec![
            AssetTools::create_visual_graph(&project_path, "PlatformerMotor")?,
            AssetTools::create_visual_graph(&project_path, "JumpController")?,
            AssetTools::create_scene(&project_path, "Platformer_Level")?,
        ])
    }

    pub fn action_rpg(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        let paths = AssetTools::get_project_paths(&project_path);
        Ok(vec![
            AssetTools::create_visual_graph(&project_path, "PlayerCombat")?,
            AssetTools::create_visual_graph(&project_path, "EnemyBrain")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "QuestRuntime")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "LootRuntime")?,
            AssetTools::create_prefab(&project_path, "Player")?,
            AssetTools::create_prefab(&project_path, "Enemy")?,
            AssetTools::create_prefab(&project_path, "QuestNPC")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "Items")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "Quests")?,
            AssetTools::create_scene(&project_path, "ActionRPG_Level")?,
        ])
    }

    pub fn survival(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        let paths = AssetTools::get_project_paths(&project_path);
        Ok(vec![
            AssetTools::create_visual_graph(&project_path, "SurvivalPlayer")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "DayNightSystem")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "CraftingSystem")?,
            AssetTools::create_prefab(&project_path, "ResourceNode")?,
            AssetTools::create_prefab(&project_path, "Campfire")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "CraftingRecipes")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "BiomeRules")?,
            AssetTools::create_scene(&project_path, "Survival_Map")?,
        ])
    }

    fn rts_scene_data(scene_name: &str) -> Value {
        let mut entities = vec![
            Self::rts_controller(1),
            Self::rts_controller(2),
            Self::rts_building("CommandCenter", 8.0, 8.0, 1),
            Self::rts_worker("Worker_A", 10.0, 8.0, 1),
            Self::rts_worker("Worker_B", 10.0, 9.0, 1),
            Self::rts_resource("GoldNode_A", 13.0, 8.0),
            Self::rts_resource("GoldNode_B", 15.0, 10.0),
            Self::rts_building("EnemyBase", 30.0, 22.0, 2),
            Self::rts_soldier("EnemyScout", 27.0, 21.0, 2),
        ];
        let serialized = entities
            .iter_mut()
            .map(GameObject::serialize)
            .collect::<Vec<_>>();
        let mut data = AssetTools::template_scene(scene_name);
        data["camera"] = json!({"x": 120.0, "y": 100.0, "zoom": 1.1});
        data["entities"] = Value::Array(serialized);
        data["settings"] = json!({
            "genre": "RTS",
            "starter_features": ["economy", "production", "fog_of_war", "formation_move"]
        });
        data
    }

    fn rts_controller(team_id: i64) -> GameObject {
        let mut entity = GameObject::new(0.0, 0.0, Some(format!("RTSController_Team{team_id}")));
        entity.visible = false;
        entity.locked = true;
        entity.layer = "EditorOnly".to_string();
        entity.add_component(default_component("RTSController").expect("RTSController"));
        entity.add_component(default_component("FogOfWar").expect("FogOfWar"));
        entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        Self::apply_team(&mut entity, team_id);
        if let Some(controller) = entity.get_component_mut("RTSController") {
            controller.set("team_id", json!(team_id));
        }
        if let Some(fog) = entity.get_component_mut("FogOfWar") {
            fog.set("team_id", json!(team_id));
        }
        if let Some(wallet) = entity.get_component_mut("EconomyWallet") {
            wallet.set(
                "resources",
                json!({"Gold": 500.0, "Wood": 250.0, "Supply": 0.0}),
            );
        }
        entity.sync_to_components();
        entity
    }

    fn rts_building(name: &str, x: f64, y: f64, team_id: i64) -> GameObject {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.tag = if team_id == 1 { "Player" } else { "Enemy" }.to_string();
        entity.layer = "Buildings".to_string();
        entity.width = 2.4;
        entity.height = 2.0;
        entity.radius = 1.2;
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("EconomyWallet").expect("EconomyWallet"));
        entity.add_component(default_component("ProductionQueue").expect("ProductionQueue"));
        entity.add_component(default_component("Buildable").expect("Buildable"));
        entity.add_component(default_component("Commandable").expect("Commandable"));
        entity.add_component(default_component("Vision").expect("Vision"));
        Self::apply_team(&mut entity, team_id);
        if let Some(queue) = entity.get_component_mut("ProductionQueue") {
            queue.set_f64("rally_x", x + if team_id == 1 { 3.0 } else { -3.0 });
            queue.set_f64("rally_y", y);
        }
        if let Some(commandable) = entity.get_component_mut("Commandable") {
            commandable.set("can_move", json!(false));
            commandable.set("can_produce", json!(true));
        }
        entity.sync_to_components();
        entity
    }

    fn rts_worker(name: &str, x: f64, y: f64, team_id: i64) -> GameObject {
        let mut entity = GameObject::new_unit(x, y, Some(name.to_string()));
        entity.tag = if team_id == 1 { "Player" } else { "Enemy" }.to_string();
        entity.layer = "Units".to_string();
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("Stats").expect("Stats"));
        entity.add_component(default_component("Inventory").expect("Inventory"));
        entity.add_component(default_component("NavAgent").expect("NavAgent"));
        entity.add_component(default_component("Worker").expect("Worker"));
        entity.add_component(default_component("Commandable").expect("Commandable"));
        entity.add_component(default_component("Vision").expect("Vision"));
        Self::apply_team(&mut entity, team_id);
        if let Some(commandable) = entity.get_component_mut("Commandable") {
            commandable.set("can_gather", json!(true));
            commandable.set("can_build", json!(true));
        }
        entity.sync_to_components();
        entity
    }

    fn rts_soldier(name: &str, x: f64, y: f64, team_id: i64) -> GameObject {
        let mut entity = GameObject::new_unit(x, y, Some(name.to_string()));
        entity.tag = if team_id == 1 { "Player" } else { "Enemy" }.to_string();
        entity.layer = "Units".to_string();
        entity.add_component(default_component("Health").expect("Health"));
        entity.add_component(default_component("Stats").expect("Stats"));
        entity.add_component(default_component("NavAgent").expect("NavAgent"));
        entity.add_component(default_component("DamageDealer").expect("DamageDealer"));
        entity.add_component(default_component("CombatTarget").expect("CombatTarget"));
        entity.add_component(default_component("Commandable").expect("Commandable"));
        entity.add_component(default_component("Vision").expect("Vision"));
        Self::apply_team(&mut entity, team_id);
        entity.sync_to_components();
        entity
    }

    fn rts_resource(name: &str, x: f64, y: f64) -> GameObject {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.tag = "Resource".to_string();
        entity.layer = "Ground".to_string();
        entity.width = 1.3;
        entity.height = 1.3;
        entity.add_component(default_component("ResourceNode").expect("ResourceNode"));
        entity.add_component(default_component("ObjectiveMarker").expect("ObjectiveMarker"));
        entity.sync_to_components();
        entity
    }

    fn apply_team(entity: &mut GameObject, team_id: i64) {
        entity.add_component(default_component("Team").expect("Team"));
        if let Some(team) = entity.get_component_mut("Team") {
            team.set("team_id", json!(team_id));
            team.set(
                "team_name",
                json!(match team_id {
                    1 => "Player",
                    2 => "Enemy",
                    _ => "Neutral",
                }),
            );
            team.set(
                "color",
                json!(match team_id {
                    1 => [80, 160, 255],
                    2 => [255, 95, 95],
                    _ => [160, 160, 160],
                }),
            );
        }
    }
}
