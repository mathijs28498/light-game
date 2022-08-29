use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    input::mouse::{self, MouseMotion},
    prelude::*,
    render::view::window,
    window::WindowId,
};
use bevy_vulkano::{BevyVulkanoWindows, PipelineSyncData};
use vulkano::{image::ImageAccess, sync::GpuFuture};

use crate::{
    environment_objects::components::*,
    general::components::*,
    player::components::*,
    rendering::{components::*, functions::*, shader_data_types::*, data_types::*},
    MousePosition,
};

use rand::Rng;
use std::sync::Arc;

use vulkano::device::Queue;

use nalgebra_glm as glm;

pub(super) fn insert_render_pass_system(
    mut commands: Commands,
    vulkano_windows: NonSend<BevyVulkanoWindows>,
) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();
    let format = window_renderer.swapchain_format();

    let vulkano_device = VulkanoDevice::new::<LightVertex>(queue, format);
    commands.insert_resource(vulkano_device);
}

pub(super) fn pre_render_setup_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
) {
    for (window_id, mut frame_data) in pipeline_frame_data.data_per_window.iter_mut() {
        let window_renderer =
            if let Some(window_renderer) = vulkano_windows.get_window_renderer_mut(*window_id) {
                window_renderer
            } else {
                return;
            };
        let before = match window_renderer.acquire() {
            Err(e) => {
                bevy::log::error!("Failed to start frame: {}", e);
                None
            }
            Ok(f) => Some(f),
        };
        frame_data.before = before;
    }
}

pub(super) fn post_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
    diagnostics: Res<Diagnostics>,
) {
    if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(avg) = diag.average() {
            let primary = vulkano_windows
                .get_winit_window(WindowId::primary())
                .unwrap();
            primary.set_title(&format!(
                "Light game {:.2} fps ({:.2} ms/frame)",
                avg,
                1. / avg * 1000.
            ));
        }
    }

    for (window_id, frame_data) in pipeline_frame_data.data_per_window.iter_mut() {
        let window_renderer =
            if let Some(window_renderer) = vulkano_windows.get_window_renderer_mut(*window_id) {
                window_renderer
            } else {
                return;
            };
        if let Some(after) = frame_data.after.take() {
            window_renderer.present(after, false);
        }
    }
}

pub(super) fn main_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
    mut vulkano_device: ResMut<VulkanoDevice>,
    light_query: Query<(&RenderObject<LightVertex>, &PositionComp, &LightComp)>,
    env_object_query: Query<(&EnvironmentObjectComp, &AABBComp)>,
    mouse_position: Res<MousePosition>,
) {
    let mut frame_data = pipeline_frame_data.get_mut(WindowId::primary()).unwrap();
    let window_renderer =
        if let Some(window_renderer) = vulkano_windows.get_primary_window_renderer_mut() {
            window_renderer
        } else {
            return;
        };

    // We take the before pipeline future leaving None in its place
    if let Some(before_future) = frame_data.before.take() {
        let mut after_future: Box<dyn GpuFuture> = vulkano_device.do_pass(
            before_future,
            window_renderer.swapchain_image_view(),
            light_query,
            &mouse_position,
        );

        let after_drawing = after_future.then_signal_fence_and_flush().unwrap().boxed();
        // Update after pipeline future (so post render will know to present frame)
        frame_data.after = Some(after_drawing);
    }
}

pub(crate) fn update_light_polygons_system(
    vulkano_device: Res<VulkanoDevice>,
    mut light_query: Query<(&mut RenderObject<LightVertex>, &PositionComp, &mut LightComp)>,
    env_object_query: Query<&AABBComp, With<EnvironmentObjectComp>>,
) {
    for (mut render_object, position, mut light) in &mut light_query.iter_mut() {
        let (light_polygon, recalculate) = light.calculate_light_polygon(&position, &env_object_query);
        if !recalculate {
            continue;
        }
        let light_vertices = light_polygon.iter().map(|p| LightVertex {
            position: [p.x, p.y],
        });

        let mut vertices = vec![LightVertex {
            position: [0., 0.],
        }];
        vertices.extend(light_vertices);
        while vertices.len() < 3 {
            vertices.push(LightVertex { position: [0., 0.] });
        }

        render_object.update_vertex_buffer(vertices, vulkano_device.queue.clone());

        // let indices = calculate_indices_polygon(vertices.len() - 1);
    }
}