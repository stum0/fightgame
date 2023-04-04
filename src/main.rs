use bevy::{
    prelude::*,
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

            facing_right: true,
        },
        Target {
            position: Vec2::new(-width / 4.0, height),
        },
    ));
}

#[derive(Default, Component)]
pub struct Player {
    pub position: Vec2,

    pub facing_right: bool,
}

#[derive(Default, Component)]
pub struct Target {
    position: Vec2,
}

pub fn movement(
    mut windows: Query<&mut Window>,
    mut target_query: Query<&mut Target>,
    touches: Res<Touches>,
    mouse: Res<Input<MouseButton>>,
) {
    for mut target in target_query.iter_mut() {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                if mouse.pressed(MouseButton::Left)
                    || mouse.pressed(MouseButton::Right)
                    || touches.any_just_pressed()
                {
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
    mut player_query: Query<(&mut Transform, &mut Player)>,
    mut target_query: Query<&mut Target>,
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
) {
    let target = match target_query.iter_mut().next() {
        Some(target) => target,
        None => return,
    };
    for (mut transform, mut player) in player_query.iter_mut() {
        let current_position = player.position;
        let direction = target.position - current_position;
        let distance_to_target = direction.length();

        if distance_to_target > 0.0 {
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

        transform.translation = Vec3::new(player.position.x, player.position.y, 0.0);
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
