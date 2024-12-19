use bevy::prelude::*;

use crate::ui::assets::*;
use crate::ui::style::{background_image_style, big_button_style, text_style, NORMAL_BUTTON};
use crate::TEXT_COLOR;

use super::{MenuButtonAction, MenuState};

pub fn home_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load assets
    let background_image = load_background_image(&asset_server);
    let button_background_image = load_button_background_image(&asset_server);
    let title_image = load_title_image(&asset_server);
    let font = load_font(&asset_server);

    let button_text_style = text_style(font.clone(), 33.0, TEXT_COLOR);

    // Main container for the menu
    commands
        .spawn((
            (
                background_image_style(),
                BackgroundColor(Color::NONE),
                StateScoped(MenuState::Main),
            ),
            ImageNode::new(background_image), // Set the background image
        ))
        .with_children(|parent| {
            // Display the game title as an image
            let image_width = 800.0;
            let aspect_ratio = 601.0 / 94.0;
            let image_height = image_width / aspect_ratio;

            parent.spawn((
                Node {
                    margin: UiRect::bottom(Val::Px(120.0)), // Add space below the title image
                    width: Val::Px(image_width),
                    height: Val::Px(image_height),
                    ..default()
                },
                ImageNode::new(title_image),
            ));

            // Add buttons for each action available in the menu
            for (action, label) in [
                (MenuButtonAction::Solo, "Singleplayer"),
                (MenuButtonAction::Multi, "Multiplayer"),
                (MenuButtonAction::Settings, "Settings"),
                (MenuButtonAction::Quit, "Quit"),
            ] {
                parent
                    .spawn((
                        (
                            Button,
                            big_button_style(), // Use large button style
                            BackgroundColor(NORMAL_BUTTON.into()),
                            ImageNode::new(button_background_image.clone()),
                        ),
                        action,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(label, button_text_style.clone()));
                    });
            }
        });
}
