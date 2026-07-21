use std::io;
use std::path::Path;

use serde_json::{Value, json};

use crate::engine::archetype_library::ArchetypeLibrary;
use crate::engine::asset_tools::AssetTools;
use crate::engine::component::default_component;
use crate::engine::prefab_manager::PrefabManager;
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
            "demo" | "complete_demo" | "playable_demo" => {
                Self::complete_playable_demo(project_path)
            }
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
            AssetTools::create_luau_script(&project_path, "RTSCameraController")?,
            AssetTools::create_luau_script(&project_path, "RTSUnitCommands")?,
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
            AssetTools::create_luau_script(&project_path, "PlayerController")?,
            AssetTools::create_luau_script(&project_path, "EnemyBrain")?,
            AssetTools::create_visual_graph(&project_path, "PlayerController")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "InputBindings")?,
            AssetTools::create_scene(&project_path, "TopDown_Level")?,
        ])
    }

    pub fn platformer(project_path: impl AsRef<Path>) -> io::Result<Vec<std::path::PathBuf>> {
        Ok(vec![
            AssetTools::create_luau_script(&project_path, "PlatformerMotor")?,
            AssetTools::create_luau_script(&project_path, "JumpController")?,
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
        let scene = AssetTools::create_scene(&project_path, "Survival_Map")?;
        AssetTools::write_json(&scene, &Self::survival_scene_data("Survival_Map"))?;
        let recipes = AssetTools::create_json(&project_path, Some(&paths.data), "CraftingRecipes")?;
        AssetTools::write_json(
            &recipes,
            &json!({
                "format": "MiniForgeCraftingRecipes",
                "recipes": [],
                "schema": {
                    "recipe": {"id": "string", "ingredients": "item stacks", "outputs": "item stacks"},
                    "item_stack": {"id": "string", "quantity": "integer", "metadata": "object"}
                }
            }),
        )?;
        let library = ArchetypeLibrary::with_defaults();
        let prefab_manager = PrefabManager::new(&project_path);
        let mut prefabs = Vec::new();
        for (key, filename) in [
            ("survival_actor", "SurvivalActor.prefab"),
            ("survival_loot_container", "LootContainer.prefab"),
            ("survival_harvestable", "Harvestable.prefab"),
            ("survival_crafting_station", "CraftingStation.prefab"),
        ] {
            let mut entity = library.instantiate(key, 0.0, 0.0, None).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("missing built-in archetype {key}"),
                )
            })?;
            prefabs.push(prefab_manager.save_prefab(&mut entity, Some(filename))?);
        }
        let mut created = vec![
            AssetTools::create_visual_graph(&project_path, "SurvivalPlayer")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "DayNightSystem")?,
            AssetTools::create_json(&project_path, Some(&paths.data), "SurvivalSettings")?,
        ];
        created.extend(prefabs);
        created.extend([
            recipes,
            AssetTools::create_json(&project_path, Some(&paths.data), "BiomeRules")?,
            scene,
        ]);
        Ok(created)
    }

    fn survival_scene_data(scene_name: &str) -> Value {
        let library = ArchetypeLibrary::with_defaults();
        let specs = [
            ("survival_actor", 4.0, 4.0),
            ("survival_loot_container", 7.0, 4.0),
            ("survival_harvestable", 10.0, 4.0),
            ("survival_crafting_station", 13.0, 4.0),
        ];
        let mut entities = specs
            .into_iter()
            .filter_map(|(key, x, y)| library.instantiate(key, x, y, None))
            .collect::<Vec<_>>();
        entities.extend(Self::survival_hud_entities());
        let serialized = entities
            .iter_mut()
            .map(GameObject::serialize)
            .collect::<Vec<_>>();
        let mut data = AssetTools::template_scene(scene_name);
        data["entities"] = Value::Array(serialized);
        data["settings"] = json!({
            "genre": "Survival",
            "starter_features": [
                "health",
                "survival_needs",
                "weighted_inventory",
                "searchable_loot",
                "data_driven_crafting",
                "harvestable_resources",
                "automatic_survival_hud"
            ],
            "contains_game_content": false
        });
        data
    }

    fn survival_hud_entities() -> Vec<GameObject> {
        [
            ("SurvivalHealth", "Health", "health", 24.0),
            ("SurvivalHunger", "Hunger", "hunger", 58.0),
            ("SurvivalThirst", "Thirst", "thirst", 92.0),
            ("SurvivalEnergy", "Energy", "energy", 126.0),
            ("SurvivalStamina", "Stamina", "stamina", 160.0),
        ]
        .into_iter()
        .map(|(name, label, source, y)| {
            let mut entity = GameObject::new(0.0, 0.0, Some(name.to_string()));
            entity.tag = "UI".to_string();
            entity.layer = "UI".to_string();
            entity.add_component(default_component("UIElement").expect("UIElement"));
            entity
                .add_component(default_component("SurvivalUIBinding").expect("SurvivalUIBinding"));
            if let Some(ui) = entity.get_component_mut("UIElement") {
                ui.set("element_type", json!("ProgressBar"));
                ui.set("text", json!(label));
                ui.set_f64("x", 24.0);
                ui.set_f64("y", y);
                ui.set_f64("width", 240.0);
                ui.set_f64("height", 26.0);
                ui.set_f64("progress", 100.0);
                ui.set_f64("max_progress", 100.0);
                ui.set("sorting_order", json!(100));
            }
            if let Some(binding) = entity.get_component_mut("SurvivalUIBinding") {
                binding.set("source", json!(source));
                binding.set("label", json!(label));
            }
            entity
        })
        .collect()
    }

    pub fn complete_playable_demo(
        project_path: impl AsRef<Path>,
    ) -> io::Result<Vec<std::path::PathBuf>> {
        let project_path = project_path.as_ref().to_path_buf();
        AssetTools::ensure_project_folders(&project_path)?;
        let paths = AssetTools::get_project_paths(&project_path);
        let mut created = Vec::new();

        let menu = AssetTools::create_scene(&project_path, "Demo_Menu")?;
        AssetTools::write_json(&menu, &Self::demo_menu_scene("Demo_Menu"))?;
        created.push(menu);

        let game = AssetTools::create_scene(&project_path, "Demo_Game")?;
        AssetTools::write_json(&game, &Self::demo_game_scene("Demo_Game"))?;
        created.push(game);

        created.push(AssetTools::create_luau_script(&project_path, "DemoPlayer")?);
        AssetTools::create_file(
            paths.scripts.join("DemoPlayer.luau"),
            r#"function on_start()
    ui_text("Commander ready")
end
function on_update(dt: number)
    if input_pressed("A") then move(-4.0 * dt, 0.0) end
    if input_pressed("D") then move(4.0 * dt, 0.0) end
    if input_pressed("W") then move(0.0, -4.0 * dt) end
    if input_pressed("S") then move(0.0, 4.0 * dt) end
end
function on_key_down(key: string)
    if key == "Space" then spawn("SparkBurst", entity_id, 0) play_sound("ui_confirm") end
    if key == "Escape" then load_scene("Demo_Menu.scene") end
end
function on_collision_enter(other: string)
    set_ui_text("HUD_Status", "Contact: " .. other)
end
function on_destroy()
    play_sound("unit_lost")
end
"#,
            true,
        )?;

        created.push(AssetTools::create_luau_script(&project_path, "DemoMenu")?);
        AssetTools::create_file(
            paths.scripts.join("DemoMenu.luau"),
            r#"function on_key_down(key: string)
    if key == "Enter" then load_scene("Demo_Game.scene") end
end
"#,
            true,
        )?;

        created.push(AssetTools::create_visual_graph(
            &project_path,
            "DemoButtonClick",
        )?);
        created.push(AssetTools::create_particle_preset(
            &project_path,
            "ImpactSparks",
        )?);
        created.push(AssetTools::create_shader(&project_path, "sprite_lit_fog")?);
        created.push(AssetTools::create_material(
            &project_path,
            "DemoLitMaterial",
        )?);
        created.push(AssetTools::create_audio_event(&project_path, "ui_confirm")?);
        created.push(AssetTools::create_audio_event(&project_path, "unit_lost")?);
        created.push(AssetTools::create_prefab(&project_path, "DemoWorker")?);
        created.push(AssetTools::create_prefab(&project_path, "DemoEnemy")?);
        AssetTools::write_json(
            paths.data.join("DemoSaveSlot.json"),
            &json!({
                "kind": "MiniForgeDemoSave",
                "checkpoint": "start",
                "resources": {"Gold": 500.0, "Wood": 250.0},
                "unlocked": ["worker", "soldier", "barracks"]
            }),
        )?;
        created.push(paths.data.join("DemoSaveSlot.json"));
        Ok(created)
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

    fn demo_menu_scene(scene_name: &str) -> Value {
        let mut title = GameObject::new(0.0, 0.0, Some("MenuTitle".to_string()));
        title.script = Some("DemoMenu.luau".to_string());
        title.add_component(default_component("UIElement").expect("UIElement"));
        if let Some(ui) = title.get_component_mut("UIElement") {
            ui.set("element_type", json!("Label"));
            ui.set("text", json!("MiniForge Complete Demo"));
            ui.set_f64("x", 80.0);
            ui.set_f64("y", 60.0);
            ui.set_f64("width", 420.0);
            ui.set_f64("height", 48.0);
        }
        let mut start = GameObject::new(0.0, 0.0, Some("StartButton".to_string()));
        start.add_component(default_component("UIElement").expect("UIElement"));
        if let Some(ui) = start.get_component_mut("UIElement") {
            ui.set("element_type", json!("Button"));
            ui.set("text", json!("Press Enter / Click Start"));
            ui.set("interactable", json!(true));
            ui.set("on_click", json!("Demo_Game.scene"));
            ui.set_f64("x", 96.0);
            ui.set_f64("y", 132.0);
            ui.set_f64("width", 260.0);
            ui.set_f64("height", 48.0);
        }
        let mut data = AssetTools::template_scene(scene_name);
        data["entities"] = json!([title.serialize(), start.serialize()]);
        data["ui_canvases"] =
            json!([crate::engine::ui_canvas::UiCanvasRoot::default_hud().to_value()]);
        data
    }

    fn demo_game_scene(scene_name: &str) -> Value {
        let mut entities = vec![
            Self::demo_player(),
            Self::rts_controller(1),
            Self::rts_building("CommandCenter", 8.0, 8.0, 1),
            Self::rts_worker("Worker_A", 10.0, 8.0, 1),
            Self::rts_worker("Worker_B", 10.0, 9.0, 1),
            Self::rts_resource("GoldNode_A", 13.0, 8.0),
            Self::rts_building("EnemyBase", 28.0, 20.0, 2),
            Self::rts_soldier("EnemyScout", 25.0, 19.0, 2),
            Self::demo_particles(),
            Self::demo_hud_status(),
        ];
        let serialized = entities
            .iter_mut()
            .map(GameObject::serialize)
            .collect::<Vec<_>>();
        let mut data = AssetTools::template_scene(scene_name);
        data["camera"] = json!({"x": 120.0, "y": 120.0, "zoom": 1.05});
        data["entities"] = Value::Array(serialized);
        data["settings"] = json!({
            "genre": "complete_demo",
            "shows": ["menus", "gameplay", "ui", "audio", "save_load", "rts", "scripting", "particles", "animations"]
        });
        data["ui_canvases"] =
            json!([crate::engine::ui_canvas::UiCanvasRoot::default_hud().to_value()]);
        data
    }

    fn demo_player() -> GameObject {
        let mut player = GameObject::new_unit(5.0, 5.0, Some("DemoPlayer".to_string()));
        player.tag = "Player".to_string();
        player.layer = "Units".to_string();
        player.script = Some("DemoPlayer.luau".to_string());
        player.add_component(default_component("Health").expect("Health"));
        player.add_component(default_component("Stats").expect("Stats"));
        player.add_component(default_component("Animator").expect("Animator"));
        player.add_component(default_component("ParticleEmitter").expect("ParticleEmitter"));
        player.add_component(default_component("Saveable").expect("Saveable"));
        player.add_component(default_component("Material2D").expect("Material2D"));
        if let Some(save) = player.get_component_mut("Saveable") {
            save.set("save_key", json!("demo_player"));
        }
        player.sync_to_components();
        player
    }

    fn demo_particles() -> GameObject {
        let mut particles = GameObject::new(6.0, 5.0, Some("ImpactSparksEmitter".to_string()));
        particles.add_component(default_component("ParticleEmitter").expect("ParticleEmitter"));
        if let Some(emitter) = particles.get_component_mut("ParticleEmitter") {
            emitter.set("looped", json!(true));
            emitter.set("rate", json!(24.0));
            emitter.set("burst_count", json!(12));
        }
        particles
    }

    fn demo_hud_status() -> GameObject {
        let mut hud = GameObject::new(0.0, 0.0, Some("HUD_Status".to_string()));
        hud.add_component(default_component("UIElement").expect("UIElement"));
        if let Some(ui) = hud.get_component_mut("UIElement") {
            ui.set("element_type", json!("Label"));
            ui.set("text", json!("Gather gold, build units, survive."));
            ui.set_f64("x", 24.0);
            ui.set_f64("y", 24.0);
            ui.set_f64("width", 360.0);
            ui.set_f64("height", 36.0);
        }
        hud
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
