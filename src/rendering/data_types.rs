use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use vulkano::{
    buffer::{BufferContents, ImmutableBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage,
        PrimaryAutoCommandBuffer, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::Format,
    image::{view::ImageView, ImageAspects, ImageLayout, ImageViewAbstract},
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

use bevy::ecs::system::Query;

use crate::{
    general::{components::*, data_types::*},
    player::components::*,
    rendering::{components::*, shader_data_types::*},
};

pub(super) struct RenderPassExecutor {
    pipeline: Arc<GraphicsPipeline>,
    descriptor_set: Option<Arc<PersistentDescriptorSet>>,
    framebuffer: Arc<Framebuffer>,
    command_buffer_builder: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    viewport: Viewport,
}

pub struct LightRenderPipeline {
    pub(crate) queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    descriptor_sets: Vec<Option<Arc<PersistentDescriptorSet>>>,
}

impl RenderPassExecutor {
    pub(super) fn new(
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

    pub(super) fn clear_framebuffer_image(&mut self) {
        let fb_image = self.framebuffer.attachments()[0].image();
        self.command_buffer_builder
            .as_mut()
            .unwrap()
            .clear_color_image(ClearColorImageInfo::image(fb_image))
            .unwrap();
    }

    // TODO: Make PushConstants generic
    pub(super) fn do_pass<T>(
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

    pub(super) fn execute<F>(&mut self, queue: Arc<Queue>, before_future: F) -> Box<dyn GpuFuture>
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

impl LightRenderPipeline {
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
            .vertex_input_state(BuffersDefinition::new().vertex::<LightVertex>())
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
        }
    }

    pub(crate) fn do_pass<F>(
        &mut self,
        before_future: F,
        final_image: Arc<dyn ImageViewAbstract + 'static>,
        image_index: usize,
        light_query: Query<(&RenderObject<LightVertex>, &PositionComp, &LightComp)>,
        mouse_position: &MousePosition,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        let dims = final_image.image().dimensions().width_height();

        let descriptor_set = match &self.descriptor_sets[image_index] {
            Some(ds) => ds.clone(),
            None => {
                let ds = PersistentDescriptorSet::new(
                    self.pipeline.layout().set_layouts()[0].clone(),
                    [WriteDescriptorSet::image_view(
                        0,
                        ImageView::new_default(final_image.image().clone()).unwrap(),
                    )],
                )
                .unwrap();
                self.descriptor_sets[image_index] = Some(ds.clone());
                ds
            }
        };

        let mut executor = RenderPassExecutor::new(
            self.pipeline.clone(),
            Some(descriptor_set),
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
