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
    input::{
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    prelude::*,
    window::{close_on_esc, CursorMoved, PresentMode, WindowMode, WindowResizeConstraints},
};

use nalgebra::*;
use rand::prelude::*;
use rand::Rng;

use bevy_vulkano::BevyVulkanoWindows;

use vulkano::{device::Features, pipeline::compute};
use vulkano_util::{context::VulkanoConfig, window::VulkanoWindows};

use nalgebra_glm as glm;

use std::sync::Arc;

use bevy_vulkano::{VulkanoWinitConfig, VulkanoWinitPlugin};

pub(crate) mod environment_objects;
pub(crate) mod general;
pub(crate) mod ext_traits;
pub(crate) mod physics;
pub(crate) mod player;
pub(crate) mod rendering;

use crate::{
    environment_objects::{components::*, *},
    general::{components::*, *},
    physics::*,
    player::{components::*, *},
    rendering::{components::*, data_types::*, shader_data_types::*, *},
};

struct AllPluginBundle;

impl PluginGroup for AllPluginBundle {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(RenderPlugin);
        group.add(PhysicsPlugin);
        group.add(GeneralPlugin);
        group.add(PlayerPlugin);
    }
}

fn main() {
    // TODO: Make sure the proper inserts are done in the right places.
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
        .insert_resource(MousePosition {
            position: glm::Vec2::new(0., 0.),
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
        .add_plugins(GeneralPluginBundle)
        .add_plugins(AllPluginBundle)
        .run();
}

