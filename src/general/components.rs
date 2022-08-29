use nalgebra_glm as glm;

use bevy::prelude::*;

#[derive(Component)]
pub struct PositionComp {
    pub(crate) position: glm::Vec2,
}

#[derive(Component)]
pub struct VelocityComp {
    pub(crate) velocity: glm::Vec2,
    pub(crate) wanted_velocity: glm::Vec2,
    pub(crate) jump_pressed: bool,
}

#[derive(Component)]
pub struct MousePosition {
    pub(crate) position: glm::Vec2,
}