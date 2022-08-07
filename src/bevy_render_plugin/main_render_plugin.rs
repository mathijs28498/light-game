use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::view::window,
    window::WindowId,
};
use bevy_vulkano::{BevyVulkanoWindows, PipelineSyncData};
use vulkano::{image::ImageAccess, sync::GpuFuture};

use crate::vulkano_backend::vulkano_device::{MyVertex, RenderObject, VertexTest, VulkanoDevice};

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    GuiInit,
    GuiDefine,
    RenderStart,
    Render,
    RenderFinish,
}

pub struct MainRenderPlugin;

impl Plugin for MainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(insert_render_pass_system)
            .add_stage_after(
                CoreStage::PostUpdate,
                RenderStage::GuiInit,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::GuiInit,
                RenderStage::GuiDefine,
                SystemStage::parallel(),
            )
            .add_stage_after(
                RenderStage::GuiDefine,
                RenderStage::RenderStart,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::RenderStart,
                RenderStage::Render,
                SystemStage::single_threaded(),
            )
            .add_stage_after(
                RenderStage::Render,
                RenderStage::RenderFinish,
                SystemStage::single_threaded(),
            )
            // Render systems
            .add_system_set_to_stage(
                RenderStage::RenderStart,
                SystemSet::new().with_system(pre_render_setup_system),
            )
            .add_system_set_to_stage(
                RenderStage::Render,
                SystemSet::new().with_system(main_render_system),
            )
            .add_system_set_to_stage(
                RenderStage::RenderFinish,
                SystemSet::new().with_system(post_render_system),
            );
    }
}

/// Insert our render pass at startup
fn insert_render_pass_system(mut commands: Commands, vulkano_windows: NonSend<BevyVulkanoWindows>) {
    let window_renderer = vulkano_windows.get_primary_window_renderer().unwrap();
    let queue = window_renderer.graphics_queue();
    let format = window_renderer.swapchain_format();

    let vertices = vec![
        MyVertex {
            position: [-0.0, -0.0],
        },
        MyVertex {
            position: [-0.5, -0.5],
        },
        MyVertex {
            position: [0.5, -0.5],
        },
        MyVertex {
            position: [0.5, 0.5],
        },
        MyVertex {
            position: [-0.5, 0.5],
        },
    ];
    let render_object = RenderObject::new(&queue, vertices);
    commands.spawn().insert(render_object);

    let vertices = vec![
        MyVertex {
            position: [-0.0, -0.0],
        },
        MyVertex {
            position: [-0.75, -0.75],
        },
        MyVertex {
            position: [0.8, -0.2],
        },
        MyVertex {
            position: [0.1, 0.3],
        },
        MyVertex {
            position: [-0.2, 0.7],
        },
    ];
    let render_object = RenderObject::new(&queue, vertices);
    commands.spawn().insert(render_object);

    let vulkano_device = VulkanoDevice::new::<MyVertex>(queue, format);
    commands.insert_resource(vulkano_device);
}

fn pre_render_setup_system(
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

fn post_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
) {
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

// Only draw primary now...
// You could render different windows in their own systems...
pub fn main_render_system(
    mut vulkano_windows: NonSendMut<BevyVulkanoWindows>,
    diagnostics: Res<Diagnostics>,
    mut pipeline_frame_data: ResMut<PipelineSyncData>,
    mut vulkano_device: ResMut<VulkanoDevice>,
    render_object_query: Query<&RenderObject<MyVertex>, With<RenderObject<MyVertex>>>,
) {
    if let Some(diag) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(avg) = diag.average() {
            println!("{:.2} fps ({:.2} ms/frame)", avg, 1. / avg * 1000.);
        }
    }
    let mut frame_data = pipeline_frame_data.get_mut(WindowId::primary()).unwrap();
    let window_renderer =
        if let Some(window_renderer) = vulkano_windows.get_primary_window_renderer_mut() {
            window_renderer
        } else {
            return;
        };

    // We take the before pipeline future leaving None in its place
    if let Some(before_future) = frame_data.before.take() {
        let final_image_view = window_renderer.swapchain_image_view();
        let mut after_future: Box<dyn GpuFuture> = vulkano_device.do_pass(
            before_future,
            window_renderer.swapchain_image_view(),
            render_object_query,
        );

        let after_drawing = after_future.then_signal_fence_and_flush().unwrap().boxed();
        // Update after pipeline future (so post render will know to present frame)
        frame_data.after = Some(after_drawing);
    }
}
