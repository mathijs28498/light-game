use bytemuck::{Pod, Zeroable};
use std::{
    sync::Arc,
    time::{Duration, SystemTime}, alloc::System,
    cmp::max,
};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage, RenderPassBeginInfo,
        SubpassContents,
    },
    descriptor_set::{DescriptorSet, PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    image::ImageAspects,
    image::{view::ImageView, ImageAccess, ImageLayout, ImageUsage, SwapchainImage},
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
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use rand::Rng;
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::game_object::{
    game_object::{DottedLine, EnvironmentObject, Light, Line, AABB},
    help_functions::{calculate_indices_polygon, get_all_points},
};
use nalgebra_glm as glm;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
    position: [f32; 2],
}
impl_vertex!(Vertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct PushConstants {
    mouse_pos: glm::Vec2,
    resolution: [f32; 2],
    dimensions: [f32; 2],
    light_center: glm::Vec2,
    light_color: glm::Vec3,
    light_brightness: f32,
    light_radius: f32,
    time_passed: f32,
}

pub struct VulkanoDevice {
    event_loop: Option<EventLoop<()>>,
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,
    framebuffers: Vec<Arc<Framebuffer>>,
    descriptor_sets: Arc<Vec<Arc<PersistentDescriptorSet>>>,
}

impl VulkanoDevice {
    pub fn new_with_initialization() -> VulkanoDevice {
        let required_extensions = vulkano_win::required_extensions();

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
                    .find(|&q| {
                        q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false)
                    })
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

        // TODO: Look into triangle strip in assembly_state
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

        let (mut framebuffers, mut descriptor_sets) =
            window_size_dependent_setup(&images, render_pass.clone(), &pipeline, &mut viewport);

        VulkanoDevice {
            event_loop: Some(event_loop),
            surface,
            device,
            queue,
            swapchain,
            render_pass,
            pipeline,
            viewport,
            framebuffers,
            descriptor_sets,
        }
    }

    pub fn run(&mut self) {
        let mut recreate_swapchain = false;
        let mut previous_frame_end = Some(sync::now(self.device.clone()).boxed());
        let surface = self.surface.clone();
        let mut device = self.device.clone();
        let mut queue = self.queue.clone();
        let mut swapchain = self.swapchain.clone();
        let mut render_pass = self.render_pass.clone();
        let mut pipeline = self.pipeline.clone();
        let mut viewport = self.viewport.clone();
        let mut framebuffers = self.framebuffers.clone();
        let mut descriptor_sets = self.descriptor_sets.clone();

        let mut mouse_pos = glm::Vec2::new(0., 0.);
        let mut time_passed = 0.;
        let dimensions = surface.window().inner_size();
        let sb_offset = -1.;

        // TODO: add debug to game objects as well
        let mut env_objects: Vec<Box<dyn EnvironmentObject>> = vec![
            Box::new(AABB::new(
                glm::Vec2::new(sb_offset, sb_offset),
                glm::Vec2::new(
                    dimensions.width as f32 - sb_offset,
                    dimensions.height as f32 - sb_offset,
                ),
            )),
            Box::new(Line::new(
                glm::Vec2::new(200., 100.),
                glm::Vec2::new(400., 100.),
            )),
            Box::new(Line::new(
                glm::Vec2::new(100., 150.),
                glm::Vec2::new(800., 150.),
            )),
        ];

        let start_p = glm::Vec2::new(100., 300.);
        let end_p = glm::Vec2::new(700., 400.);
        let gap_amount = 30;

        env_objects.push(Box::new(DottedLine::new(
            start_p,
            end_p,
            gap_amount,
        )));

        let start_p = glm::Vec2::new(100., 400.);
        let end_p = glm::Vec2::new(700., 500.);
        env_objects.push(Box::new(DottedLine::new(
            start_p,
            end_p,
            gap_amount,
        )));

        let mut lights = vec![
            Light::new(glm::Vec3::new(0.2, 0.1, 0.7), mouse_pos.clone(), 300., 3.),
        ];

        let aol_delta = 5;
        let mut amount_of_lights = 20;
        let mut last_aol_up = SystemTime::now();
        let mut last_aol_down = SystemTime::now();
        let mut last_updated_lights = SystemTime::now();
        let max_update_aol_millis = 150;
        let max_update_lights_millis = 1500;
        generate_random_lights(&mut lights, amount_of_lights);

        let event_loop = std::mem::replace(&mut self.event_loop, None);
        if let Some(el) = event_loop {
            el.run(move |event, _, control_flow| match event {
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
                    mouse_pos = glm::Vec2::new(position.x as f32, position.y as f32);
                    (&mut lights[0]).set_center(mouse_pos);
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::R),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    if last_updated_lights.elapsed().unwrap().as_millis() > max_update_lights_millis {
                        generate_random_lights(&mut lights, amount_of_lights);
                        last_updated_lights = SystemTime::now();
                    }
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Up),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    if last_aol_up.elapsed().unwrap().as_millis() > max_update_aol_millis {
                        amount_of_lights += aol_delta;
                        println!("{}", amount_of_lights);
                        last_aol_up = SystemTime::now();
                    }
                }
                Event::WindowEvent {
                    event:
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Down),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    if last_aol_down.elapsed().unwrap().as_millis() > max_update_aol_millis {
                        amount_of_lights = max(0, amount_of_lights - aol_delta);
                        println!("{}", amount_of_lights);
                        last_aol_down = SystemTime::now();
                    }
                }
                Event::RedrawEventsCleared => {
                    let dimensions = surface.window().inner_size();
                    if dimensions.width == 0 || dimensions.height == 0 {
                        return;
                    }
                    previous_frame_end.as_mut().unwrap().cleanup_finished();
                    if recreate_swapchain {
                        let (new_swapchain, new_images) =
                            match swapchain.recreate(SwapchainCreateInfo {
                                image_extent: dimensions.into(),
                                ..swapchain.create_info()
                            }) {
                                Ok(r) => r,
                                Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => {
                                    return
                                }
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
                    let dim = surface.window().inner_size();
                    let res = [dim.width as f32, dim.height as f32];

                    builder
                        .clear_color_image(ClearColorImageInfo::image(fb_image).clone())
                        .unwrap();

                    for light in lights.iter_mut() {
                        let (vertices, indices) = create_vertices(&env_objects, light);

                        let vertex_buffer = CpuAccessibleBuffer::from_iter(
                            device.clone(),
                            BufferUsage::all(),
                            false,
                            vertices,
                        )
                        .unwrap();

                        let index_buffer = CpuAccessibleBuffer::from_iter(
                            device.clone(),
                            BufferUsage::all(),
                            false,
                            indices,
                        )
                        .unwrap();

                        let push_constants = PushConstants {
                            mouse_pos: mouse_pos.into(),
                            resolution: res,
                            dimensions: [dimensions.width as f32, dimensions.height as f32],
                            light_center: light.get_center().clone(),
                            light_color: light.color.clone(),
                            light_brightness: light.brightness,
                            light_radius: light.get_radius(),
                            time_passed,
                        };

                        builder
                            .begin_render_pass(
                                RenderPassBeginInfo {
                                    clear_values: vec![None],
                                    ..RenderPassBeginInfo::framebuffer(
                                        framebuffers[image_num].clone(),
                                    )
                                },
                                SubpassContents::Inline,
                            )
                            .unwrap()
                            .set_viewport(0, [viewport.clone()])
                            .bind_pipeline_graphics(pipeline.clone())
                            .push_constants(pipeline.layout().clone(), 0, push_constants)
                            .bind_descriptor_sets(
                                PipelineBindPoint::Graphics,
                                pipeline.layout().clone(),
                                0,
                                descriptor_sets[image_num].clone(),
                            )
                            .bind_vertex_buffers(0, vertex_buffer.clone())
                            .bind_index_buffer(index_buffer.clone())
                            .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                            .unwrap()
                            .end_render_pass()
                            .unwrap();
                    }

                    time_passed += 0.01;

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
    }
}

// TODO: Draw a concave polygon more efficiently
fn create_vertices(
    env_objects: &Vec<Box<dyn EnvironmentObject>>,
    light: &mut Light,
) -> (Vec<Vertex>, Vec<u32>) {
    let light_polygon = light.calculate_light_polygon(env_objects);
    let light_vertices = light_polygon.iter().map(|p| Vertex {
        position: [p.x, p.y],
    });

    let mut vertices = vec![Vertex {
        position: [light.get_center().x, light.get_center().y],
    }];
    vertices.extend(light_vertices);
    while vertices.len() < 3 {
        vertices.push(Vertex { position: [0., 0.] });
    }

    let indices = calculate_indices_polygon(vertices.len() - 1);

    (vertices, indices)
}

fn generate_random_lights(lights: &mut Vec<Light>, amount_of_lights: usize) {
    if lights.len() > 1 {
        lights.drain(1..);
    }

    let colors = vec![
        glm::Vec3::new(0.85, 0.33, 0.04),
        glm::Vec3::new(0.23, 0.85, 0.09),
        glm::Vec3::new(0.85, 0.06, 0.2),
        glm::Vec3::new(0.09, 0.7, 0.7),
        glm::Vec3::new(0.85, 0.06, 0.06),
    ];

    let positions = vec![
        glm::Vec2::new(101., 101.),
        glm::Vec2::new(351., 101.),
        glm::Vec2::new(701., 101.),
        glm::Vec2::new(101., 351.),
        glm::Vec2::new(351., 351.),
        glm::Vec2::new(701., 351.),
        glm::Vec2::new(101., 551.),
        glm::Vec2::new(351., 551.),
        glm::Vec2::new(701., 551.),
    ];
    
    let mut rng = rand::thread_rng();
    for i in 0..amount_of_lights {
        let color_offset = glm::Vec3::new(
            rng.gen_range(0.0..0.2) - 0.1,
            rng.gen_range(0.0..0.2) - 0.1,
            rng.gen_range(0.0..0.2) - 0.1,
        );
        lights.push(
            Light::new(
                colors[i % colors.len()] + color_offset,
                glm::Vec2::new(rng.gen_range(31.0..771.0), rng.gen_range(31.0..571.0)),
                rng.gen_range(100.0..200.0),
                // 100.0,
                rng.gen_range(0.2..0.8)
            )
        );
    }
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    pipeline: &Arc<GraphicsPipeline>,
    viewport: &mut Viewport,
) -> (
    Vec<Arc<Framebuffer>>,
    Arc<Vec<Arc<PersistentDescriptorSet>>>,
) {
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

    (framebuffers, Arc::new(descriptor_sets))
}
