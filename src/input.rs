use std::time::Duration;

use bevy::prelude::*;
use bevy::utils::Instant;
use bevy_ggrs::{ggrs, PlayerInputs, Rollback};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::{
    components::{MoveDir, Player, Target},
    GgrsConfig,
};

const INPUT_MOVE: u8 = 1 << 0;
const INPUT_FIRE: u8 = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CustomInput {
    pub inp: u8,
    pub target_x: f32,
    pub target_y: f32,
}

unsafe impl Zeroable for CustomInput {}
unsafe impl Pod for CustomInput {}

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
    let mut last_touch_timestamp: Option<Instant> = None;
    let touch_threshold = Duration::from_secs_f32(2.0);

    for touch in touches.iter() {
        let touch_pos = touch.position();
        let (camera, camera_transform) = camera_query.single();

        for window in windows.iter_mut() {
            let touch_position = get_touch_position(&window, camera, camera_transform, touch_pos);
            input.target_x = touch_position.x;
            input.target_y = touch_position.y;
        }

        // Check if the current touch is within the threshold since the last touch
        if let Some(last_timestamp) = last_touch_timestamp {
            if Instant::now().duration_since(last_timestamp) < touch_threshold {
                // If within the threshold, trigger the shoot action and don't move
                input.inp |= INPUT_FIRE;
                last_touch_timestamp = None;
            } else {
                // If not within the threshold, update the last touch timestamp and move
                last_touch_timestamp = Some(Instant::now());
                input.inp |= INPUT_MOVE;
            }
        } else {
            // If there was no previous touch, update the last touch timestamp and move
            last_touch_timestamp = Some(Instant::now());
            input.inp |= INPUT_MOVE;
        }
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

    if keys.pressed(KeyCode::Q) {
        for window in windows.iter_mut() {
            if let Some(cursor) = window.cursor_position() {
                let (camera, camera_transform) = camera_query.single();
                let click_position = get_click_position(&window, camera, camera_transform, cursor);
                input.target_x = click_position.x;
                input.target_y = click_position.y;
            }
        }
        input.inp |= INPUT_FIRE;
    }

    input
}

pub fn get_click_position(
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

pub fn get_touch_position(
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

pub fn move_system(
    mut query: Query<(&mut Transform, &mut Target, &mut Player, &mut MoveDir), With<Rollback>>,
    inputs: Res<PlayerInputs<GgrsConfig>>,
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
                let player_speed = 0.05;
                let normalized_direction = direction / distance_to_target;
                let movement = normalized_direction * player_speed;

                if movement.length() < distance_to_target {
                    t.translation += Vec3::new(movement.x, movement.y, 0.0);
                } else {
                    t.translation = Vec3::new(tg.x, tg.y, 0.0);
                    p.moving = false;
                }
                if normalized_direction.x > 0.0 {
                    move_dir.0 = Vec2::X;
                    t.rotation = Quat::from_rotation_y(std::f32::consts::PI);
                } else {
                    move_dir.0 = -Vec2::X;
                    t.rotation = Quat::from_rotation_y(0.0);
                }
            } else {
                p.moving = false;
            }
        }
    }
}

pub fn fire(input: CustomInput) -> bool {
    input.inp & INPUT_FIRE != 0
}
