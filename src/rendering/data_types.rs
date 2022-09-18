use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use vulkano::{
    buffer::{BufferContents, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage, CopyImageInfo,
        PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{DescriptorSetsCollection, PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::{ClearColorValue, Format},
    image::{
        view::ImageView, AttachmentImage, ImageAspects, ImageDimensions, ImageLayout, ImageUsage,
        ImageViewAbstract, StorageImage,
    },
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::{BuffersDefinition, Vertex},
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{
        AttachmentDescription, AttachmentReference, Framebuffer, FramebufferCreateInfo, LoadOp,
        RenderPass, RenderPassCreateInfo, StoreOp, Subpass, SubpassDescription,
    },
    sync::GpuFuture,
};

use bevy::{ecs::system::Query, prelude::*, render::texture::ImageFormat};

use nalgebra_glm as glm;

use crate::{
    general::{components::*, data_types::*},
    player::components::*,
    rendering::{components::*, shader_data_types::*},
};

pub struct CameraRes {
    pub(crate) position: glm::Vec2,
}

pub struct RenderImageContainerRes {
    pub(crate) light_images: Vec<Arc<AttachmentImage>>,
}

pub(super) struct RenderPassExecutor {
    pub(super) command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    pub(super) queue: Arc<Queue>,
    pub(super) viewport: Viewport,
}

impl RenderImageContainerRes {
    pub(super) fn new(queue: &Arc<Queue>, dims: &[u32; 2]) -> Self {
        let light_images = (0..3)
            .map(|_| {
                AttachmentImage::with_usage(
                    queue.device().clone(),
                    dims.clone(),
                    Format::R8G8B8A8_UNORM,
                    ImageUsage {
                        storage: true,
                        transfer_dst: true,
                        ..ImageUsage::none()
                    },
                )
                .unwrap()
            })
            // .iter()
            .collect();
        Self { light_images }
    }
}

impl RenderPassExecutor {
    pub(super) fn new(dims: &[u32; 2], queue: Arc<Queue>) -> Self {
        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dims[0] as f32, dims[1] as f32],
            depth_range: 0.0..1.0,
        };

        let builder = AutoCommandBufferBuilder::primary(
            queue.device().clone(),
            queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        Self {
            command_buffer_builder: Some(builder),
            queue,
            viewport,
        }
    }

    pub(super) fn execute<F>(&mut self, future: F) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        future
            .then_execute(
                self.queue.clone(),
                self.command_buffer_builder.take().unwrap().build().unwrap(),
            )
            .unwrap()
            .boxed()
    }
}
