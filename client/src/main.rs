#![allow(clippy::type_complexity)]

mod camera;
mod constants;
mod game;
mod input;
mod lighting;
mod menu;
mod network;
mod player;
mod splash_screen;
mod ui;
mod world;

use crate::world::ClientWorldMap;
use bevy::{
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use clap::Parser;
use constants::{TEXTURE_PATH_BASE, TEXTURE_PATH_CUSTOM};
use input::{data::GameAction, keyboard::get_bindings};
use menu::settings::{DisplayQuality, Volume};
use menu::solo::SelectedWorld;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Flag to use custom textures
    #[arg(long, help = "Use custom textures instead of base textures")]
    use_custom_textures: bool,
}

#[derive(Component)]
pub struct MenuCamera;

pub const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

// Enum that will be used as a global state for the game
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum GameState {
    Splash,
    #[default]
    Menu,
    PreGameLoading,
    Game,
}

#[derive(Event)]
pub struct LoadWorldEvent {
    pub world_name: String,
}

#[derive(Resource, Serialize, Deserialize)]
pub struct KeyMap {
    pub map: BTreeMap<GameAction, Vec<KeyCode>>,
}

#[derive(Resource, Debug)]
pub struct TexturePath {
    pub path: String,
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Determine which texture path to use
    let texture_path = if args.use_custom_textures {
        TEXTURE_PATH_CUSTOM
    } else {
        TEXTURE_PATH_BASE
    };

    debug!(
        "Using {} for textures",
        if args.use_custom_textures {
            "custom textures"
        } else {
            "base textures"
        }
    );

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            // Ensures that pixel-art textures will remain pixelated, and not become a blurry mess
            .set(ImagePlugin::default_nearest())
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARNING: This is a native-only feature. It will not work with WebGL or WebGPU
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../data".to_string(),
                ..Default::default()
            }),
    );
    app.add_plugins(EguiPlugin);
    app.add_plugins(WorldInspectorPlugin::new());
    app.add_event::<LoadWorldEvent>();
    network::add_base_netcode(&mut app);
    app.insert_resource(DisplayQuality::Medium)
        .insert_resource(Volume(7))
        .insert_resource(get_bindings())
        .insert_resource(SelectedWorld::default())
        // Declare the game state, whose starting value is determined by the `Default` trait
        .insert_resource(ClientWorldMap { ..default() })
        .insert_resource(TexturePath {
            path: texture_path.to_string(),
        })
        .init_state::<GameState>()
        .enable_state_scoped_entities::<GameState>()
        // Adds the plugins for each state
        .add_plugins((
            splash_screen::splash_plugin,
            menu::menu_plugin,
            game::game_plugin,
        ))
        .run();
}
