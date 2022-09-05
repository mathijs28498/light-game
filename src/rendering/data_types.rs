use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use vulkano::{
    buffer::{BufferContents, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage,
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
    single_pass_renderpass,
    sync::GpuFuture,
};

use bevy::{prelude::*, ecs::system::Query, render::texture::ImageFormat};

use nalgebra_glm as glm;

use crate::{
    general::{components::*, data_types::*},
    player::components::*,
    rendering::{components::*, shader_data_types::*},
};

// TODO
// [ ] - Make a resource that contains all images and descriptors
// [ ] - Make sure that the CreatureRenderPipeline uses the new images
// [ ] - Make system to draw the lights
// [ ] - Make bloom system by using multiple images in different resolutions (these don't need to be global)
// [ ] - 
// [ ] - 
// [ ] - 


pub struct CameraRes {
    pub(crate) position: glm::Vec2,
}

pub struct ClearFramebufferPipeline {
    pub(crate) queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pub(super) framebuffers: Vec<Option<Arc<Framebuffer>>>,
}

pub struct LightRenderPipeline {
    pub(crate) queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    images: Vec<Arc<AttachmentImage>>,
    descriptor_sets: Vec<Arc<PersistentDescriptorSet>>,
    framebuffers: Vec<Option<Arc<Framebuffer>>>,
}

pub struct CreatureRenderPipeline {
    pub(crate) queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    descriptor_sets: Vec<Option<Arc<PersistentDescriptorSet>>>,
    framebuffers: Vec<Option<Arc<Framebuffer>>>,
}

pub(super) struct RenderPassExecutor {
    pub(super) command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    queue: Arc<Queue>,
    pub(super) viewport: Viewport,
}

impl ClearFramebufferPipeline {
    pub(crate) fn new(queue: Arc<Queue>, image_format: Format) -> Self {
        let render_pass = single_pass_renderpass!(
            queue.device().clone(),
            attachments: {
                color: {
                    load: DontCare,
                    store: DontCare,
                    format: image_format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        Self {
            queue,
            render_pass,
            framebuffers: vec![None, None, None],
        }
    }

    pub(crate) fn do_pass<F>(
        &mut self,
        future: F,
        fb_image: Arc<dyn ImageViewAbstract + 'static>,
        fb_image_index: usize,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        // Get the descriptor set/framebuffer in constructor
        let dims = fb_image.image().dimensions().width_height();

        let framebuffer = match &self.framebuffers[fb_image_index] {
            Some(fb) => fb.clone(),
            None => {
                let fb = Framebuffer::new(
                    self.render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![fb_image.clone()],
                        ..Default::default()
                    },
                )
                .unwrap();
                self.framebuffers[fb_image_index] = Some(fb.clone());
                fb
            }
        };

        let mut executor = RenderPassExecutor::new(&dims, self.queue.clone());
        let mut builder = executor.command_buffer_builder.as_mut().unwrap();

        builder
            .clear_color_image(ClearColorImageInfo::image(
                framebuffer.attachments()[0].image(),
            ))
            .unwrap();
        executor.execute(future)
    }
}

impl LightRenderPipeline {
    pub(crate) fn new(queue: Arc<Queue>, image_format: Format, dims: &[u32; 2]) -> Self {
        // TODO: Change to load: DontCare - store: DontCare
        let render_pass = single_pass_renderpass!(
            queue.device().clone(),
            attachments: {
                color: {
                    load: Load,
                    store: Store,
                    format: image_format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        mod vs {
            vulkano_shaders::shader! {
                ty: "vertex",
                path: "src/shaders/light_pipeline.vert"
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: "src/shaders/light_pipeline.frag"
            }
        }

        let vs = vs::load(queue.device().clone()).unwrap();
        let fs = fs::load(queue.device().clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .vertex_input_state(BuffersDefinition::new().vertex::<LightVertex>())
            .input_assembly_state(InputAssemblyState::new())
            .vertex_shader(vs.entry_point("main").unwrap(), ())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(fs.entry_point("main").unwrap(), ())
            .build(queue.device().clone())
            .unwrap();

        let mut images = Vec::new();
        let mut descriptor_sets = Vec::new();

        for i in 0..3 {
            let image = AttachmentImage::with_usage(
                queue.device().clone(),
                dims.clone(),
                Format::R8G8B8A8_UNORM,
                ImageUsage {
                    storage: true,
                    transfer_dst: true,
                    ..ImageUsage::none()
                },
            )
            .unwrap();

            let descriptor_set = PersistentDescriptorSet::new(
                pipeline.layout().set_layouts()[0].clone(),
                [WriteDescriptorSet::image_view(
                    0,
                    ImageView::new_default(image.clone()).unwrap(),
                )],
            )
            .unwrap();

            images.push(image);
            descriptor_sets.push(descriptor_set);
        }

        Self {
            queue,
            render_pass,
            pipeline,
            images,
            descriptor_sets,
            framebuffers: vec![None, None, None],
        }
    }

    pub(crate) fn do_pass<F>(
        &mut self,
        before_future: F,
        fb_image_index: usize,
        dims: &[u32; 2],
        final_image: Arc<dyn ImageViewAbstract + 'static>,
        light_query: Query<(&RenderObjectComp<LightVertex>, &PositionComp, &LightComp)>,
        mouse_position: &MousePosition,
        camera: &CameraRes,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        let image = self.images[fb_image_index].clone();
        let descriptor_set = self.descriptor_sets[fb_image_index].clone();
        let framebuffer = Framebuffer::new(self.render_pass.clone(), FramebufferCreateInfo {
            attachments: vec![final_image],
            ..Default::default()
        }).unwrap();

        let mut executor = RenderPassExecutor::new(&dims, self.queue.clone());
        let mut builder = executor.command_buffer_builder.as_mut().unwrap();

        builder
            .clear_color_image(ClearColorImageInfo {
                clear_value: ClearColorValue::Float([0.0, 0.0, 0.0, 0.0]),
                ..ClearColorImageInfo::image(image)
            })
            .expect("Failed to clear color image");

        for (render_object, position, light) in &light_query {
            if let Some(vertex_buffer) = render_object.vertex_buffer.as_ref() {
                let index_buffer = render_object.index_buffer.as_ref().unwrap().clone();
                let index_length = index_buffer.len();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![None],
                            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                        },
                        SubpassContents::Inline,
                    )
                    .unwrap()
                    .set_viewport(0, [executor.viewport.clone()])
                    .bind_pipeline_graphics(self.pipeline.clone())
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .bind_index_buffer(index_buffer)
                    .push_constants(
                        self.pipeline.layout().clone(),
                        0,
                        LightPushConstants {
                            mouse_pos: mouse_position.position.clone(),
                            resolution: [dims[0] as f32, dims[1] as f32],
                            time_passed: 0.,
                            light_brightness: light.brightness,
                            light_radius: light.get_radius(),
                            light_center: position.position.clone(),
                            light_color: light.color,
                            camera_position: camera.position.clone(),
                        },
                    )
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.pipeline.layout().clone(),
                        0,
                        descriptor_set.clone(),
                    )
                    .draw_indexed(index_length as u32, 1, 0, 0, 0)
                    .unwrap()
                    .end_render_pass()
                    .unwrap();
            }
        }
        executor.execute(before_future)
    }
}

impl CreatureRenderPipeline {
    pub(crate) fn new(queue: Arc<Queue>, image_format: Format) -> Self {
        let render_pass = RenderPass::new(
            queue.device().clone(),
            RenderPassCreateInfo {
                attachments: vec![AttachmentDescription {
                    format: Some(image_format),
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
                path: "src/shaders/creature_pipeline.vert"
            }
        }

        mod fs {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: "src/shaders/creature_pipeline.frag"
            }
        }

        let vs = vs::load(queue.device().clone()).unwrap();
        let fs = fs::load(queue.device().clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .vertex_input_state(BuffersDefinition::new().vertex::<CreatureVertex>())
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
            descriptor_sets: vec![None, None, None],
            framebuffers: vec![None, None, None],
        }
    }

    pub(crate) fn do_pass<F>(
        &mut self,
        before_future: F,
        image: Arc<dyn ImageViewAbstract + 'static>,
        image_index: usize,
        image_query: Query<(
            &RenderObjectComp<CreatureVertex>,
            &PositionComp,
            &CreatureComp,
        )>,
        mouse_position: &MousePosition,
        camera: &CameraRes,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        // Get the descriptor set/framebuffer in constructor
        let dims = image.image().dimensions().width_height();

        let descriptor_set = match &self.descriptor_sets[image_index] {
            Some(ds) => ds.clone(),
            None => {
                let ds = PersistentDescriptorSet::new(
                    self.pipeline.layout().set_layouts()[0].clone(),
                    [WriteDescriptorSet::image_view(
                        0,
                        ImageView::new_default(image.image().clone()).unwrap(),
                    )],
                )
                .unwrap();
                self.descriptor_sets[image_index] = Some(ds.clone());
                ds
            }
        };

        let framebuffer = match &self.framebuffers[image_index] {
            Some(fb) => fb.clone(),
            None => {
                let fb = Framebuffer::new(
                    self.render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![image.clone()],
                        ..Default::default()
                    },
                )
                .unwrap();
                self.framebuffers[image_index] = Some(fb.clone());
                fb
            }
        };

        let mut executor = RenderPassExecutor::new(&dims, self.queue.clone());
        let mut builder = executor.command_buffer_builder.as_mut().unwrap();

        for (render_object, position, creature) in &image_query {
            if let Some(vertex_buffer) = render_object.vertex_buffer.as_ref() {
                let index_buffer = render_object.index_buffer.as_ref().unwrap().clone();
                let index_length = index_buffer.len();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![None],
                            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                        },
                        SubpassContents::Inline,
                    )
                    .unwrap()
                    .set_viewport(0, [executor.viewport.clone()])
                    .bind_pipeline_graphics(self.pipeline.clone())
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .bind_index_buffer(index_buffer)
                    .push_constants(
                        self.pipeline.layout().clone(),
                        0,
                        CreaturePushConstants {
                            resolution: [dims[0] as f32, dims[1] as f32],
                            model_center: position.position.clone(),
                            model_color: creature.color.clone(),
                            padding: 0.,
                            camera_position: camera.position.clone(),
                        },
                    )
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        self.pipeline.layout().clone(),
                        0,
                        descriptor_set.clone(),
                    )
                    .draw_indexed(index_length as u32, 1, 0, 0, 0)
                    .unwrap()
                    .end_render_pass()
                    .unwrap();
            }
        }
        executor.execute(before_future)
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
