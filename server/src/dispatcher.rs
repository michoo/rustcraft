use crate::chat::{setup_chat_resources, ChatMessageEvent};
use crate::init::{ServerLobby, TickCounter};
use crate::player::handle_player_inputs;
use crate::world::generation::setup_world;
use crate::world::save::SaveRequestEvent;
use crate::world::WorldUpdateRequestEvent;
use crate::{chat, world};
use bevy::prelude::*;
use bevy_renet::renet::{DefaultChannel, RenetServer, ServerEvent};
use bincode::Options;
use rand::random;
use shared::messages::{AuthRegisterResponse, ChatConversation, ClientToServerMessage};
use shared::world::ServerWorldMap;

#[derive(Resource)]
pub struct BroadcastTimer {
    pub timer: Timer,
}

pub fn setup_resources_and_events(app: &mut App) {
    app.insert_resource(BroadcastTimer {
        timer: Timer::from_seconds(2.0, TimerMode::Repeating),
    })
    .add_event::<WorldUpdateRequestEvent>()
    .add_event::<SaveRequestEvent>();

    setup_chat_resources(app);
}

pub fn register_systems(app: &mut App) {
    app.insert_resource(ServerWorldMap { ..default() });
    app.add_systems(Startup, setup_world);

    app.add_systems(Update, server_update_system);

    app.add_systems(Update, chat::broadcast_chat_messages);

    app.add_systems(
        Update,
        (world::broadcast_world_state, world::send_world_update),
    );

    app.add_systems(Update, world::save::save_world_system);
}

fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    resources: (
        ResMut<RenetServer>,
        ResMut<ChatConversation>,
        ResMut<ServerLobby>,
        Res<TickCounter>,
    ),
    event_writers: (
        EventWriter<ChatMessageEvent>,
        EventWriter<AppExit>,
        EventWriter<WorldUpdateRequestEvent>,
        EventWriter<SaveRequestEvent>,
    ),
) {
    let (mut server, mut chat_conversation, mut lobby, tick) = resources;
    let (mut ev_chat, mut ev_app_exit, mut ev_world_update_request, mut ev_save_request) =
        event_writers;

    for event in server_events.read() {
        println!("event received");
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Player {} connected.", client_id);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Player {} disconnected: {}", client_id, reason);
            }
        }
    }

    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            //println!("msg received {:?}", &message);

            let msg = bincode::options().deserialize::<ClientToServerMessage>(&message);
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => {
                    println!("Failed to parse incoming message: {}", e);
                    continue;
                }
            };

            match msg {
                ClientToServerMessage::AuthRegisterRequest(auth_req) => {
                    println!("Auth request received {:?}", auth_req);

                    if lobby.players.values().any(|v| *v == auth_req.username) {
                        println!("Username already in map: {}", &auth_req.username);
                        return;
                    }

                    let new_session_token = generate_session_token();
                    lobby
                        .players
                        .insert(new_session_token, auth_req.username.clone());
                    println!("New lobby : {:?}", lobby);
                    // TODO: add cleanup system if no heartbeat
                    let msg = &AuthRegisterResponse {
                        username: auth_req.username,
                        session_token: new_session_token,
                    };
                    let payload = bincode::options().serialize(msg).unwrap();
                    server.send_message(client_id, DefaultChannel::ReliableOrdered, payload);
                }
                ClientToServerMessage::ChatMessage(chat_msg) => {
                    println!("Chat message received: {:?}", &chat_msg);
                    chat_conversation.messages.push(chat_msg);
                    ev_chat.send(ChatMessageEvent);
                }
                ClientToServerMessage::ShutdownOrder(order) => {
                    println!("Received shutdown order... {:?}", order);
                    // TODO: add permission checks
                    println!("Server is going down...");
                    ev_app_exit.send(AppExit::Success);
                }
                ClientToServerMessage::PlayerInputs(inputs) => {
                    handle_player_inputs(inputs, &tick);
                }
                ClientToServerMessage::SaveWorldRequest(save_req) => {
                    println!(
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
                    ev_world_update_request.send(WorldUpdateRequestEvent {
                        render_distance,
                        client: client_id,
                        chunks: requested_chunks,
                        player_chunk_position,
                    });
                }
            }
        }
    }
}

fn generate_session_token() -> u128 {
    let random_value: u128 = random();
    random_value
}
