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
use game_object::game_object::*;

use game_object::help_functions::calculate_indices_polygon;
use nalgebra::*;
use rand::prelude::*;
use rand::Rng;

use bevy_vulkano::BevyVulkanoWindows;

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

mod physics;
use physics::general_physics::*;

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
        group.add(PhysicsPlugin);
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
        .add_system(mouse_event_system)
        .add_system(mouse_event_system2)
        .run();
}

fn update_light_polygons_system(
    mut light_query: Query<(&mut RenderObject<SimpleVertex>, &Position, &mut Light)>,
    env_object_query: Query<&AABB, With<EnvironmentObjectComp>>,
) {
    for (mut render_object, position, mut light) in &mut light_query.iter_mut() {
        let (light_polygon, recalculate) = light.calculate_light_polygon(&position, &env_object_query);
        if !recalculate {
            continue;
        }
        let light_vertices = light_polygon.iter().map(|p| SimpleVertex {
            position: [p.x, p.y],
        });

        let mut vertices = vec![SimpleVertex {
            position: [position.position.x, position.position.y],
        }];
        vertices.extend(light_vertices);
        while vertices.len() < 3 {
            vertices.push(SimpleVertex { position: [0., 0.] });
        }

        render_object.update_vertex_buffer(vertices);

        // let indices = calculate_indices_polygon(vertices.len() - 1);
    }
}

// TODO: Make system on startup for renderobject that gets its queue
fn mouse_event_system2(
    mut commands: Commands,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mouse_position: Res<MousePosition>,
    player_query: Query<(&Position, &Light), With<PlayerLight>>,
    vulkano_windows: NonSend<BevyVulkanoWindows>,
) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();
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
                .insert(Position {
                    position: light_pos.clone(),
                })
                .insert(EnvironmentObjectComp)
                .insert(Light::new(
                    colors[rng.gen_range(0..5)]
                        + glm::Vec3::new(
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                            rng.gen_range(0.0..0.2) - 0.1,
                        ),
                    100.,
                    3.,
                ))
                .insert(RenderObject::<SimpleVertex>::new(queue.clone()))
                .insert(Velocity {
                    velocity: dir * 350.,
                    wanted_velocity: dir * 350.,
                    jump_pressed: false,
                });
        }
    }
}
// TODO: Make sure the mouse position is correct after resizing
fn mouse_event_system(
    mut commands: Commands,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_position: ResMut<MousePosition>,
    mut mouse_light_query: Query<(&MouseLight, &mut Position, &mut Light)>,
    // player_query: Query<&Position, With<PlayerLight>>,
) {
    let mp = mouse_position.as_mut();
    for event in cursor_moved_events.iter() {
        mp.position.x = event.position.x;
        mp.position.y = 720. - event.position.y;

        for (mouse_light, mut position, mut light) in &mut mouse_light_query.iter_mut() {
            position.position = mp.position.clone();
            light.polygon = None;
        }
    }
}
