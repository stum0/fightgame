use std::sync::{Arc, Mutex};

use bevy::render::camera::ScalingMode;
use bevy::{prelude::*, window::Window};
use bevy_asset_loader::prelude::*;
use bevy_egui::egui::{Pos2, TextEdit};
use bevy_ggrs::ggrs::PlayerType;
use bevy_ggrs::{ggrs, GGRSPlugin, GGRSSchedule, RollbackIdProvider, Session};
use bevy_matchbox_nostr::prelude::*;
use components::*;
use log::Level;
use nostr_sdk::{
    serde_json, Client, ClientMessage, EventBuilder, Filter, Keys, RelayPoolNotification, Tag,
    Timestamp,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
mod components;
use spells::*;
mod spells;
use input::*;
mod input;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

pub fn main() {
    console_log::init_with_level(Level::Warn).expect("error initializing log");

    let mut app = App::new();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .register_rollback_component::<Transform>()
        .register_rollback_component::<Target>()
        .register_rollback_component::<BulletReady>()
        .register_rollback_component::<MoveDir>()
        .build(&mut app);

    app.add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::Menu),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameState::AssetLoading)
        .add_system(create_nostr_key.in_schedule(OnEnter(GameState::AssetLoading)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "SwordGame".to_string(),
                // fill the entire browser window
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(EguiPlugin)
        .add_system(menu.run_if(in_state(GameState::Menu)))
        .add_system(find_game.in_schedule(OnEnter(GameState::FindGameMenu)))
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_system(start_matchbox_socket.in_schedule(OnEnter(GameState::Matchmaking)))
        .add_systems((
            wait_for_players.run_if(
                resource_exists::<MatchboxSocket<SingleChannel>>()
                    .and_then(in_state(GameState::Matchmaking)),
            ),
            spawn_players.in_schedule(OnEnter(GameState::InGame)),
        ))
        .add_system(log_ggrs_events.in_set(OnUpdate(GameState::InGame)))
        .add_systems(
            (
                move_system,
                reload_bullet.after(move_system),
                fire_bullets.after(move_system).after(reload_bullet),
                move_bullet.after(fire_bullets),
                kill_players.after(move_bullet),
                // respawn_players.after(kill_players),
            )
                .in_schedule(GGRSSchedule),
        )
        .add_system(update_facing)
        .insert_resource(GameName {
            name: String::new(),
        })
        .run();
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    AssetLoading,
    Menu,
    FindGameMenu,
    Matchmaking,
    InGame,
}

#[derive(Resource)]
struct LocalPlayerHandle(usize);

#[derive(Resource, Default, Debug)]
pub struct GameName {
    pub name: String,
}

#[derive(AssetCollection, Resource)]
pub struct ImageAssets {
    #[asset(path = "eggbullet.png")]
    bullet: Handle<Image>,
    #[asset(path = "ostrich.png")]
    player_1: Handle<Image>,
    #[asset(path = "red_ostrich.png")]
    player_2: Handle<Image>,
}

#[derive(Debug)]
pub struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = CustomInput;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are called `PeerId`s
    type Address = PeerId;
}
fn create_nostr_key(mut commands: Commands) {
    let keys = Keys::generate();
    let relay = "wss://nostr.lu.ke".to_string();
    commands.spawn(Nostr { relay, keys });
}

fn menu(
    mut contexts: EguiContexts,
    window: Query<&Window>,
    mut next_state: ResMut<NextState<GameState>>,
    nostr_query: Query<&Nostr>,
    mut game_name: ResMut<GameName>,
) {
    let nostr = nostr_query.iter().next().unwrap();
    let window = window.iter().next().unwrap();

    let screen_size = egui::Vec2::new(window.width(), window.height());

    let screen_center = screen_size / 2.0;

    let pos = Pos2::new(screen_center.x, screen_center.y / 2.0);

    egui::Window::new("cool game name")
        .resizable(false)
        .collapsible(false)
        .fixed_pos(pos)
        .show(contexts.ctx_mut(), |ui| {
            ui.add(
                TextEdit::singleline(&mut game_name.name)
                    .hint_text("Enter game name")
                    .id(egui::Id::new("game_name_input")),
            );

            if ui.button("Create Game").clicked() {
                //let room_url = "ws://127.0.0.1:5000/nostrclient/api/v1/relay";
                let game_name = game_name.name.clone();
                let nostr_keys = nostr.keys.clone();
                let relay = nostr.relay.clone();

                info!("connecting to nostr relay: {:?}", relay);

                //list game
                spawn_local(async move {
                    let tag = "matchbox-nostr-v1";
                    let new_game = serde_json::to_string(&game_name).expect("serializing request");

                    let broadcast_peer = ClientMessage::new_event(
                        EventBuilder::new_text_note(new_game, &[Tag::Hashtag(tag.to_string())])
                            .to_event(&nostr_keys)
                            .unwrap(),
                    );

                    warn!("LIST GAME {:?}", broadcast_peer);

                    let client = Client::new(&nostr_keys);
                    #[cfg(target_arch = "wasm32")]
                    client.add_relay(&relay).await.unwrap();

                    client.connect().await;
                    client.send_msg(broadcast_peer).await.unwrap();
                    client.disconnect().await.unwrap();
                });
                next_state.set(GameState::Matchmaking);
            }
            if ui.button("Find Game").clicked() {
                next_state.set(GameState::FindGameMenu);
            };
        });
}

fn find_game(nostr_query: Query<&Nostr>) {
    let nostr = nostr_query.iter().next().unwrap();
    let nostr_keys = nostr.keys.clone();
    let relay = nostr.relay.clone();

    info!("connecting to nostr relay: {:?}", relay);
    spawn_local(async move {
        let tag = "matchbox-nostr-v1";

        info!("LOOKING FOR GAMES");

        let client = Client::new(&nostr_keys);
        #[cfg(target_arch = "wasm32")]
        client.add_relay(&relay).await.unwrap();

        client.connect().await;
        //send sub message

        let subscription = Filter::new().since(Timestamp::now()).hashtag(tag);

        client.subscribe(vec![subscription]).await;

        client
            .handle_notifications(|notification| async {
                if let RelayPoolNotification::Event(_url, event) = notification {
                    info!("{:?}", event.content);
                    let game: String =
                        serde_json::from_str(&event.content).expect("deserializing request");

                    info!("GAME FOUND {:?}", game);
                }
                Ok(())
            })
            .await
            .unwrap();
    });
}

fn spawn_players(
    mut commands: Commands,
    mut rip: ResMut<RollbackIdProvider>,
    images: Res<ImageAssets>,
) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    //player 1
    let p1_position = Vec2::new(-5.0, 0.0);
    commands.spawn((
        Player {
            facing_right: true,
            handle: 0,
            moving: false,
        },
        MoveDir(Vec2::X),
        BulletReady(true),
        Target::default(),
        rip.next(),
        // Create custom size and color and offset for the "bar"
        SpriteBundle {
            sprite: Sprite {
                //color: Color::rgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: images.player_1.clone(),
            transform: Transform::from_xyz(p1_position.x, p1_position.y, 0.0),
            ..Default::default()
        },
    ));
    //player 2
    let p2_position = Vec2::new(5.0, 0.0);
    commands.spawn((
        Player {
            facing_right: false,
            handle: 1,
            moving: false,
        },
        MoveDir(-Vec2::X),
        BulletReady(true),
        Target::default(),
        rip.next(),
        SpriteBundle {
            sprite: Sprite {
                //color: Color::rgb(1., 0.47, 0.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: images.player_2.clone(),
            transform: Transform::from_xyz(p2_position.x, p2_position.y, 0.0),
            ..Default::default()
        },
    ));
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerEvent {
    /// Sent by the server to the connecting peer, immediately after connection
    /// before any other events
    NewPeer(PeerId),
}

fn start_matchbox_socket(mut commands: Commands, nostr_query: Query<&Nostr>) {
    // let room_url = "ws://localhost:8080";
    let nostr = nostr_query.iter().next().unwrap();

    warn!("connecting to nostr relay: {:?}", nostr.relay);
    warn!("pubkey: {:?}", nostr.keys.public_key());
    commands.open_socket(
        WebRtcSocketBuilder::new(nostr.relay.to_owned(), nostr.keys.clone())
            .add_channel(ChannelConfig::ggrs()),
    );
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // regularly call update_peers to update the list of connected peers
    for (peer, new_state) in socket.update_peers() {
        // you can also handle the specific dis(connections) as they occur:
        match new_state {
            PeerState::Connected => info!("peer {peer:?} connected"),
            PeerState::Disconnected => info!("peer {peer:?} disconnected"),
        }
    }

    let connected_peers = socket.connected_peers().count();
    let remaining = 2 - (connected_peers + 1);

    if remaining > 0 {
        return;
    }

    info!("All peers have joined, going in-game");
    let players = socket.players();

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(2)
        .with_input_delay(2);

    for (i, player) in players.into_iter().enumerate() {
        if player == PlayerType::Local {
            info!("adding local player: {:?}", i);
            commands.insert_resource(LocalPlayerHandle(i));
        }

        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }
    info!("ggrs session started: {:?}", session_builder);
    // move the channel out of the socket (required because GGRS takes ownership of it)
    let socket = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(socket)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2PSession(ggrs_session));
    next_state.set(GameState::InGame);
}

// fn respawn_players(
//     mut commands: Commands,
//     player_query: Query<(Entity, &Player), (With<Despawned>, Without<Bullet>)>,
// ) {
//     for (entity, player) in player_query.iter() {
//         let position = match player.handle {
//             0 => Vec2::new(-5.0, 0.0),
//             1 => Vec2::new(5.0, 0.0),
//             _ => unreachable!(),
//         };

//         commands.entity(entity).remove::<Despawned>();
//         commands
//             .entity(entity)
//             .insert(Transform::from_xyz(position.x, position.y, 0.0));
//     }
// }

fn log_ggrs_events(mut session: ResMut<Session<GgrsConfig>>) {
    match session.as_mut() {
        Session::P2PSession(s) => {
            for event in s.events() {
                info!("GGRS Event: {:?}", event);
            }
        }
        _ => panic!("This example focuses on p2p."),
    }
}
