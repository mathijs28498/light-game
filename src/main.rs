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
    input::mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    prelude::*,
    window::{close_on_esc, CursorMoved, PresentMode, WindowMode, WindowResizeConstraints},
};
use game_object::game_object::{EnvironmentObjectComp, Light, MouseLight, AABB, Ray, EnvironmentObject};
use game_object::help_functions::calculate_indices_polygon;
use nalgebra::*;
use rand::prelude::*;

mod vulkano_backend;
use vulkano::{device::Features, pipeline::compute};
use vulkano_backend::{
    compute_device::{self}, //ComputeDevice, PushConstants, BUFFER_SIZE, HEIGHT, WIDTH,},
    test_multi_render_passes::multi_main,
    vulkano_device::{self, RenderObject, SimpleVertex, VulkanoDevice},
};
use vulkano_util::{context::VulkanoConfig, window::VulkanoWindows};

mod bevy_render_plugin;
use bevy_render_plugin::main_render_plugin::MainRenderPlugin;

use nalgebra_glm as glm;

mod game_object;

use rand::Rng;
use std::sync::Arc;

use bevy_vulkano::{VulkanoWinitConfig, VulkanoWinitPlugin};

#[derive(Component)]
pub struct MousePosition {
    pub position: glm::Vec2,
}

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
        .add_system(update_light_polygons_system)
        .add_system(print_mouse_events_system)
        .run();
}

fn update_light_polygons_system(
    mut light_query: Query<(&mut RenderObject<SimpleVertex>, &mut Light)>,
    env_object_query: Query<&AABB, With<EnvironmentObjectComp>>,
) {
    for (mut render_object, mut light) in &mut light_query.iter_mut() {
        let light_polygon = light.calculate_light_polygon(&env_object_query);
        let light_vertices = light_polygon.iter().map(|p| SimpleVertex {
            position: [p.x, p.y],
        });

        let mut vertices = vec![SimpleVertex {
            position: [light.get_center().x, light.get_center().y],
        }];
        vertices.extend(light_vertices);
        while vertices.len() < 3 {
            vertices.push(SimpleVertex { position: [0., 0.] });
        }

        render_object.update_vertex_buffer(vertices);

        // let indices = calculate_indices_polygon(vertices.len() - 1);
    }
}

// TODO: Make sure the mouse position is correct after resizing
fn print_mouse_events_system(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
    mut mouse_light_query: Query<(&MouseLight, &mut Light)>,
) {
    let mp = mouse_position.as_mut();
    for event in cursor_moved_events.iter() {
        mp.position.x = event.position.x;
        mp.position.y = 720. - event.position.y;

        for (mouse_light, mut light) in &mut mouse_light_query.iter_mut() {
            light.set_center(mp.position.clone());
        }
    }
}
