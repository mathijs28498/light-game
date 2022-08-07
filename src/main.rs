// TODO: Implement 2d raytracing
// TODO: Draw using vulkano with a texture in a quad
//          This will make ui libraries work
// TODO: Make image buffer gpu only

#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unreachable_code)]

use bevy::{
    app::{AppExit, PluginGroupBuilder},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::mouse::MouseMotion,
    prelude::*,
    window::{WindowResizeConstraints, WindowMode, PresentMode, close_on_esc},
};
use nalgebra::*;
use rand::prelude::*;

mod vulkano_backend;
use vulkano::{
    pipeline::compute,
    device::Features,
};
use vulkano_backend::{
    compute_device::{self}, //ComputeDevice, PushConstants, BUFFER_SIZE, HEIGHT, WIDTH,},
    vulkano_device::{self, VulkanoDevice},
    test_multi_render_passes::multi_main
};
use vulkano_util::{
    window::VulkanoWindows,
    context::VulkanoConfig,
};

mod bevy_render_plugin;
use bevy_render_plugin::main_render_plugin::MainRenderPlugin;

mod game_object;

use rand::Rng;
use std::sync::Arc;

use bevy_vulkano::{
    VulkanoWinitConfig, VulkanoWinitPlugin
};


struct PluginBundle;

impl PluginGroup for PluginBundle {
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
        // See `MainRenderPlugin` how rendering is orchestrated
        group.add(MainRenderPlugin);
    }
}

fn main() {
    // let mut vd = VulkanoDevice::new_with_initialization();
    // vd.run();
    App::new()
        // Vulkano configs (Modify this if you want to add features to vulkano (vulkan backend).
        // You can also disable primary window opening here
        .insert_non_send_resource(VulkanoWinitConfig {
            vulkano_config: VulkanoConfig {
                device_features: Features {
                    fill_mode_non_solid: true,
                    ..Features::none()
                },
                ..VulkanoConfig::default()
            },
            ..VulkanoWinitConfig::default()
        })
        // Window configs for primary window
        .insert_resource(WindowDescriptor {
            width: 500.,
            height: 500.,
            title: "Bevy Vulkano".to_string(),
            present_mode: PresentMode::Immediate,
            resizable: false,
            mode: WindowMode::Windowed,
            ..WindowDescriptor::default()
        })
        .add_plugins(PluginBundle)
        .add_system(close_on_esc)
        .run();
    
}

