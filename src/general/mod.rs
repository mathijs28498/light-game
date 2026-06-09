pub(crate) mod components;
pub(crate) mod data_types;
pub(crate) mod functions;
pub(crate) mod system;

use nalgebra_glm as glm;

use bevy::{
    app::PluginGroupBuilder,
    core::CorePlugin,
    diagnostic::{DiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    input::InputPlugin,
    log::LogPlugin,
    prelude::*,
    time::TimePlugin,
    window::{close_on_esc, PresentMode, WindowMode},
};
use bevy_vulkano::VulkanoWinitPlugin;

use crate::{
    general::{components::*, data_types::*, system::*},
    physics::*,
    player::*,
    rendering::*,
};

pub(crate) struct GeneralPlugin;

impl Plugin for GeneralPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MousePosition {
            position: glm::vec2(0., 0.),
        })
        // Window configs for primary window
        .insert_resource(WindowDescriptor {
            width: 1280.,
            height: 720.,
            title: "Bevy Vulkano".to_string(),
            present_mode: PresentMode::Immediate,
            resizable: false,
            mode: WindowMode::Windowed,
            ..WindowDescriptor::default()
        })
        .add_system(mouse_position_update_system)
        .add_system(close_on_esc);
    }
}

pub(crate) struct GeneralPluginBundle;

impl PluginGroup for GeneralPluginBundle {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        // Basic Bevy plugins
        group.add(LogPlugin);
        group.add(CorePlugin);
        group.add(TimePlugin);
        group.add(DiagnosticsPlugin);
        group.add(FrameTimeDiagnosticsPlugin);
        group.add(InputPlugin);

        // This plugin has to go first. 
        // If the WindowDescriptor is added after the vulkanowinitplugin,
        // the fps will be capped at 60
        group.add(GeneralPlugin);

        // Don't add default bevy plugins or WinitPlugin. This owns "core loop" (runner).
        // Bevy winit and render should be excluded
        group.add(VulkanoWinitPlugin);

        // Custom plugins
        group.add(RenderPlugin);
        group.add(PhysicsPlugin);
        group.add(PlayerPlugin);
    }
}
