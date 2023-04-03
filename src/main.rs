use bevy::{
    math::vec2,
    prelude::*,
    render::render_resource::{Extent3d, Texture, TextureDimension, TextureFormat},
    sprite::MaterialMesh2dBundle,
    window::{Window, WindowResolution},
};

const ARENA_WIDTH: f32 = 1280.0;
const ARENA_HEIGHT: f32 = 800.0;

fn main() {
    let mut app = App::new();

    app.add_state::<AppState>();

    app.insert_resource(ClearColor(Color::rgb_u8(0, 0, 0)));

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "SwordGame".to_string(),
            resolution: WindowResolution::new(ARENA_WIDTH, ARENA_HEIGHT),
            ..Default::default()
        }),
        ..Default::default()
    }));

    app.add_startup_system(setup);
    app.add_system(movement);
    app.add_system(move_to_click);
    app.add_system(jumping);
    app.add_system(dash_cooldown);
    app.add_system(update_facing);
    app.run();
}

#[derive(States, PartialEq, Eq, Debug, Clone, Hash, Default)]
enum AppState {
    #[default]
    InGame,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2dBundle::default());

    let width = ARENA_WIDTH;
    let height = -ARENA_HEIGHT / 4.0;

    //let quad_position = Vec3::new(0.0, -quad_height * 1.5, 0.0);

    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(Mesh::from(shape::Quad::new(Vec2::new(width, height))))
            .into(),
        transform: Transform::from_xyz(0.0, height - 125.0, 0.0),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        ..default()
    });

    commands.spawn((
        SpriteBundle {
            texture: asset_server.load("bevy_pixel_dark.png"),
            transform: Transform::from_xyz(-width / 4.0, height, 0.0),
            ..default()
        },
        Player {
            position: Vec2::new(-width / 4.0, height),
            vertical_velocity: Vec2::ZERO.y,
            jumps_taken: 0,
            facing_right: true,
            on_ground: true,
        },
        Target {
            position: Vec2::new(-width / 4.0, height),
        },
        Dash {
            is_dashing: false,
            distance_dashed: 0.0,
            max_dash_distance: 250.0,
            cooldown: 3.0,
            cooldown_timer: 0.0,
            original_target_position: Vec2::ZERO,
            started_moving: false,
            started_jumping: false,
        },
        Velocity(Vec3::ZERO),
    ));
}

#[derive(Default, Component)]
pub struct Player {
    pub position: Vec2,
    pub vertical_velocity: f32,
    pub jumps_taken: u32,
    pub facing_right: bool,
    pub on_ground: bool,
}

#[derive(Default, Component)]
pub struct Velocity(pub Vec3);

#[derive(Default, Component)]
pub struct Target {
    position: Vec2,
}

#[derive(Default, Component)]
pub struct Dash {
    pub is_dashing: bool,
    pub distance_dashed: f32,
    pub max_dash_distance: f32,
    pub cooldown: f32,
    pub cooldown_timer: f32,
    pub original_target_position: Vec2,
    pub started_moving: bool,
    pub started_jumping: bool,
}

pub fn movement(
    mut windows: Query<&mut Window>,
    mut target_query: Query<&mut Target>,

    mouse: Res<Input<MouseButton>>,
) {
    for mut target in target_query.iter_mut() {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
                    let world_position = window_to_world_coordinates(&window, cursor);
                    target.position.x = world_position.x; // Only update the x position
                }
            }
        }
    }
}

fn window_to_world_coordinates(window: &Window, cursor_position: Vec2) -> Vec2 {
    Vec2::new(
        cursor_position.x - window.width() * 0.5,
        cursor_position.y - window.height() * 0.5,
    )
}

pub fn move_to_click(
    mut player_query: Query<(&mut Transform, &mut Player, &mut Dash)>,
    mut target_query: Query<&mut Target>,
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
) {
    let ground_y = -ARENA_HEIGHT / 4.0;

    let mut target = match target_query.iter_mut().next() {
        Some(target) => target,
        None => return,
    };
    for (mut transform, mut player, mut dash) in player_query.iter_mut() {
        let current_position = player.position;
        let direction = target.position - current_position;
        let distance_to_target = direction.length();

        if keyboard.just_pressed(KeyCode::E) && dash.cooldown_timer <= 0.0 && !dash.is_dashing {
            dash.cooldown_timer = dash.cooldown;
            dash.is_dashing = true;
            dash.original_target_position = target.position;
            dash.started_moving = distance_to_target > dash.max_dash_distance;
            dash.started_jumping = !player.on_ground;
        }

        if dash.is_dashing {
            let dash_direction = if player.facing_right {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(-1.0, 0.0)
            };
            let dash_speed = 500.0;
            let movement = dash_direction * dash_speed * time.delta_seconds();

            player.position += movement;
            dash.distance_dashed += movement.length();

            if dash.distance_dashed >= dash.max_dash_distance {
                dash.is_dashing = false;
                dash.distance_dashed = 0.0;
                // Set the target position based on whether the player started moving or standing still
                if dash.started_moving {
                    target.position = dash.original_target_position;
                } else {
                    target.position = player.position;
                }
            }
        } else if distance_to_target > 0.0 {
            let player_speed = 200.0;
            let normalized_direction = direction / distance_to_target;
            let movement = normalized_direction * player_speed * time.delta_seconds();

            if movement.length() < distance_to_target {
                player.position += movement;
            } else {
                player.position = target.position;
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
        }

        // Apply gravity and vertical velocity
        let gravity = -1000.0; // Change this value to control gravity strength

        if dash.is_dashing && !player.on_ground {
            player.vertical_velocity *= 0.1;
        } else {
            player.vertical_velocity += gravity * time.delta_seconds();
        }

        player.position.y += player.vertical_velocity * time.delta_seconds();

        // Prevent the player from going below the ground
        if player.position.y <= ground_y + 4.0 {
            player.position.y = ground_y;
            player.vertical_velocity = 0.0;
            player.jumps_taken = 0;
            player.on_ground = true;
        } else {
            player.on_ground = false;
        }

        // Turn off dashing when the dash ends in the air
        if dash.is_dashing && dash.distance_dashed >= dash.max_dash_distance {
            dash.is_dashing = false;
            dash.distance_dashed = 0.0;
        }

        transform.translation = Vec3::new(player.position.x, player.position.y, 0.0);
    }
}

pub fn jumping(keyboard: Res<Input<KeyCode>>, mut player_query: Query<&mut Player>) {
    for mut player in player_query.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) && player.jumps_taken < 2 {
            player.vertical_velocity = 500.0; // Change this value to control jump height
            player.jumps_taken += 1;
        }
    }
}

pub fn dash_cooldown(mut player_query: Query<&mut Dash>, time: Res<Time>) {
    for mut dash in player_query.iter_mut() {
        if dash.cooldown_timer > 0.0 {
            dash.cooldown_timer -= time.delta_seconds();
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
