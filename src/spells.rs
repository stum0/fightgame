use bevy::{math::Vec3Swizzles, prelude::*, time, utils::Instant};
use bevy_ggrs::{PlayerInputs, RollbackIdProvider};

use crate::{
    components::{Bullet, BulletDistance, BulletReady, Despawned, Health, MoveDir, Player},
    input::fire,
    GgrsConfig, ImageAssets,
};

const PLAYER_RADIUS: f32 = 0.5;
const BULLET_RADIUS: f32 = 0.025;
pub const BULLET_SPEED: f32 = 0.1;

pub fn fire_bullets(
    mut commands: Commands,
    inputs: Res<PlayerInputs<GgrsConfig>>,
    images: Res<ImageAssets>,
    mut player_query: Query<(&Transform, &mut Player, &mut BulletReady, &mut MoveDir)>,
    mut rip: ResMut<RollbackIdProvider>,
) {
    for (transform, mut player, mut bullet, mut move_dir) in player_query.iter_mut() {
        let (input, _) = inputs[player.handle];

        let mouse_position = Vec2::new(input.target_x, input.target_y);

        if fire(input) && bullet.ready {
            let player_pos = transform.translation.xy();
            let direction_to_mouse = (mouse_position - player_pos).normalize();
            let pos = player_pos + direction_to_mouse * PLAYER_RADIUS + BULLET_RADIUS;
            if direction_to_mouse.x > 0.0 {
                move_dir.0 = Vec2::X;
                player.facing_right = true;
            } else {
                move_dir.0 = -Vec2::X;
                player.facing_right = false;
            }
            commands.spawn((
                Bullet {
                    shooter: player.handle,
                },
                rip.next(),
                BulletDistance { traveled: 0.0 },
                MoveDir(direction_to_mouse),
                SpriteBundle {
                    transform: Transform::from_translation(pos.extend(500.))
                        .with_rotation(Quat::from_rotation_arc_2d(Vec2::X, direction_to_mouse)),
                    texture: images.bullet.clone(),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(0.3, 0.3)),
                        ..default()
                    },
                    ..default()
                },
            ));
            bullet.ready = false;
        }
    }
}

pub fn reload_bullet(
    inputs: Res<PlayerInputs<GgrsConfig>>,
    mut query: Query<(&mut BulletReady, &Player)>,
    time: Res<Time>,
) {
    for (mut can_fire, player) in query.iter_mut() {
        let (input, _) = inputs[player.handle];

        can_fire.timer.tick(time.delta());

        if can_fire.timer.finished() && !fire(input) {
            // Reset the timer
            can_fire.ready = true;
            can_fire.timer.reset();
        }
    }
}

pub fn move_bullet(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &MoveDir, &mut BulletDistance), With<Bullet>>,
) {
    for (bullet, mut transform, dir, mut distance) in query.iter_mut() {
        if distance.traveled < 3.0 {
            let delta = (dir.0 * BULLET_SPEED).extend(0.);
            transform.translation += delta;

            // Update the traveled distance
            distance.traveled += BULLET_SPEED;
        } else {
            commands.entity(bullet).despawn();
        }
    }
}

pub fn kill_players(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Transform, &Player, &mut Health), Without<Bullet>>,

    bullet_query: Query<(Entity, &Transform, &Bullet), With<Bullet>>,
) {
    for (player, player_transform, player_info, mut health) in player_query.iter_mut() {
        for (bullet, bullet_transform, bullet_info) in bullet_query.iter() {
            let distance = Vec2::distance(
                player_transform.translation.xy(),
                bullet_transform.translation.xy(),
            );
            // Check if the bullet's shooter handle is different from the player's handle

            if distance < PLAYER_RADIUS + BULLET_RADIUS && bullet_info.shooter != player_info.handle
            {
                health.current -= 1;
                if health.current == 0 {
                    commands.entity(player).despawn();
                    info!("Player {} died", player_info.handle);
                }

                commands.entity(bullet).despawn();
            }
        }
    }
}
