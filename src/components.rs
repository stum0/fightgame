use bevy::prelude::*;
use nostr_sdk::Keys;

#[derive(Component, Reflect, Default)]
pub struct BulletReady(pub bool);

#[derive(Component, Reflect, Default)]
pub struct BulletDistance {
    pub traveled: f32,
}

#[derive(Component, Reflect, Default, Clone, Copy)]
pub struct MoveDir(pub Vec2);

#[derive(Component)]
pub struct Player {
    pub facing_right: bool,
    pub handle: usize,
    pub moving: bool,
}

#[derive(Default, Reflect, Component)]
pub struct Target {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Reflect, Default)]
pub struct Bullet {
    pub shooter: usize,
}

#[derive(Component)]
pub struct Despawned;

#[derive(Component)]
pub struct Nostr {
    pub keys: Keys,
    pub relay: String,
}
