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
    environment::components::*,
    general::{components::*, data_types::*},
    player::components::*,
    rendering::{components::*, data_types::*, functions::*, shader_data_types::*},
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

    let light_render_pipeline = LightRenderPipeline::new(queue.clone(), format.clone());
    commands.insert_resource(light_render_pipeline);

    let image_render_pipeline = ImageRenderPipeline::new(queue, format);
    commands.insert_resource(image_render_pipeline);
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
        frame_data.after = match window_renderer.acquire() {
            Err(e) => {
                bevy::log::error!("Failed to start frame: {}", e);
                None
            }
            Ok(f) => Some(f),
        };
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
    mut light_render_pipeline: ResMut<LightRenderPipeline>,
    mut image_render_pipeline: ResMut<ImageRenderPipeline>,
    light_query: Query<(&RenderObject<LightVertex>, &PositionComp, &LightComp)>,
    image_query: Query<&RenderObject<ImageVertex>>,
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
    if let Some(mut after_future) = frame_data.after.take() {
        after_future = light_render_pipeline.do_pass(
            after_future,
            window_renderer.swapchain_image_view(),
            window_renderer.image_index(),
            light_query,
            &mouse_position,
        );

        after_future = image_render_pipeline.do_pass(
            after_future,
            window_renderer.swapchain_image_view(),
            window_renderer.image_index(),
            image_query,
            &mouse_position,
        );

        let after_drawing = after_future.then_signal_fence_and_flush().unwrap().boxed();
        // Update after pipeline future (so post render will know to present frame)
        frame_data.after = Some(after_drawing);
    }
}

pub(crate) fn update_light_polygons_system(
    light_render_pipeline: Res<LightRenderPipeline>,
    mut light_query: Query<(
        &mut RenderObject<LightVertex>,
        &PositionComp,
        &mut LightComp,
    )>,
    env_object_query: Query<&AABBComp, With<EnvironmentObjectComp>>,
) {
    for (mut render_object, position, mut light) in &mut light_query.iter_mut() {
        let (light_polygon, recalculate) =
            light.calculate_light_polygon(&position, &env_object_query);
        if !recalculate {
            continue;
        }
        let light_vertices = light_polygon.iter().map(|p| LightVertex {
            position: [p.x, p.y],
        });

        let mut vertices = vec![LightVertex { position: [0., 0.] }];
        vertices.extend(light_vertices);
        while vertices.len() < 3 {
            vertices.push(LightVertex { position: [0., 0.] });
        }

        render_object.update_vertex_buffer_light(vertices, light_render_pipeline.queue.clone());

        // let indices = calculate_indices_polygon(vertices.len() - 1);
    }
}
