pub(crate) mod components;
pub(crate) mod data_types;
pub(crate) mod functions;
pub(crate) mod system;

use bevy::{
    prelude::*,
    app::PluginGroupBuilder,
    window::close_on_esc,
};
use bevy_vulkano::VulkanoWinitPlugin;

use crate::general::system::*;

pub(crate) struct GeneralPlugin;

impl Plugin for GeneralPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(mouse_position_update_system)
            .add_system(close_on_esc);
    }
}

pub(crate) struct GeneralPluginBundle;

impl PluginGroup for GeneralPluginBundle {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        // Minimum plugins for the demo
        group.add(bevy::log::LogPlugin);
        group.add(bevy::core::CorePlugin);
        group.add(bevy::time::TimePlugin);
        group.add(bevy::diagnostic::DiagnosticsPlugin);
        group.add(bevy::diagnostic::FrameTimeDiagnosticsPlugin);
        group.add(bevy::input::InputPlugin);
        // Don't add default bevy plugins or WinitPlugin. This owns "core loop" (runner).
        // Bevy winit and render should be excluded
        group.add(VulkanoWinitPlugin);
    }
}
