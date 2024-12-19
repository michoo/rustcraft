use crate::ui::assets::*;
use crate::ui::style::text_style;
use crate::{ui::assets::load_background_image, TEXT_COLOR};
use bevy::{
    asset::AssetServer,
    prelude::{BuildChildren, ButtonBundle, Commands, NodeBundle, Res, StateScoped, TextBundle},
    ui::{AlignItems, FlexDirection, JustifyContent, Node, UiImage, UiRect, Val},
};

use crate::menus::{MenuButtonAction, MenuState};

pub fn settings_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let background_image = load_background_image(&asset_server);
    let font = load_font(&asset_server);

    let button_style = Node {
        width: Val::Px(400.0),
        height: Val::Px(60.0),
        margin: UiRect::all(Val::Px(20.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..Default::default()
    };

    let button_text_style = text_style(font.clone(), 33.0, TEXT_COLOR);

    commands
        .spawn((
            NodeBundle {
                style: Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                ..Default::default()
            },
            UiImage::new(background_image),
            StateScoped(MenuState::Settings),
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    for (action, text) in [
                        (MenuButtonAction::SettingsControls, "Controls"),
                        (MenuButtonAction::BackToMainMenu, "Back"),
                    ] {
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: button_style.clone(),
                                    ..Default::default()
                                },
                                action,
                            ))
                            .with_children(|parent| {
                                parent.spawn(TextBundle::from_section(
                                    text,
                                    button_text_style.clone(),
                                ));
                            });
                    }
                });
        });
}
