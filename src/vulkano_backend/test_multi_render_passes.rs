use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents, ClearColorImageInfo
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    image::{view::ImageView, ImageAccess, ImageAspects, ImageLayout, ImageUsage, SwapchainImage},
    impl_vertex,
    instance::{Instance, InstanceCreateInfo},
    pipeline::{
        graphics::{
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{
        AttachmentDescription, AttachmentReference, Framebuffer, FramebufferCreateInfo, LoadOp,
        RenderPass, RenderPassCreateInfo, StoreOp, Subpass, SubpassDescription,
    },
    swapchain::{
        acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub fn multi_main() {
    // let library = VulkanLibrary::new().unwrap();
    // let required_extensions = vulkano_win::required_extensions(&library);
    let required_extensions = vulkano_win::required_extensions();

    // let instance = Instance::new(
    //     library,
    //     InstanceCreateInfo {
    //         enabled_extensions: required_extensions,
    //         ..Default::default()
    //     },
    // )
    // .unwrap();

    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        enumerate_portability: true,
        ..Default::default()
    })
    .unwrap();

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .expect("No suitable physical device found");

    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
            ..Default::default()
        },
    )
    .unwrap();

    let queue = queues.next().unwrap();

    let (mut swapchain, images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,
                image_format,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage {
                    input_attachment: true,
                    transfer_dst: true,
                    ..ImageUsage::color_attachment()
                },
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
    };

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 4],
    }
    impl_vertex!(Vertex, position, color);

    let vertices = [
        Vertex {
            position: [-0.5, -0.25],
            color: [1., 0., 0., 1.],
        },
        Vertex {
            position: [0.0, 0.5],
            color: [1., 0., 0., 1.],
        },
        Vertex {
            position: [0.25, -0.1],
            color: [1., 0., 0., 1.],
        },
    ];
    let vertex_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices)
            .unwrap();

    let vertices = [
        Vertex {
            position: [-0.25, -0.5],
            color: [0., 0., 1., 1.],
        },
        Vertex {
            position: [0.5, 0.0],
            color: [0., 0., 1., 1.],
        },
        Vertex {
            position: [-0.1, 0.25],
            color: [0., 0., 1., 1.],
        },
    ];
    let vertex_buffer2 =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices)
            .unwrap();
            
    mod vs {
        vulkano_shaders::shader! {
            ty: "vertex",
            src: "
                #version 450

                layout(location = 0) in vec2 position;
                layout(location = 1) in vec4 color;

                layout(location = 0) out vec4 f_in_color;

                void main() {
                    gl_Position = vec4(position, 0.0, 1.0);
                    f_in_color = color;
                }
            "
        }
    }

    mod fs {
        vulkano_shaders::shader! {
            ty: "fragment",
            src: "
                #version 450

                layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput input_attachment;
                layout(location = 0) in vec4 f_in_color;
                layout(location = 0) out vec4 f_color;

                void main() {
                    vec4 input_col = subpassLoad(input_attachment);
                    if (f_in_color.w < 0.) {
                        f_color = vec4(f_in_color.xyz, 1.);
                    } else {
                        f_color = input_col + f_in_color;
                    }
                }
            "
        }
    }

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    let render_pass = RenderPass::new(
        device.clone(),
        RenderPassCreateInfo {
            attachments: vec![AttachmentDescription {
                format: Some(swapchain.image_format()),
                // We keep the previous contents of the swapchain image unchanged...
                load_op: LoadOp::Load,
                // ...and store the result.
                store_op: StoreOp::Store,
                // When acquired, images in the swapchain are in the `Undefined` layout which
                // must be transitioned into a different one if you want to use the data. You can
                // use any other layout, but `General` is the only one which works for all purposes.
                initial_layout: ImageLayout::General,
                // The only valid image layout for presenting a swapchain image to the surface is
                // `PresentSrc`.
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

    let pipeline = GraphicsPipeline::start()
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
        .input_assembly_state(InputAssemblyState::new())
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .build(device.clone())
        .unwrap();

    let mut viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [0.0, 0.0],
        depth_range: 0.0..1.0,
    };

    // Our input attachment is a swapchain image, which is why we create the descriptor set for it
    // inside `window_size_dependent_setup`.
    let (mut framebuffers, mut descriptor_sets) =
        window_size_dependent_setup(&images, render_pass.clone(), &pipeline, &mut viewport);

    let mut recreate_swapchain = false;
    let mut draw = false;

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    let mut mouse_pos = [0., 0.];

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }

        Event::WindowEvent {
            event: WindowEvent::CursorMoved { position, .. },
            ..
        } => {
            mouse_pos = [position.x as f32, position.y as f32];
        }
        Event::RedrawEventsCleared => {
            let dimensions = surface.window().inner_size();
            if dimensions.width == 0 || dimensions.height == 0 {
                return;
            }

            let screen_mouse_pos = [
                mouse_pos[0] / dimensions.width as f32 * 2. - 1.,
                mouse_pos[1] / dimensions.height as f32 * 2. - 1.,
                ];

            let vertices = [
                Vertex {
                    position: [-0.5, -0.25],
                    color: [0., 1., 0., 1.],
                },
                Vertex {
                    position: [-0.25, -0.5],
                    color: [0., 1., 0., 1.],
                },
                Vertex {
                    position: screen_mouse_pos.clone(),
                    color: [0., 1., 0., 1.],
                },
            ];
            let vertex_buffer3 =
                CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices)
                    .unwrap();

            previous_frame_end.as_mut().unwrap().cleanup_finished();

            if recreate_swapchain {
                let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                    image_extent: dimensions.into(),
                    ..swapchain.create_info()
                }) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

                swapchain = new_swapchain;
                (framebuffers, descriptor_sets) = window_size_dependent_setup(
                    &new_images,
                    render_pass.clone(),
                    &pipeline,
                    &mut viewport,
                );
                recreate_swapchain = false;
            }

            let (image_num, suboptimal, acquire_future) =
                match acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

            if suboptimal {
                recreate_swapchain = true;
            }

            let mut builder = AutoCommandBufferBuilder::primary(
                device.clone(),
                queue.family(),
                CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();

            let fb_image = framebuffers[image_num].attachments()[0].image();
            let mut clear_color_image_info = ClearColorImageInfo::image(fb_image).clone();
            clear_color_image_info.image_layout = ImageLayout::General;

            builder
                .clear_color_image(clear_color_image_info)
                .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        // The clear value is `None`, because our renderpass was created with
                        // `LoadOp::Load` so we don't want to clear it.
                        clear_values: vec![None],
                        ..RenderPassBeginInfo::framebuffer(framebuffers[image_num].clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .set_viewport(0, [viewport.clone()])
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer.clone())
                // We bind the descriptor set with the swapchain image as our input attachment.
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_sets[image_num].clone(),
                )
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        // The clear value is `None`, because our renderpass was created with
                        // `LoadOp::Load` so we don't want to clear it.
                        clear_values: vec![None],
                        ..RenderPassBeginInfo::framebuffer(framebuffers[image_num].clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .set_viewport(0, [viewport.clone()])
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer2.clone())
                // We bind the descriptor set with the swapchain image as our input attachment.
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_sets[image_num].clone(),
                )
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        // The clear value is `None`, because our renderpass was created with
                        // `LoadOp::Load` so we don't want to clear it.
                        clear_values: vec![None],
                        ..RenderPassBeginInfo::framebuffer(framebuffers[image_num].clone())
                    },
                    SubpassContents::Inline,
                )
                .unwrap()
                .set_viewport(0, [viewport.clone()])
                .bind_pipeline_graphics(pipeline.clone())
                .bind_vertex_buffers(0, vertex_buffer3.clone())
                // We bind the descriptor set with the swapchain image as our input attachment.
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    pipeline.layout().clone(),
                    0,
                    descriptor_sets[image_num].clone(),
                )
                .draw(vertex_buffer.len() as u32, 1, 0, 0)
                .unwrap()
                .end_render_pass()
                .unwrap();

            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
                .take()
                .unwrap()
                .join(acquire_future)
                .then_execute(queue.clone(), command_buffer)
                .unwrap()
                .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                .then_signal_fence_and_flush();

            match future {
                Ok(future) => {
                    previous_frame_end = Some(future.boxed());
                }
                Err(FlushError::OutOfDate) => {
                    recreate_swapchain = true;
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    previous_frame_end = Some(sync::now(device.clone()).boxed());
                }
            }
        }
        _ => (),
    });
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    pipeline: &Arc<GraphicsPipeline>,
    viewport: &mut Viewport,
) -> (Vec<Arc<Framebuffer>>, Vec<Arc<PersistentDescriptorSet>>) {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // As there are multiple swapchain images, we need to create one descriptor set for each and
    // select the corrent one for the frame when drawing it to it. The descriptor set tells the
    // fragment shader which image to use as the input attachment.
    let descriptor_sets = images
        .iter()
        .map(|image| {
            PersistentDescriptorSet::new(
                pipeline.layout().set_layouts()[0].clone(),
                [WriteDescriptorSet::image_view(
                    0,
                    ImageView::new_default(image.clone()).unwrap(),
                )],
            )
            .unwrap()
        })
        .collect();

    (framebuffers, descriptor_sets)
}
