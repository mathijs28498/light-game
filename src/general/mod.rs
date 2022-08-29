pub(crate) mod components;
pub(crate) mod data_types;
pub(crate) mod functions;
pub(crate) mod system;

use bevy::prelude::*;

use crate::general::system::*;

pub(crate) struct GeneralPlugin;

impl Plugin for GeneralPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(mouse_position_update_system);
    }
}


