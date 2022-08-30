pub(crate) mod components;
pub(crate) mod system;

use bevy::prelude::*;

use crate::player::system::*;

pub(crate) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(mouse_event_system);
    }
}
