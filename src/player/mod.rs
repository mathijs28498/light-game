pub(crate) mod components;
pub(crate) mod system;

use bevy::prelude::*;

use crate::player::system::*;

pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(shoot_light_system)
            .add_system(move_camera_system);
    }
}
