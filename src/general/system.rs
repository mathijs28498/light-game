use bevy::prelude::*;

use crate::{
    general::data_types::*,
};

// TODO: Make sure the mouse position is correct after resizing
pub(super) fn mouse_position_update_system(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    let mp = mouse_position.as_mut();
    for event in cursor_moved_events.iter() {
        mp.position.x = event.position.x;
        mp.position.y = 720. - event.position.y;
    }
}