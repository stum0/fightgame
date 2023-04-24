use bevy::prelude::*;
use bevy_mod_simplest_healthbar::HealthTrait;

#[derive(Component, Reflect, Default)]
pub struct BulletReady {
    pub ready: bool,
    pub timer: Timer,
}

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
pub struct LobbyText;
#[derive(Component)]
pub struct LobbyUI;

#[derive(Component, Reflect, Default, Clone, Copy)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}
impl HealthTrait for Health {
    fn current(&self) -> u32 {
        self.current
    }

    fn max(&self) -> u32 {
        self.max
    }
}

#[derive(Component)]
pub struct BarCamera;