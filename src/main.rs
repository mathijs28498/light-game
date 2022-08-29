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
pub(crate) mod glm_traits;
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
        group.add(PhysicsPlugin);
        group.add(GeneralPlugin);
    }
}

fn main() {
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
        .add_plugins(PluginBundle)
        .add_system(close_on_esc)
        .add_system(mouse_event_system2)
        .run();
}

// TODO: Make system on startup for renderobject that gets its queue
fn mouse_event_system2(
    mut commands: Commands,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mouse_position: Res<MousePosition>,
    player_query: Query<(&PositionComp, &LightComp), With<PlayerLightComp>>,
) {
    let mp = mouse_position.as_ref();

    let colors = vec![
        glm::Vec3::new(0.85, 0.33, 0.04),
        glm::Vec3::new(0.23, 0.85, 0.09),
        glm::Vec3::new(0.85, 0.06, 0.2),
        glm::Vec3::new(0.09, 0.7, 0.7),
        glm::Vec3::new(0.85, 0.06, 0.06),
    ];
    let mut rng = rand::thread_rng();

    for event in mouse_button_input_events.iter() {
        if event.state == ButtonState::Pressed && event.button == MouseButton::Left {
            let (player_pos, light) = player_query.single();
            let dir = (mp.position - player_pos.position).normalize();
            let light_pos = player_pos.position + dir * 50.;

            commands
                .spawn()
                .insert(PositionComp {
                    position: light_pos.clone(),
                })
                .insert(EnvironmentObjectComp)
                .insert(LightComp::new(
                    colors[rng.gen_range(0..5)]
                        + glm::Vec3::new(
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                        ),
                    100.,
                    3.,
                ))
                .insert(RenderObject::<LightVertex>::new())
                .insert(VelocityComp {
                    velocity: dir * 350.,
                    wanted_velocity: dir * 350.,
                    jump_pressed: false,
                });
        }
    }
}
