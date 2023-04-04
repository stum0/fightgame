use bevy::{prelude::*, render::camera::ScalingMode, window::Window};
use log::Level;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(Level::Info));
    let mut app = App::new();

    app.add_state::<AppState>();

    app.insert_resource(ClearColor(Color::rgb_u8(0, 0, 0)));

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "SwordGame".to_string(),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: true,
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
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);

    // commands.spawn(MaterialMesh2dBundle {
    //     mesh: meshes
    //         .add(Mesh::from(shape::Quad::new(Vec2::new(width, height))))
    //         .into(),
    //     transform: Transform::from_xyz(0.0, height - 125.0, 0.0),
    //     material: materials.add(ColorMaterial::from(Color::PURPLE)),
    //     ..default()
    // });

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
        },
        Target {
            position: p1_position,
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
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera_query.single();

    for touch in touches.iter_just_pressed() {
        let touch_pos = touch.position();
        for mut target in target_query.iter_mut() {
            for window in windows.iter_mut() {
                let world_position =
                    window_to_world_coordinates_touch(&window, camera, camera_transform, touch_pos);
                target.position = world_position;
            }
        }
    }

    for mut target in target_query.iter_mut() {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                if mouse.just_pressed(MouseButton::Left) || mouse.just_pressed(MouseButton::Right) {
                    let world_position =
                        window_to_world_coordinates(&window, camera, camera_transform, cursor);
                    target.position = world_position;
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

pub fn move_to_click(
    mut player_query: Query<(&mut Transform, &mut Player)>,
    mut target_query: Query<&mut Target>,
    time: Res<Time>,
    //keyboard: Res<Input<KeyCode>>,
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
            let player_speed = 10.0;
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
        println!("Player position: {:?}", player.position);
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
