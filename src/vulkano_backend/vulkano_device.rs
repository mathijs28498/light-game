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
    game_object::{
        game_object::{DottedLine, EnvironmentObject, Light, Line, Position, AABB},
        help_functions::{calculate_indices_polygon, get_all_points},
    },
    MousePosition,
};
use nalgebra_glm as glm;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct VertexTest {
    pub position: [f32; 2],
    pub color: [f32; 3],
}
impl_vertex!(VertexTest, position, color);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct LightVertex {
    pub position: [f32; 2],
}
impl_vertex!(LightVertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PushConstants {
    mouse_pos: glm::Vec2,
    resolution: [f32; 2],
    time_passed: f32,
    light_radius: f32,
    light_center: glm::Vec2,
    light_color: glm::Vec3,
    light_brightness: f32,
}

// Create generic for VertexTest
// Add vertices to constructor
#[derive(Component)]
pub struct RenderObject<T>
where
    T: Zeroable + Pod,
    [T]: BufferContents,
{
    vertex_buffer: Option<Arc<ImmutableBuffer<[T]>>>,
    index_buffer: Option<Arc<ImmutableBuffer<[u32]>>>,
}

impl<T> RenderObject<T>
where
    T: Zeroable + Pod,
    [T]: BufferContents,
{
    pub fn new() -> Self {
        Self {
            vertex_buffer: None,
            index_buffer: None,
        }
    }

    pub fn update_vertex_buffer(&mut self, vertices: Vec<T>, queue: Arc<Queue>) {
        let (index_buffer, ib_future)= calculate_index_buffer_polygon(&queue, vertices.len());


        let (vertex_buffer, vb_future) = ImmutableBuffer::from_iter(
            vertices,
            BufferUsage::vertex_buffer(),
            queue,
        ).unwrap();

        // vb_future.
        // TODO: Await futures!!

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }
}

pub fn calculate_index_buffer_polygon(
    queue: &Arc<Queue>,
    amount_of_vertices: usize,
) -> (Arc<ImmutableBuffer<[u32]>>, CommandBufferExecFuture<NowFuture, PrimaryAutoCommandBuffer>) {
    let indices = calculate_indices_polygon(amount_of_vertices - 1);
    ImmutableBuffer::from_iter(
        indices,
        BufferUsage::index_buffer(),
        queue.clone(),
    )
    .unwrap()
}

pub struct RenderPassExecutor {
    pipeline: Arc<GraphicsPipeline>,
    descriptor_set: Option<Arc<PersistentDescriptorSet>>,
    framebuffer: Arc<Framebuffer>,
    command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    viewport: Viewport,
}

impl RenderPassExecutor {
    pub fn new(
        pipeline: Arc<GraphicsPipeline>,
        descriptor_set: Option<Arc<PersistentDescriptorSet>>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        image: Arc<dyn ImageViewAbstract + 'static>,
    ) -> Self {
        let dims = image.image().dimensions().width_height();
        let framebuffer = Framebuffer::new(
            render_pass,
            FramebufferCreateInfo {
                attachments: vec![image],
                ..Default::default()
            },
        )
        .unwrap();

        let builder = AutoCommandBufferBuilder::primary(
            queue.device().clone(),
            queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dims[0] as f32, dims[1] as f32],
            depth_range: 0.0..1.0,
        };

        Self {
            pipeline,
            descriptor_set,
            framebuffer,
            command_buffer_builder: Some(builder),
            viewport,
        }
    }

    pub fn clear_framebuffer_image(&mut self) {
        let fb_image = self.framebuffer.attachments()[0].image();
        self.command_buffer_builder
            .as_mut()
            .unwrap()
            .clear_color_image(ClearColorImageInfo::image(fb_image))
            .unwrap();
    }

    // TODO: Make PushConstants generic
    pub fn do_pass<T>(
        &mut self,
        vertex_buffer: Arc<ImmutableBuffer<[T]>>,
        index_buffer: Arc<ImmutableBuffer<[u32]>>,
        push_constants: Option<PushConstants>,
    ) where
        T: Zeroable + Pod,
        [T]: BufferContents,
    {
        let index_length = index_buffer.len();
        let builder = self.command_buffer_builder.as_mut().unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![None],
                    ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
                },
                SubpassContents::Inline,
            )
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_vertex_buffers(0, vertex_buffer)
            .bind_index_buffer(index_buffer);

        if let Some(pc) = push_constants {
            builder.push_constants(self.pipeline.layout().clone(), 0, pc);
        }

        //                         .push_constants(pipeline.layout().clone(), 0, push_constants)
        //                         .bind_descriptor_sets(
        //                             PipelineBindPoint::Graphics,
        //                             pipeline.layout().clone(),
        //                             0,
        //                             descriptor_sets[image_num].clone(),
        //                         )
        //                         .bind_vertex_buffers(0, vertex_buffer.clone())
        //                         .bind_index_buffer(index_buffer.clone())
        //                         .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
        //                         .unwrap()
        //                         .end_render_pass()
        //                         .unwrap();

        if let Some(descriptor_set) = &self.descriptor_set {
            builder.bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set.clone(),
            );
        }

        builder
            .draw_indexed(index_length as u32, 1, 0, 0, 0)
            .unwrap()
            .end_render_pass()
            .unwrap();
    }

    fn execute<F>(&mut self, queue: Arc<Queue>, before_future: F) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        before_future
            .then_execute(
                queue,
                self.command_buffer_builder.take().unwrap().build().unwrap(),
            )
            .unwrap()
            .boxed()
    }
}

pub struct VulkanoDevice {
    pub queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    descriptor_sets: Vec<Option<Arc<PersistentDescriptorSet>>>,
    // descriptor_sets: Arc<Vec<Arc<PersistentDescriptorSet>>>,
    // event_loop: Option<EventLoop<()>>,
    // surface: Arc<Surface<Window>>,
    // device: Arc<Device>,
    // queue: Arc<Queue>,
    // swapchain: Arc<Swapchain<Window>>,
    // render_pass: Arc<RenderPass>,
    // viewport: Viewport,
    // framebuffers: Vec<Arc<Framebuffer>>,
    // descriptor_sets: Arc<Vec<Arc<PersistentDescriptorSet>>>,
}

impl VulkanoDevice {
    pub fn new<T>(queue: Arc<Queue>, final_output_format: Format) -> Self
    where
        T: Vertex,
    {
        let render_pass = RenderPass::new(
            queue.device().clone(),
            RenderPassCreateInfo {
                attachments: vec![AttachmentDescription {
                    format: Some(final_output_format),
                    // We keep the previous contents of the swapchain image unchanged...
                    load_op: LoadOp::Load,
                    // ...and store the result.
                    store_op: StoreOp::Store,
                    // When acquired, images in the swapchain are in the `Undefined` layout which
                    // must be transitioned into a different one if you want to use the data. You can
                    // use any other layout, but `General` is the only one which works for all purposes.
                    initial_layout: ImageLayout::General,
                    final_layout: ImageLayout::PresentSrc,
                    ..Default::default()
                }],
                subpasses: vec![SubpassDescription {
                    color_attachments: vec![Some(AttachmentReference {
                        attachment: 0,
                        // The only valid image layouts for color attachments are
                        // `ColorAttachmentOptimal` and `General`.
                        layout: ImageLayout::General,
                        ..Default::default()
                    })],
                    input_attachments: vec![Some(AttachmentReference {
                        attachment: 0,
                        // The only valid layouts for input attachments are
                        // `ShaderReadOnlyOptimal` and `General`.
                        layout: ImageLayout::General,
                        aspects: ImageAspects {
                            // We select the color aspect. Not that there is anything else, we will be
                            // binding a swapchain image.
                            color: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    })],
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .unwrap();

        mod vs {
            vulkano_shaders::shader! {
                ty: "vertex",
                path: "src/shaders/basic_pipeline.vert"
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: "src/shaders/basic_pipeline.frag"
            }
        }

        let vs = vs::load(queue.device().clone()).unwrap();
        let fs = fs::load(queue.device().clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .vertex_input_state(BuffersDefinition::new().vertex::<T>())
            .input_assembly_state(InputAssemblyState::new())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(queue.device().clone())
            .unwrap();

        Self {
            queue,
            render_pass,
            pipeline,
            descriptor_sets: Vec::new(),
        }
    }

    pub fn do_pass<F>(
        &mut self,
        before_future: F,
        final_image: Arc<dyn ImageViewAbstract + 'static>,
        light_query: Query<(&RenderObject<LightVertex>, &Position, &Light)>,
        mouse_position: &MousePosition,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        let dims = final_image.image().dimensions().width_height();
        let descriptor_set = PersistentDescriptorSet::new(
            self.pipeline.layout().set_layouts()[0].clone(),
            [WriteDescriptorSet::image_view(
                0,
                ImageView::new_default(final_image.image().clone()).unwrap(),
            )],
        )
        .unwrap();

        let mut executor = RenderPassExecutor::new(
            self.pipeline.clone(),
            Some(descriptor_set.clone()),
            self.queue.clone(),
            self.render_pass.clone(),
            final_image.clone(),
        );

        executor.clear_framebuffer_image();
        for (render_object, position, light) in &light_query {
            if let Some(vertex_buffer) = render_object.vertex_buffer.as_ref() {
                executor.do_pass(
                    vertex_buffer.clone(),
                    render_object.index_buffer.as_ref().unwrap().clone(),
                    Some(PushConstants {
                        mouse_pos: mouse_position.position.clone(),
                        resolution: [dims[0] as f32, dims[1] as f32],
                        time_passed: 0.,
                        light_brightness: light.brightness,
                        light_radius: light.get_radius(),
                        light_center: position.position.clone(),
                        light_color: light.color,
                    }),
                );
            }
        }
        executor.execute(self.queue.clone(), before_future)
    }
}
