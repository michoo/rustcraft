use crate::chat::{setup_chat_resources, ChatMessageEvent};
use crate::init::{ServerLobby, TickCounter};
use crate::player::handle_player_inputs;
use crate::time::update_server_time;
use crate::world::save::SaveRequestEvent;
use crate::world::BlockInteractionEvent;
use crate::world::WorldUpdateRequestEvent;
use crate::{chat, world};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer, ServerEvent};
use bincode::Options;
use shared::messages::{
    AuthRegisterResponse, ChatConversation, ClientToServerMessage, PlayerSpawnEvent,
    ServerToClientMessage,
};
use shared::world::ServerWorldMap;
use shared::GameServerConfig;

#[derive(Resource)]
pub struct BroadcastTimer {
    pub timer: Timer,
}

pub fn setup_resources_and_events(app: &mut App) {
    app.insert_resource(BroadcastTimer {
        timer: Timer::from_seconds(2.0, TimerMode::Repeating),
    })
    .add_event::<WorldUpdateRequestEvent>()
    .add_event::<SaveRequestEvent>()
    .add_event::<BlockInteractionEvent>();

    setup_chat_resources(app);
}

pub fn register_systems(app: &mut App) {
    app.add_systems(Update, server_update_system);

    app.add_systems(Update, chat::broadcast_chat_messages);

    app.add_systems(
        Update,
        (world::broadcast_world_state, world::send_world_update),
    );

    app.add_systems(Update, world::save::save_world_system);
    app.add_systems(Update, world::handle_block_interactions);

    app.add_systems(Update, update_server_time);
}

#[allow(clippy::type_complexity)]
fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    (mut server, mut chat_conversation, mut lobby, tick): (
        ResMut<RenetServer>,
        ResMut<ChatConversation>,
        ResMut<ServerLobby>,
        Res<TickCounter>,
    ),
    (
        mut ev_chat,
        mut ev_app_exit,
        mut ev_world_update_request,
        mut ev_save_request,
        mut ev_block_interaction,
    ): (
        EventWriter<ChatMessageEvent>,
        EventWriter<AppExit>,
        EventWriter<WorldUpdateRequestEvent>,
        EventWriter<SaveRequestEvent>,
        EventWriter<BlockInteractionEvent>,
    ),
    config: Res<GameServerConfig>,
    mut world_map: ResMut<ServerWorldMap>,
) {
    for event in server_events.read() {
        debug!("event received");
        match event {
            ServerEvent::ClientConnected { client_id } => {
                info!("Player {} connected.", client_id);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                info!("Player {} disconnected: {}", client_id, reason);
            }
        }
    }

    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let msg = bincode::options().deserialize::<ClientToServerMessage>(&message);
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to parse incoming message: {}", e);
                    continue;
                }
            };

            match msg {
                ClientToServerMessage::AuthRegisterRequest(auth_req) => {
                    info!("Auth request received {:?}", auth_req);

                    if lobby.players.values().any(|v| *v == auth_req.username) {
                        debug!("Username already in map: {}", &auth_req.username);
                        return;
                    }

                    // let new_session_token = generate_session_token();
                    lobby
                        .players
                        .insert(client_id.raw(), auth_req.username.clone());
                    debug!("New lobby : {:?}", lobby);

                    // Déterminer la position de spawn du joueur
                    let spawn_position = world_map
                        .player_positions
                        .get(&auth_req.username)
                        .cloned() // Utiliser la position sauvegardée si elle existe
                        .unwrap_or_else(|| Vec3::new(0.0, 80.0, 0.0)); // Sinon, une position par défaut

                    // Ajouter la position dans le ServerWorldMap
                    world_map
                        .player_positions
                        .entry(auth_req.username.clone())
                        .or_insert(spawn_position);

                    let spawn_message = PlayerSpawnEvent {
                        id: client_id.raw(),
                        name: auth_req.username,
                        position: spawn_position,
                    };

                    // TODO: add cleanup system if no heartbeat
                    let msg = &(AuthRegisterResponse {
                        username: spawn_message.name.clone(),
                        session_token: client_id.raw() as u128,
                        spawn_event: spawn_message.clone(),
                    });
                    let auth_response_payload = bincode::options().serialize(msg).unwrap();

                    server.send_message(
                        client_id,
                        DefaultChannel::ReliableOrdered,
                        auth_response_payload,
                    );

                    for (id, name) in lobby.players.iter() {
                        let spawn_message = PlayerSpawnEvent {
                            id: *id,
                            name: name.into(),
                            position: Vec3::new(0.0, 80.0, 0.0),
                        };

                        let spawn_message_wrapped =
                            &ServerToClientMessage::PlayerSpawn(spawn_message);

                        let spawn_payload = bincode::options()
                            .serialize(&spawn_message_wrapped.clone())
                            .unwrap();

                        server.broadcast_message(DefaultChannel::ReliableUnordered, spawn_payload);
                        info!("Sending spawn order {:?}", spawn_message_wrapped);
                    }
                }
                ClientToServerMessage::ChatMessage(chat_msg) => {
                    info!("Chat message received: {:?}", &chat_msg);
                    chat_conversation.messages.push(chat_msg);
                    ev_chat.send(ChatMessageEvent);
                }
                ClientToServerMessage::Exit(order) => {
                    debug!("Received shutdown order... {:?}", order);
                    // TODO: add permission checks
                    if config.is_solo {
                        info!("Server is going down...");
                        ev_app_exit.send(AppExit::Success);
                    } else {
                        server.disconnect(client_id);
                        lobby.players.remove(&(order.session_token as u64));
                        info!("Player {:?} disconnected", client_id);
                    }
                }
                ClientToServerMessage::PlayerInputs(inputs) => {
                    handle_player_inputs(inputs, &tick);
                }
                ClientToServerMessage::SaveWorldRequest(save_req) => {
                    debug!(
                        "Save request received from client with session token: {}",
                        save_req.session_token
                    );

                    ev_save_request.send(SaveRequestEvent);
                }
                ClientToServerMessage::WorldUpdateRequest {
                    player_chunk_position,
                    requested_chunks,
                    render_distance,
                } => {
                    debug!(
                        "Received WorldUpdateRequest: client_id = {}, player_chunk_position = {:?}, render_distance = {}, requested_chunks = {}",
                        client_id,
                        player_chunk_position,
                        render_distance,
                        requested_chunks.len(),
                    );
                    ev_world_update_request.send(WorldUpdateRequestEvent {
                        render_distance,
                        client: client_id,
                        chunks: requested_chunks,
                        player_chunk_position,
                    });
                }
                ClientToServerMessage::BlockInteraction {
                    position,
                    block_type,
                } => {
                    debug!(
                        "Block interaction received at {:?}: {:?}",
                        position, block_type
                    );

                    ev_block_interaction.send(BlockInteractionEvent {
                        position,
                        block_type,
                    });
                }
                ClientToServerMessage::SetPlayerPosition { position } => {
                    world_map.player_positions.insert(client_id.raw(), position);
                }
            }
        }
    }
}

// fn generate_session_token() -> u128 {
//     let random_value: u128 = random();
//     random_value
// }
