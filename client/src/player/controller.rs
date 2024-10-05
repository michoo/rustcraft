use crate::camera::CameraController;
use crate::constants::GRAVITY;
use crate::input::keyboard::*;
use crate::player::{Player, ViewMode};
use crate::ui::UIMode;
use crate::world::RenderDistance;
use crate::world::{load_chunk_around_player, WorldMap, WorldRenderRequestUpdateEvent, WorldSeed};
use crate::KeyMap;
use bevy::prelude::*;

fn is_block_at_position(position: Vec3, world_map: &WorldMap) -> bool {
    world_map
        .get_block_by_coordinates(&IVec3::new(
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        ))
        .is_some()
}

fn check_player_collision(player_position: Vec3, player: &Player, world_map: &WorldMap) -> bool {
    // Vérification de la collision avec les pieds et la tête du joueur
    let foot_position = Vec3::new(
        player_position.x,
        player_position.y - player.height / 2.0,
        player_position.z,
    );
    let head_position = Vec3::new(
        player_position.x,
        player_position.y + player.height / 2.0,
        player_position.z,
    );

    // On vérifie les coins du joueur
    let offsets = [
        Vec3::new(-player.width / 2.0, 0.0, -player.width / 2.0), // bas gauche devant
        Vec3::new(player.width / 2.0, 0.0, -player.width / 2.0),  // bas droite devant
        Vec3::new(-player.width / 2.0, 0.0, player.width / 2.0),  // bas gauche derrière
        Vec3::new(player.width / 2.0, 0.0, player.width / 2.0),   // bas droite derrière
    ];

    // Vérifier la collision au niveau des pieds
    for offset in &offsets {
        let check_pos = foot_position + *offset;
        if is_block_at_position(check_pos, world_map) {
            return true;
        }
    }

    // Vérifier la collision au niveau de la tête
    for offset in &offsets {
        let check_pos = head_position + *offset;
        if is_block_at_position(check_pos, world_map) {
            return true;
        }
    }

    false
}

// System to move the player based on keyboard input
pub fn player_movement_system(
    queries: (
        Query<(&mut Transform, &mut Player, &mut Handle<StandardMaterial>)>,
        Query<&Transform, (With<Camera>, With<CameraController>, Without<Player>)>,
    ),
    resources: (
        Res<Time>,
        Res<ButtonInput<KeyCode>>,
        Res<WorldSeed>,
        Res<RenderDistance>,
        Res<UIMode>,
        Res<KeyMap>,
        ResMut<Assets<StandardMaterial>>,
        ResMut<WorldMap>,
    ),
    mut ev_render: EventWriter<WorldRenderRequestUpdateEvent>,
) {
    let (mut player_query, camera_query) = queries;
    let (
        time,
        keyboard_input,
        world_seed,
        render_distance,
        ui_mode,
        key_map,
        mut materials,
        mut world_map,
    ) = resources;

    let (mut player_transform, mut player, material_handle_mut_ref) = player_query.single_mut();
    let camera_transform = camera_query.single();

    if *ui_mode == UIMode::Closed {
        if is_action_just_pressed(GameAction::ToggleViewMode, &keyboard_input, &key_map) {
            player.toggle_view_mode();
        }

        if is_action_just_pressed(GameAction::ToggleChunkDebugMode, &keyboard_input, &key_map) {
            player.toggle_chunk_debug_mode();
        }

        // fly mode (f key)
        if is_action_just_pressed(GameAction::ToggleFlyMode, &keyboard_input, &key_map) {
            player.toggle_fly_mode();
        }
    }

    load_chunk_around_player(
        player_transform.translation,
        &mut world_map,
        world_seed.0,
        &mut ev_render,
        render_distance,
    );

    let material_handle = &*material_handle_mut_ref;
    match player.view_mode {
        ViewMode::FirstPerson => {
            // make player transparent
            if let Some(material) = materials.get_mut(material_handle) {
                material.base_color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            }
        }
        ViewMode::ThirdPerson => {
            if let Some(material) = materials.get_mut(material_handle) {
                material.base_color = Color::srgba(1.0, 0.0, 0.0, 1.0);
            }
        }
    }

    let speed = if player.is_flying { 15.0 } else { 5.0 };

    let jump_velocity = 10.0;

    // flying mode
    if player.is_flying && *ui_mode == UIMode::Closed {
        if is_action_pressed(GameAction::FlyUp, &keyboard_input, &key_map) {
            player_transform.translation.y += speed * 2.0 * time.delta_seconds();
        }
        if is_action_pressed(GameAction::FlyDown, &keyboard_input, &key_map) {
            player_transform.translation.y -= speed * 2.0 * time.delta_seconds();
        }
    }

    // Calculate movement directions relative to the camera
    let mut forward = camera_transform.forward().xyz();
    forward.y = 0.0;

    let mut right = camera_transform.right().xyz();
    right.y = 0.0;

    let mut direction = Vec3::ZERO;

    if *ui_mode == UIMode::Closed {
        // Adjust direction based on key presses
        if is_action_pressed(GameAction::MoveBackward, &keyboard_input, &key_map) {
            direction -= forward;
        }
        if is_action_pressed(GameAction::MoveForward, &keyboard_input, &key_map) {
            direction += forward;
        }
        if is_action_pressed(GameAction::MoveLeft, &keyboard_input, &key_map) {
            direction -= right;
        }
        if is_action_pressed(GameAction::MoveRight, &keyboard_input, &key_map) {
            direction += right;
        }
    }

    // Move the player (xy plane only), only if there are no blocks and UI is closed
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();

        // Déplacement sur l'axe X
        let new_pos_x = player_transform.translation
            + Vec3::new(direction.x, 0.0, 0.0) * speed * time.delta_seconds();

        if player.is_flying || !check_player_collision(new_pos_x, &player, &world_map) {
            player_transform.translation.x = new_pos_x.x;
        }

        // Déplacement sur l'axe Z
        let new_pos_z = player_transform.translation
            + Vec3::new(0.0, 0.0, direction.z) * speed * time.delta_seconds();

        if player.is_flying || !check_player_collision(new_pos_z, &player, &world_map) {
            player_transform.translation.z = new_pos_z.z;
        }
    }

    // Handle jumping (if on the ground) and gravity, only if not flying
    if !player.is_flying {
        if player.on_ground && is_action_pressed(GameAction::Jump, &keyboard_input, &key_map) {
            // Player can jump only when grounded
            player.vertical_velocity = jump_velocity;
            player.on_ground = false;
        } else if !player.on_ground {
            // Apply gravity when the player is in the air
            player.vertical_velocity += GRAVITY * time.delta_seconds();
        }
    }

    // apply gravity and verify vertical collisions
    let new_y = player_transform.translation.y + player.vertical_velocity * time.delta_seconds();

    // Vérifier uniquement les collisions verticales (sol et plafond)
    if check_player_collision(
        Vec3::new(
            player_transform.translation.x,
            new_y,
            player_transform.translation.z,
        ),
        &player,
        &world_map,
    ) {
        // Si un bloc est détecté sous le joueur, il reste sur le bloc
        player.on_ground = true;
        player.vertical_velocity = 0.0; // Réinitialiser la vélocité verticale si le joueur est au sol
    } else {
        // Si aucun bloc n'est détecté sous le joueur, il continue de tomber
        player_transform.translation.y = new_y;
        player.on_ground = false;
    }

    // If the player is below the world, reset their position
    const FALL_LIMIT: f32 = -50.0;
    if player_transform.translation.y < FALL_LIMIT {
        player_transform.translation = Vec3::new(0.0, 100.0, 0.0);
        player.vertical_velocity = 0.0;
    }
}
