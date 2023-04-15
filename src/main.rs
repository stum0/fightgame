use bevy::math::Vec3Swizzles;
use bevy::{prelude::*, render::camera::ScalingMode, window::Window};
use bevy_asset_loader::prelude::*;
use bevy_ggrs::*;
use bevy_matchbox::prelude::*;
use bytemuck::{Pod, Zeroable};
use rand::Rng;
use serde::{Deserialize, Serialize};

const INPUT_MOVE: u8 = 1 << 0;
const INPUT_FIRE: u8 = 1 << 1;

// #[wasm_bindgen]
pub fn main() {
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
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_systems((setup, start_matchbox_socket).in_schedule(OnEnter(GameState::Matchmaking)))
        .add_systems((
            wait_for_players.run_if(in_state(GameState::Matchmaking)),
            spawn_players.in_schedule(OnEnter(GameState::InGame)),
        ))
        .add_system(log_ggrs_events.in_set(OnUpdate(GameState::InGame)))
        .add_systems(
            (
                move_system,
                reload_bullet.after(move_system),
                fire_bullets.after(move_system).after(reload_bullet),
                move_bullet.after(fire_bullets),
                kill_players.after(move_bullet).after(move_system),
                respawn_players.after(kill_players),
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

#[derive(Component, Reflect, Default)]
pub struct BulletReady {
    pub ready: bool,
    pub timer: Timer,
}

#[derive(Component, Reflect, Default, Clone, Copy)]
pub struct MoveDir(pub Vec2);

#[derive(Component)]
pub struct Player {
    pub facing_right: bool,
    handle: usize,
    moving: bool,
}

#[derive(Default, Reflect, Component)]
pub struct Target {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Reflect, Default)]
pub struct Bullet;

#[derive(AssetCollection, Resource)]
struct ImageAssets {
    #[asset(path = "bullet.png")]
    bullet: Handle<Image>,
    #[asset(path = "bevy_pixel_dark.png")]
    player: Handle<Image>,
}

#[derive(Debug)]
struct GgrsConfig;

impl ggrs::Config for GgrsConfig {
    // 4-directions + fire fits easily in a single byte
    type Input = CustomInput;
    type State = u8;
    // Matchbox' WebRtcSocket addresses are called `PeerId`s
    type Address = PeerId;
}

#[derive(Component)]
pub struct Despawned;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CustomInput {
    pub inp: u8,
    pub target_x: f32,
    pub target_y: f32,
}

unsafe impl Zeroable for CustomInput {}
unsafe impl Pod for CustomInput {}

fn setup(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);
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
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0., 0.47, 1.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: images.player.clone(),
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
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1., 0.47, 0.),
                custom_size: Some(Vec2::new(1., 1.)),
                ..Default::default()
            },
            texture: images.player.clone(),
            transform: Transform::from_xyz(p2_position.x, p2_position.y, 0.0),
            ..Default::default()
        },
    ));
}

fn get_click_position(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    cursor_position: Vec2,
) -> Vec2 {
    let screen_size = Vec2::new(window.width(), window.height());
    let screen_position = cursor_position / screen_size;
    let clip_position = (screen_position - Vec2::new(0.5, 0.5)) * 2.0;
    let mut click_position = camera
        .projection_matrix()
        .inverse()
        .project_point3(clip_position.extend(0.0));
    click_position = *camera_transform * click_position;
    click_position.truncate()
}

fn get_touch_position(
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
    let mut touch_position = camera
        .projection_matrix()
        .inverse()
        .project_point3(clip_position.extend(0.0));
    touch_position = *camera_transform * touch_position;
    touch_position.truncate()
}

fn move_system(
    mut query: Query<(&mut Transform, &mut Target, &mut Player, &mut MoveDir), With<Rollback>>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    time: Res<Time>,
) {
    for (mut t, mut tg, mut p, mut move_dir) in query.iter_mut() {
        let input = inputs[p.handle].0.inp;

        if input & INPUT_MOVE != 0 {
            let click_position =
                Vec2::new(inputs[p.handle].0.target_x, inputs[p.handle].0.target_y);

            tg.x = click_position.x;
            tg.y = click_position.y;
            p.moving = true;
        }

        if p.moving {
            let current_position = Vec2::new(t.translation.x, t.translation.y);
            let direction = Vec2::new(tg.x, tg.y) - current_position;
            let distance_to_target = direction.length();

            if distance_to_target > 0.0 {
                let player_speed = 10.0;
                let normalized_direction = direction / distance_to_target;
                let movement = normalized_direction * player_speed * time.delta_seconds();

                if movement.length() < distance_to_target {
                    t.translation += Vec3::new(movement.x, movement.y, 0.0);
                } else {
                    t.translation = Vec3::new(tg.x, tg.y, 0.0);
                    p.moving = false;
                }
                if normalized_direction.x > 0.0 {
                    move_dir.0 = Vec2::X;
                    p.facing_right = true;
                } else {
                    move_dir.0 = -Vec2::X;
                    p.facing_right = false;
                }
            } else {
                p.moving = false;
            }
        }
    }
}

fn start_matchbox_socket(mut commands: Commands) {
    let room_url = "ws://127.0.0.1:3536/extreme_bevy?next=2";
    info!("connecting to matchbox server: {:?}", room_url);
    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

fn wait_for_players(
    mut commands: Commands,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
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
    next_state.set(GameState::InGame);
}

pub fn input(
    _handle: In<ggrs::PlayerHandle>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut windows: Query<&mut Window>,
    touches: Res<Touches>,
) -> CustomInput {
    let mut input = CustomInput {
        inp: 0,
        target_x: 0.0,
        target_y: 0.0,
    };

    let mut touch_count = 0;
    for touch in touches.iter() {
        touch_count += 1;
        let touch_pos = touch.position();
        let (camera, camera_transform) = camera_query.single();

        for window in windows.iter_mut() {
            let touch_position = get_touch_position(&window, camera, camera_transform, touch_pos);
            input.target_x = touch_position.x;
            input.target_y = touch_position.y;
        }
        input.inp |= INPUT_MOVE;
    }

    if touch_count >= 2 {
        input.inp |= INPUT_FIRE;
    }

    if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                let (camera, camera_transform) = camera_query.single();
                let click_position = get_click_position(&window, camera, camera_transform, cursor);
                input.target_x = click_position.x;
                input.target_y = click_position.y;
            }
        }
        input.inp |= INPUT_MOVE;
    }

    if keys.any_pressed([KeyCode::Space, KeyCode::Return]) {
        input.inp |= INPUT_FIRE;
    }

    input
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
const PLAYER_RADIUS: f32 = 0.5;
const BULLET_RADIUS: f32 = 0.025;

fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    images: Res<ImageAssets>,
    mut player_query: Query<(&Transform, &Player, &mut BulletReady, &MoveDir)>,
    mut rip: ResMut<RollbackIdProvider>,
    time: Res<Time>,
) {
    for (transform, player, mut bullet_ready, move_dir) in player_query.iter_mut() {
        let (input, _) = inputs[player.handle];
        if fire(input) && bullet_ready.ready {
            bullet_ready.timer.tick(time.delta());
            if bullet_ready.timer.finished() {
                let player_pos = transform.translation.xy();
                let pos = player_pos + move_dir.0 * PLAYER_RADIUS + BULLET_RADIUS;
                commands.spawn((
                    Bullet,
                    rip.next(),
                    *move_dir,
                    SpriteBundle {
                        transform: Transform::from_translation(pos.extend(200.))
                            .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, move_dir.0)),
                        texture: images.bullet.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(0.3, 0.1)),
                            ..default()
                        },
                        ..default()
                    },
                ));
                bullet_ready.timer.reset();
            }
        } else {
            bullet_ready.timer.tick(time.delta());
        }
    }
}

pub fn fire(input: CustomInput) -> bool {
    input.inp & INPUT_FIRE != 0
}

fn reload_bullet(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut query: Query<(&mut BulletReady, &Player)>,
) {
    for (mut can_fire, player) in query.iter_mut() {
        let (input, _) = inputs[player.handle];
        if !fire(input) {
            can_fire.ready = true;
        }
    }
}

fn move_bullet(mut query: Query<(&mut Transform, &MoveDir), With<Bullet>>) {
    for (mut transform, dir) in query.iter_mut() {
        let delta = (dir.0 * 0.35).extend(0.);
        transform.translation += delta;
    }
}

fn kill_players(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), (With<Player>, Without<Bullet>)>,
    bullet_query: Query<&Transform, With<Bullet>>,
) {
    for (player, player_transform) in player_query.iter() {
        for bullet_transform in bullet_query.iter() {
            let distance = Vec2::distance(
                player_transform.translation.xy(),
                bullet_transform.translation.xy(),
            );
            if distance < PLAYER_RADIUS + BULLET_RADIUS {
                commands.entity(player).insert(Despawned);
            }
        }
    }
}

fn respawn_players(
    mut commands: Commands,
    player_query: Query<(Entity, &Player), (With<Despawned>, Without<Bullet>)>,
) {
    for (entity, player) in player_query.iter() {
        let position = match player.handle {
            0 => Vec2::new(-5.0, 0.0),
            1 => Vec2::new(5.0, 0.0),
            _ => unreachable!(),
        };

        commands.entity(entity).remove::<Despawned>();
        commands
            .entity(entity)
            .insert(Transform::from_xyz(position.x, position.y, 0.0));
    }
}

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
