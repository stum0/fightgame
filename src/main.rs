use bevy::reflect::TypeUuid;
use bevy::{prelude::*, render::camera::ScalingMode, tasks::IoTaskPool, window::Window};
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;

const INPUT_MOVE: u8 = 1 << 0;
const INPUT_FIRE: u8 = 1 << 1;

fn main() {
    let mut app = App::new();

    app.add_state::<AppState>();

    GGRSPlugin::<GgrsConfig>::new()
        .with_input_system(input)
        .register_rollback_component::<Transform>()
        .build(&mut app);

    app.insert_resource(ClearColor(Color::rgb_u8(0, 0, 0)));

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "SwordGame".to_string(),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            ..Default::default()
        }),
        ..Default::default()
    }));

    app.add_startup_systems((setup, start_matchbox_socket));

    app.add_systems((
        move_to_click.in_schedule(GGRSSchedule),
        update_facing,
        wait_for_players,
        movement,
    ));
    app.run();
}

#[derive(States, PartialEq, Eq, Debug, Clone, Hash, Default)]
enum AppState {
    #[default]
    InGame,
}

#[derive(Component, Debug, Default, Clone, Reflect, TypeUuid)]
#[uuid = "a8ab5281-68a6-41f8-b165-72bf8075d4fe"]
pub struct Player {
    pub position: Vec2,
    pub facing_right: bool,
    handle: usize,
    moving: bool,
    pub target_position: Vec2,
}

// #[derive(Component, Debug, Default, Clone, Reflect, TypeUuid)]
// #[uuid = "a19dd532-79dd-42e7-90ce-645d635412f6"]
// pub struct Target {
//     position: Vec2,
// }

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rip: ResMut<RollbackIdProvider>,
) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    //player 1
    let p1_position = Vec2::new(-5.0, 0.0);
    commands.spawn((
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
        Player {
            position: p1_position,
            facing_right: true,
            handle: 0,
            moving: false,
            target_position: p1_position,
        },
        rip.next(),
    ));
    //player 2
    let p2_position = Vec2::new(5.0, 0.0);
    commands.spawn((
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
        Player {
            position: p2_position,
            facing_right: false,
            handle: 1,
            moving: false,
            target_position: p2_position,
        },
        rip.next(),
    ));
}

pub fn movement(
    mut windows: Query<&mut Window>,
    mut player_query: Query<(&mut Player, &mut Transform)>,
    touches: Res<Touches>,
    mouse: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera_query.single();

    for touch in touches.iter_just_pressed() {
        let touch_pos = touch.position();
        for (mut player, _) in player_query.iter_mut() {
            for window in windows.iter_mut() {
                let world_position =
                    window_to_world_coordinates_touch(&window, camera, camera_transform, touch_pos);
                player.target_position = world_position;
            }
        }
    }

    for (mut player, _) in player_query.iter_mut() {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
                    let world_position =
                        window_to_world_coordinates(&window, camera, camera_transform, cursor);
                    player.target_position = world_position;
                }
            }
        }
    }
}

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
    mut player_query: Query<(&mut Transform, &mut Player)>,
    time: Res<Time>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    //keyboard: Res<Input<KeyCode>>,
) {
    for (mut transform, mut player) in player_query.iter_mut() {
        let (input, _) = inputs[player.handle];
        info!("eeeeeeee {:?}", input);

        if input & INPUT_MOVE != 0 {
            player.moving = true;
        }
        if player.moving {
            let current_position = player.position;
            let direction = player.target_position - current_position;
            let distance_to_target = direction.length();

            if distance_to_target > 0.0 {
                let player_speed = 10.0;
                let normalized_direction = direction / distance_to_target;
                let movement = normalized_direction * player_speed * time.delta_seconds();

                if movement.length() < distance_to_target {
                    player.position += movement;
                } else {
                    player.position = player.target_position
                }
                let horizontal_movement = movement.x.abs() > f32::EPSILON;
                let normalized_direction = direction / distance_to_target;
                if horizontal_movement {
                    if normalized_direction.x > 0.0 {
                        player.facing_right = true;
                    } else {
                        player.facing_right = false;
                    }
                }
            } else {
                player.moving = false;
            }
            transform.translation = Vec3::new(player.position.x, player.position.y, 0.0);
            println!("Player position: {:?}", player.position);
        }
    }
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

struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = u8;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are called `PeerId`s
    type Address = PeerId;
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

    if touches.iter_just_pressed().count() > 0 {
        input |= INPUT_MOVE;
    }
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
