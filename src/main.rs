use bevy::{prelude::*, render::camera::ScalingMode, window::Window};
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;

const INPUT_MOVE: u8 = 1 << 0;
const INPUT_FIRE: u8 = 1 << 1;

fn main() {
    // let mut app = App::new();

    // app.add_state::<AppState>();

    // GGRSPlugin::<GgrsConfig>::new()
    //     .with_input_system(input)
    //     .register_rollback_component::<Transform>()
    //     .build(&mut app);

    // app.insert_resource(ClearColor(Color::rgb_u8(0, 0, 0)));

    // app.add_plugins(DefaultPlugins.set(WindowPlugin {
    //     primary_window: Some(Window {
    //         title: "SwordGame".to_string(),
    //         fit_canvas_to_parent: true,
    //         prevent_default_event_handling: false,
    //         ..Default::default()
    //     }),
    //     ..Default::default()
    // }));

    // app.add_startup_systems((setup, start_matchbox_socket));

    // app.add_systems((
    //     move_to_click.in_schedule(GGRSSchedule),
    //     update_facing,
    //     wait_for_players,
    // ));
    // app.run();

    let mut app = App::new();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .register_rollback_component::<Transform>()
        .register_rollback_component::<Target>()
        .build(&mut app);

    app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
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
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_startup_systems((setup, spawn_players, start_matchbox_socket))
        .add_systems((
            move_to_click.in_schedule(GGRSSchedule),
            wait_for_players,
            update_facing,
        ))
        .run();
}

// #[derive(States, PartialEq, Eq, Debug, Clone, Hash, Default)]
// enum AppState {
//     #[default]
//     InGame,
// }

#[derive(Component)]
pub struct Player {
    pub facing_right: bool,
    handle: usize,
    moving: bool,
}

#[derive(Component, Reflect, Default)]
pub struct Target(pub Vec2);

struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = u8;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are called `PeerId`s
    type Address = PeerId;
}

fn setup(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);
}

fn spawn_players(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rip: ResMut<RollbackIdProvider>,
) {
    //player 1
    let p1_position = Vec2::new(-5.0, 0.0);
    commands.spawn((
        Player {
            facing_right: true,
            handle: 0,
            moving: false,
        },
        Target(p1_position),
        rip.next(),
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: asset_server.load("bevy_pixel_dark.png"),
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
        Target(p2_position),
        rip.next(),
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0., 0.4, 0.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: asset_server.load("bevy_pixel_dark.png"),
            transform: Transform::from_xyz(p2_position.x, p2_position.y, 0.0),
            ..Default::default()
        },
    ));
}

// pub fn movement(
//     mut windows: Query<&mut Window>,
//     mut player_query: Query<(&mut Player, &mut Transform)>,
//     touches: Res<Touches>,
//     mouse: Res<Input<MouseButton>>,
//     camera_query: Query<(&Camera, &GlobalTransform)>,
// ) {
//     let (camera, camera_transform) = camera_query.single();

//     for touch in touches.iter_just_pressed() {
//         let touch_pos = touch.position();
//         for (mut player, _) in player_query.iter_mut() {
//             info!(
//                 "player pos {:?}, player: {:?}",
//                 player.position, player.handle
//             );
//             for window in windows.iter_mut() {
//                 let world_position =
//                     window_to_world_coordinates_touch(&window, camera, camera_transform, touch_pos);
//                 player.target_position = world_position;
//             }
//         }
//     }

//     for (mut player, _) in player_query.iter_mut() {
//         info!(
//             "player pos {:?}, player: {:?}",
//             player.position, player.handle
//         );
//         for window in windows.iter_mut() {
//             if let Some(cursor) = window.cursor_position() {
//                 if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
//                     let world_position =
//                         window_to_world_coordinates(&window, camera, camera_transform, cursor);
//                     player.target_position = world_position;
//                 }
//             }
//         }
//     }
// }

fn window_to_world_coordinates(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_position: Vec2,
) -> Vec2 {
    let screen_size = Vec2::new(window.width(), window.height());
    let screen_position = cursor_position / screen_size;
    let clip_position = (screen_position - Vec2::new(0.5, 0.5)) * 2.0;
    let mut world_position = camera
        .projection_matrix()
        .inverse()
        .project_point3(clip_position.extend(0.0));
    world_position = *camera_transform * world_position;
    world_position.truncate()
}

fn window_to_world_coordinates_touch(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_position: Vec2,
) -> Vec2 {
    let screen_size = Vec2::new(window.width(), window.height());
    let screen_position = Vec2::new(
        cursor_position.x / screen_size.x,
        1.0 - (cursor_position.y / screen_size.y),
    );
    let clip_position = (screen_position - Vec2::new(0.5, 0.5)) * 2.0;
    let mut world_position = camera
        .projection_matrix()
        .inverse()
        .project_point3(clip_position.extend(0.0));
    world_position = *camera_transform * world_position;
    world_position.truncate()
}

fn move_to_click(
    mut player_query: Query<(&mut Transform, &mut Target, &mut Player)>,
    time: Res<Time>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut windows: Query<&mut Window>,
    touches: Res<Touches>,
    mouse: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    for (mut transform, mut target, mut player) in player_query.iter_mut() {
        let (input, _) = inputs[player.handle];
        let (camera, camera_transform) = camera_query.single();

        let mut direction = Vec2::ZERO;

        if input & INPUT_MOVE != 0 {
            for window in windows.iter_mut() {
                if let Some(cursor) = window.cursor_position() {
                    let world_position =
                        window_to_world_coordinates(&window, camera, camera_transform, cursor);
                    target.0 = world_position;

                    player.moving = true;
                }
            }

            // let current_position = transform.translation;
            // direction = player.target_position - Vec2::new(current_position.x, current_position.y);
            // let distance_to_target = direction.length();
            // if distance_to_target > 0.0 {
            //     let player_speed = 10.0;
            //     let normalized_direction = direction / distance_to_target;
            //     let movement = normalized_direction * player_speed * time.delta_seconds();

            //     if movement.length() < distance_to_target {
            //         //transform.translation += Vec3::new(movement.x, movement.y, 0.0);
            //         direction.y += 1.;
            //         // Update player position
            //     } else {
            //         // transform.translation =
            //         Vec3::new(player.target_position.x, player.target_position.y, 0.0);
            //         // Update player position
            //         direction.y += 1.;
            //         player.moving = false;
            //         info!(
            //             "input {:?}, player pos {:?}, player: {:?}",
            //             input, transform.translation, player.handle
            //         );
            //     }
            // }
            // let egg = target.0 - Vec2::new(transform.translation.x, transform.translation.y);
            // direction.y += egg.y;
            // direction.y += 1.;

            direction = target.0 - Vec2::new(transform.translation.x, transform.translation.y);
        }

        if direction == Vec2::ZERO {
            continue;
        }

        let move_speed = 0.13;
        let move_delta = (direction * move_speed).extend(0.);

        transform.translation += move_delta;
    }

    // let (camera, camera_transform) = camera_query.single();

    // for (mut transform, mut player) in player_query.iter_mut() {
    //     let (input, _) = inputs[player.handle];

    //     if input & INPUT_MOVE != 0 {
    //         for window in windows.iter_mut() {
    //             if let Some(cursor) = window.cursor_position() {
    //                 let world_position =
    //                     window_to_world_coordinates(&window, camera, camera_transform, cursor);
    //                 player.target_position = world_position;
    //                 player.moving = true;
    //             }
    //         }
    //     }

    //     if player.moving {
    //         let current_position = transform.translation;
    //         let direction =
    //             player.target_position - Vec2::new(current_position.x, current_position.y);
    //         let distance_to_target = direction.length();

    //         if distance_to_target > 0.0 {
    //             let player_speed = 10.0;
    //             let normalized_direction = direction / distance_to_target;
    //             let movement = normalized_direction * player_speed * time.delta_seconds();

    //             if movement.length() < distance_to_target {
    //                 transform.translation += Vec3::new(movement.x, movement.y, 0.0);
    //                 // Update player position
    //             } else {
    //                 transform.translation =
    //                     Vec3::new(player.target_position.x, player.target_position.y, 0.0);
    //                 // Update player position
    //                 player.moving = false;
    //                 info!(
    //                     "input {:?}, player pos {:?}, player: {:?}",
    //                     input, transform.translation, player.handle
    //                 );
    //             }
    //             let horizontal_movement = movement.x.abs() > f32::EPSILON;
    //             let normalized_direction = direction / distance_to_target;
    //             if horizontal_movement {
    //                 if normalized_direction.x > 0.0 {
    //                     player.facing_right = true;
    //                 } else {
    //                     player.facing_right = false;
    //                 }
    //             }
    //         }
    //     }
    //     info!("player: {:?}, {:?}", player.handle, transform.translation);
    // }
}

pub fn update_facing(mut player_query: Query<(&Player, &mut Transform)>) {
    for (player, mut transform) in player_query.iter_mut() {
        if player.facing_right {
            transform.rotation = Quat::from_rotation_y(0.0); // Face right
        } else {
            transform.rotation = Quat::from_rotation_y(std::f32::consts::PI); // Face left
        }
    }
}

fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    info!("connecting to matchbox server: {:?}", room_url);
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

fn wait_for_players(mut commands: Commands, mut socket: ResMut<MatchboxSocket<SingleChannel>>) {
    if socket.get_channel(0).is_err() {
        return; // we've already started
    }

    // Check for new connections
    socket.update_peers();
    let players = socket.players();

    let num_players = 2;
    if players.len() < num_players {
        return; // wait for more players
    }

    info!("All peers have joined, going in-game");

    // create a GGRS P2P session
    let mut session_builder = ggrs::SessionBuilder::<GgrsConfig>::new()
        .with_num_players(num_players)
        .with_input_delay(2);

    for (i, player) in players.into_iter().enumerate() {
        session_builder = session_builder
            .add_player(player, i)
            .expect("failed to add player");
    }

    // move the channel out of the socket (required because GGRS takes ownership of it)
    let socket = socket.take_channel(0).unwrap();

    // start the GGRS session
    let ggrs_session = session_builder
        .start_p2p_session(socket)
        .expect("failed to start session");

    commands.insert_resource(bevy_ggrs::Session::P2PSession(ggrs_session));
}

fn input(
    _: In<ggrs::PlayerHandle>,
    keys: Res<Input<KeyCode>>,
    touches: Res<Touches>,
    mouse: Res<Input<MouseButton>>,
) -> u8 {
    let mut input = 0u8;

    if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        input |= INPUT_MOVE;
    }

    // if touches.iter_just_pressed().count() > 0 {
    //     input |= INPUT_MOVE;
    // }
    // if touches.iter_just_pressed() {
    //     input |= INPUT_MOVE;
    // }

    // if keys.any_pressed([KeyCode::Down, KeyCode::S]) {
    //     input |= INPUT_DOWN;
    // }
    // if keys.any_pressed([KeyCode::Left, KeyCode::A]) {
    //     input |= INPUT_LEFT
    // }
    // if keys.any_pressed([KeyCode::Right, KeyCode::D]) {
    //     input |= INPUT_RIGHT;
    // }
    // if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
    //     input |= INPUT_FIRE;
    // }

    input
}
