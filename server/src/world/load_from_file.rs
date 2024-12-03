use bevy::prelude::*;
use ron::de::from_str;
use shared::world::data::{ServerWorldMap, WorldSeed};
use shared::world::get_game_folder;
use shared::GameFolderPaths;
use std::fs;
use std::path::Path;
use std::collections::HashMap;


use crate::world::data::SAVE_PATH;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WorldData {
    pub time: u64,
    pub player_positions: HashMap<String, Vec3>,
    pub seed: WorldSeed,
    pub map: ServerWorldMap,
}

/// Charge les données combinées (carte et graine) d'un fichier
pub fn load_world_data(
    file_name: &str,
    app: &App,
) -> Result<WorldData, Box<dyn std::error::Error>> {
    // Obtenir le chemin du dossier de jeu
    let game_folder_path = app.world().get_resource::<GameFolderPaths>().unwrap();

    // Construire le chemin complet du fichier
    let file_path: PathBuf = get_game_folder(Some(&game_folder_path))
        .join(SAVE_PATH)
        .join(format!("{file_name}.ron"));
    let path: &Path = file_path.as_path();

    // Vérifier si le fichier existe
    if !path.exists() {
        info!(
            "World data file not found: {}. Generating default world and seed.",
            file_path.display()
        );
        return Ok(WorldData {
            map: ServerWorldMap {
                name: file_name.to_string(),
                ..Default::default()
            },
            seed: WorldSeed(rand::random::<u32>()),
            time: 0,
            player_positions: HashMap::new(),
        });
    }

    // Lire le contenu du fichier
    let contents: String = fs::read_to_string(path)?;
    let world_data: WorldData = from_str(&contents)?; // Désérialiser les données combinées
    Ok(world_data)
}

pub fn load_world_map(
    file_name: &str,
    app: &App,
) -> Result<ServerWorldMap, Box<dyn std::error::Error>> {
    let world_data = load_world_data(file_name, app)?;
    Ok(world_data.map)
}

pub fn load_world_time(file_name: &str, app: &App) -> Result<u64, Box<dyn std::error::Error>> {
    let world_data = load_world_data(file_name, app)?;
    Ok(world_data.time)
}

pub fn load_world_seed(
    file_name: &str,
    app: &App,
) -> Result<WorldSeed, Box<dyn std::error::Error>> {
    let world_data = load_world_data(file_name, app)?;
    Ok(world_data.seed)
}
