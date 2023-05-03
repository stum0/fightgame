use bevy::render::camera::ScalingMode;
use bevy::{prelude::*, window::Window};
use bevy_asset_loader::prelude::*;
use bevy_egui::egui::{Pos2, TextEdit};
use bevy_ggrs::ggrs::PlayerType;
use bevy_ggrs::{ggrs, GGRSPlugin, GGRSSchedule, RollbackIdProvider, Session};
use bevy_matchbox_nostr::prelude::*;
use bevy_mod_simplest_healthbar::{HealthBar, HealthBarPlugin};
use components::*;
use log::Level;
use nostr_sdk::prelude::{FromBech32, ToBech32};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{
    serde_json, Client, ClientMessage, EventBuilder, Filter, Keys, RelayPoolNotification, Tag,
    Timestamp,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
        // .register_rollback_component::<Bullet>()
        // .register_rollback_component::<Health>()
        .register_rollback_component::<Transform>()
        // .register_rollback_component::<Target>()
        // .register_rollback_component::<BulletReady>()
        // .register_rollback_component::<MoveDir>()
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
        .add_plugin(
            // Need to define which camera we are going to be spawning the stuff in relation to, as well as what is the "health" component
            HealthBarPlugin::<Health, BarCamera>::new("fonts/quicksand-light.ttf")
                // to automatically spawn bars on stuff with Health and a Transform
                .automatic_bar_creation(true),
        )
        .add_system(menu.run_if(in_state(GameState::Menu)))
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
                fire_bullets.after(move_system),
                reload_bullet.after(fire_bullets),
                move_bullet.after(fire_bullets),
                kill_players.after(move_bullet).after(move_system),
            )
                .in_schedule(GGRSSchedule),
        )
        .insert_resource(GameName {
            name: String::new(),
        })
        .insert_resource(GamesList(Arc::new(Mutex::new(Vec::new()))))
        .insert_resource(SearchGames { search: true })
        .run();
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    AssetLoading,
    Menu,
    Matchmaking,
    InGame,
}

#[derive(Resource)]
struct LocalPlayerHandle(usize);

#[derive(Resource, Default, Debug)]
pub struct GameName {
    pub name: String,
}

#[derive(Resource, Default, Debug)]
pub struct SearchGames {
    pub search: bool,
}

#[derive(Resource)]
pub struct GamesList(pub Arc<Mutex<Vec<Game>>>);

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Game {
    pub name: String,
    pub created_by: String,
}

impl Game {
    pub fn display_name(&self) -> String {
        format!("Game Name: {}, Created by: {}", self.name, self.created_by)
    }
}

fn create_nostr_key(mut commands: Commands) {
    let keys = Keys::generate();
    let relay = "wss://nostr.lu.ke".to_string();
    //let relay = "ws://localhost:8080".to_string();
    commands.spawn(Nostr { relay, keys });
}

fn menu(
    mut contexts: EguiContexts,
    window: Query<&Window>,
    mut next_state: ResMut<NextState<GameState>>,
    nostr_query: Query<&Nostr>,
    games_list: Res<GamesList>,
    mut game_name: ResMut<GameName>,
    mut search_games: ResMut<SearchGames>,
) {
    let nostr = nostr_query.iter().next().unwrap();
    let nostr_keys = nostr.keys.clone();
    let relay = nostr.relay.clone();

    let window = window.iter().next().unwrap();
    let screen_size = egui::Vec2::new(window.width(), window.height());
    let screen_center = screen_size / 2.0;
    let pos = Pos2::new(screen_center.x, screen_center.y / 2.0);

    egui::Window::new("web21")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_pos(pos)
        .show(contexts.ctx_mut(), |ui| {
            ui.add(
                TextEdit::singleline(&mut game_name.name)
                    .hint_text("Enter game name")
                    .id(egui::Id::new("game_name_input")),
            );

            if ui.small_button("Create Game").clicked() && !game_name.name.is_empty() {
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

            if search_games.search {
                let games_handle = games_list.0.clone();
                info!("connecting to nostr relay: {:?}", relay);
                spawn_local(async move {
                    let tag = "matchbox-nostr-v1";

                    let client = Client::new(&nostr_keys);
                    #[cfg(target_arch = "wasm32")]
                    client.add_relay(&relay).await.unwrap();

                    client.connect().await;
                    //send sub message

                    let subscription = Filter::new()
                        .since(Timestamp::now() - Duration::from_secs(60))
                        .hashtag(tag);

                    client.subscribe(vec![subscription]).await;

                    client
                        .handle_notifications(move |notification| {
                            let games = games_handle.clone();
                            async move {
                                if let RelayPoolNotification::Event(_url, event) = notification {
                                    info!("{:?}", event.content);
                                    let game_name: String = serde_json::from_str(&event.content)
                                        .expect("deserializing request");

                                    let mut games_lock = games.lock().unwrap();

                                    let game = Game {
                                        name: game_name,
                                        created_by: event.pubkey.to_bech32().unwrap(),
                                    };
                                    games_lock.push(game);
                                }
                                Ok(())
                            }
                        })
                        .await
                        .unwrap();
                });
                search_games.search = false;
            }

            let games_lock = games_list.0.lock().unwrap();
            ui.separator();
            ui.label("searching for games...");
            ui.separator();
            if games_lock.is_empty() {
                ui.label("No games found, please wait or create a game.");
            }

            for game in games_lock.iter() {
                let list_game = format!("GAME NAME: {} CREATED BY: {}", game.name, game.created_by);
                if ui.button(list_game).clicked() {
                    //send nostr dm with peer id to game creator
                    let reciever = XOnlyPublicKey::from_bech32(game.clone().created_by).unwrap();

                    let nostr_keys = nostr.keys.clone();
                    let relay = nostr.relay.clone();

                    info!("connecting to nostr relay: {:?}", relay);

                    //list game
                    spawn_local(async move {
                        let pub_key = PeerId(nostr_keys.public_key());
                        let new_peer = PeerEvent::NewPeer(pub_key);
                        let new_peer =
                            serde_json::to_string(&new_peer).expect("serializing request");

                        let client = Client::new(&nostr_keys);
                        #[cfg(target_arch = "wasm32")]
                        client.add_relay(&relay).await.unwrap();

                        client.connect().await;
                        client.send_direct_msg(reciever, new_peer).await.unwrap();
                        client.disconnect().await.unwrap();
                    });
                    next_state.set(GameState::Matchmaking);
                }
            }
        });
}

// fn action_bar(
//     mut contexts: EguiContexts,
//     mut player_query: Query<(&Transform, &Player, &mut Health)>,
//     window: Query<&Window>,
// ) {
//     let window = window.iter().next().unwrap();
//     let width = window.width();
//     let height = window.height();

//     for (transform, player, health) in player_query.iter_mut() {
//         let location = transform.translation.to_;
//         info!("location: {:?}", location);
//         let bevy_x = location.x;
//         let bevy_y = location.y;
//         let (egui_x, egui_y) = bevy_to_egui_coordinates(bevy_x, bevy_y, width, height);

//         let label_position = egui::Pos2::new(egui_x, egui_y);
//         info!("pos2: {:?}", label_position);
//         egui::CentralPanel::default()
//             .frame(egui::Frame::none())
//             .show(contexts.ctx_mut(), |ui| {
//                 if player.handle == 0 {
//                     let mut p1 = format!("P1: {}", health.0);
//                     ui.put(
//                         egui::Rect {
//                             min: label_position,
//                             max: label_position + egui::Vec2::new(100.0, 100.0),
//                         },
//                         TextEdit::singleline(&mut p1),
//                     );
//                     ui.label(p1);
//                 } else {
//                     let p2 = format!("P2: {}", health.0);
//                     ui.label(p2);
//                 }
//             });
//     }
// }

// fn bevy_to_egui_coordinates(bevy_x: f32, bevy_y: f32, width: f32, height: f32) -> (f32, f32) {
//     let egui_x = bevy_x + (width / 2.0);
//     let egui_y = (height / 2.0) - bevy_y;
//     (egui_x, egui_y)
// }

fn spawn_players(
    mut commands: Commands,
    mut rip: ResMut<RollbackIdProvider>,
    images: Res<ImageAssets>,
) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn((camera_bundle, BarCamera));

    let p1_rotation = Quat::from_rotation_y(std::f32::consts::PI);

    //player 1
    let p1_position = Vec2::new(-5.0, 0.0);
    commands.spawn((
        Player {
            handle: 0,
            moving: false,
        },
        MoveDir(Vec2::X),
        BulletReady { ready: true },
        Target::default(),
        rip.next(),
        Health {
            current: 99,
            max: 99,
        },
        HealthBar {
            offset: Vec2::new(0., 30.),
            size: 20.,
            color: Color::GREEN,
        },
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: images.player_1.clone(),
            transform: Transform::from_xyz(p1_position.x, p1_position.y, 0.0)
                .with_rotation(p1_rotation),
            ..Default::default()
        },
    ));
    //player 2
    let p2_position = Vec2::new(5.0, 0.0);
    commands.spawn((
        Player {
            handle: 1,
            moving: false,
        },
        MoveDir(-Vec2::X),
        BulletReady { ready: true },
        Target::default(),
        rip.next(),
        Health {
            current: 99,
            max: 99,
        },
        HealthBar {
            offset: Vec2::new(0., 30.),
            size: 20.,
            color: Color::GREEN,
        },
        SpriteBundle {
            sprite: Sprite {
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
    mut contexts: EguiContexts,
    window: Query<&Window>,
) {
    let window = window.iter().next().unwrap();
    let screen_size = egui::Vec2::new(window.width(), window.height());
    let screen_center = screen_size / 2.0;
    let pos = Pos2::new(screen_center.x, screen_center.y / 2.0);

    egui::Window::new("web21")
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .fixed_pos(pos)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Waiting for players...");
        });
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
            let frame = s.frames_ahead();

            info!("GGRS FRAME: {:?}", frame);
        }
        _ => panic!("This example focuses on p2p."),
    }
}
