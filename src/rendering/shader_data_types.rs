use bevy::prelude::*;
use bytemuck::{Pod, Zeroable};
use rand::{seq::index, Rng};
use std::{
    alloc::System,
    cmp::max,
    sync::Arc,
    time::{Duration, SystemTime},
};
use vulkano::{
    buffer::{
        BufferContents, BufferUsage, ImmutableBuffer, TypedBufferAccess,
    },
    command_buffer::{
        AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage,
        PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents, CommandBufferExecFuture
    },
    descriptor_set::{DescriptorSet, PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    format::Format,
    image::ImageAspects,
    image::ImageViewAbstract,
    image::{view::ImageView, ImageAccess, ImageLayout, ImageUsage, SwapchainImage},
    impl_vertex,
    instance::{Instance, InstanceCreateInfo},
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
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture, NowFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    general::{data_types::*, functions::*},
    environment::components::*,
    rendering::{components::*, functions::*},
};
use nalgebra_glm as glm;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct CreatureVertex {
    pub position: [f32; 2],
}
impl_vertex!(CreatureVertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct LightVertex {
    pub position: [f32; 2],
}
impl_vertex!(LightVertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct BloomVertex {
    pub position: [f32; 2],
}
impl_vertex!(BloomVertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct LightPushConstants {
    pub(super) mouse_pos: glm::Vec2,
    pub(super) resolution: [f32; 2],
    pub(super) time_passed: f32,
    pub(super) light_radius: f32,
    pub(super) light_center: glm::Vec2,
    pub(super) light_color: glm::Vec3,
    pub(super) light_brightness: f32,
    pub(super) camera_position: glm::Vec2,
}

pub(super) struct CreaturePushConstants {
    pub(super) resolution: [f32; 2],
    pub(super) model_center: glm::Vec2,
    pub(super) model_color: glm::Vec3,
    pub(super) padding: f32,
    pub(super) camera_position: glm::Vec2,
}