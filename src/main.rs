use bevy::{prelude::*, render::camera::ScalingMode, window::Window};
use bevy_asset_loader::prelude::*;
use bevy_ggrs::ggrs::PlayerType;
use bevy_ggrs::{ggrs, GGRSPlugin, GGRSSchedule, RollbackIdProvider, Session};
use bevy_matchbox_nostr::prelude::*;
use bevy_mod_simplest_healthbar::{HealthBar, HealthBarPlugin};
use components::*;
use log::Level;
use nostr_sdk::{serde_json, Client, ClientMessage, EventBuilder, Keys, Tag};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
mod components;
use spells::*;
mod spells;
use input::*;
mod input;

pub fn main() {
    console_log::init_with_level(Level::Warn).expect("error initializing log");

    let mut app = App::new();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .register_rollback_component::<Transform>()
        .register_rollback_component::<Target>()
        .register_rollback_component::<BulletReady>()
        .register_rollback_component::<MoveDir>()
        .register_rollback_component::<Health>()
        .build(&mut app);

    app.add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::AssetLoading).continue_to_state(GameState::Matchmaking),
        )
        .add_collection_to_loading_state::<_, ImageAssets>(GameState::AssetLoading)
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
        .add_plugin(
            // Need to define which camera we are going to be spawning the stuff in relation to, as well as what is the "health" component
            HealthBarPlugin::<Health, BarCamera>::new("fonts/quicksand-light.ttf")
                // to automatically spawn bars on stuff with Health and a Transform
                .automatic_bar_creation(true),
        )
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_systems(
            (lobby_startup, start_matchbox_socket).in_schedule(OnEnter(GameState::Matchmaking)),
        )
        .add_systems((
            wait_for_players.run_if(
                resource_exists::<MatchboxSocket<SingleChannel>>()
                    .and_then(in_state(GameState::Matchmaking)),
            ),
            lobby_cleanup.in_schedule(OnExit(GameState::Matchmaking)),
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
        .run();
}

#[derive(States, Clone, Eq, PartialEq, Debug, Hash, Default)]
enum GameState {
    #[default]
    AssetLoading,
    Matchmaking,
    InGame,
}

#[derive(Resource)]
struct LocalPlayerHandle(usize);

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

fn lobby_startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn((camera_bundle, BarCamera));

    // All this is just for spawning centered text.
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            background_color: Color::rgb(0.43, 0.41, 0.38).into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: Style {
                        align_self: AlignSelf::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    text: Text::from_section(
                        "Entering lobby...",
                        TextStyle {
                            font: asset_server.load("fonts/quicksand-light.ttf"),
                            font_size: 30.,
                            color: Color::BLACK,
                        },
                    ),
                    ..default()
                })
                .insert(LobbyText);
        })
        .insert(LobbyUI);
}

fn lobby_cleanup(query: Query<Entity, With<LobbyUI>>, mut commands: Commands) {
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn spawn_players(
    mut commands: Commands,
    mut rip: ResMut<RollbackIdProvider>,
    images: Res<ImageAssets>,
) {
    //player 1
    let p1_position = Vec2::new(-5.0, 0.0);
    commands.spawn((
        Player {
            facing_right: true,
            handle: 0,
            moving: false,
        },
        MoveDir(Vec2::X),
        BulletReady {
            ready: true,
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        },
        Target::default(),
        rip.next(),
        Health { current: 6, max: 6 },
        // Create custom size and color and offset for the "bar"
        HealthBar {
            offset: Vec2::new(0., 30.),
            size: 20.,
            color: Color::GREEN,
        },
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
        BulletReady {
            ready: true,
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        },
        Target::default(),
        rip.next(),
        Health { current: 6, max: 6 },
        // Create custom size and color and offset for the "bar"
        HealthBar {
            offset: Vec2::new(0., 30.),
            size: 20.,
            color: Color::GREEN,
        },
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

fn start_matchbox_socket(mut commands: Commands) {
    // let room_url = "ws://localhost:8080";
    let relay = "wss://nostr.lu.ke";

    //let room_url = "ws://127.0.0.1:5000/nostrclient/api/v1/relay";
    let nostr_keys = Keys::generate();
    let nostr_keys_clone = nostr_keys.clone();

    info!("connecting to nostr relay: {:?}", relay);

    //list game
    spawn_local(async move {
        let pub_key = PeerId(nostr_keys_clone.public_key());
        let tag = "matchbox-nostr-1";
        let new_peer = PeerEvent::NewPeer(pub_key);
        let new_peer = serde_json::to_string(&new_peer).expect("serializing request");

        let broadcast_peer = ClientMessage::new_event(
            EventBuilder::new_text_note(new_peer, &[Tag::Hashtag(tag.to_string())])
                .to_event(&nostr_keys_clone)
                .unwrap(),
        );

        warn!("BROADCAST PEER ID {:?}", broadcast_peer);

        let client = Client::new(&nostr_keys_clone);
        #[cfg(target_arch = "wasm32")]
        client.add_relay(relay).await.unwrap();

        client.connect().await;
        client.send_msg(broadcast_peer).await.unwrap();
        client.disconnect().await.unwrap();
    });

    commands.open_socket(
        WebRtcSocketBuilder::new(relay, nostr_keys).add_channel(ChannelConfig::ggrs()),
    );
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut query: Query<&mut Text, With<LobbyText>>,
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
    query.single_mut().sections[0].value = format!("Waiting for {remaining} more player(s)",);
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
