use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo,
    },
    image::{view::ImageView, ImageAccess, ImageUsage, SwapchainImage},
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
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, Surface, Swapchain, SwapchainCreateInfo,
        SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
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
    position: [f32; 3],
}
impl_vertex!(Vertex, position);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct PushConstants {
    mouse_pos: glm::Vec2,
    resolution: [f32; 2],
    dimensions: [f32; 2],
    time_passed: f32,
    amount_of_lights: u32,
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
                    image_usage: ImageUsage::color_attachment(),
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

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
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

        let mut framebuffers =
            window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);

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
            start_p.clone(),
            end_p.clone(),
            gap_amount,
        )));

        let start_p1 = glm::Vec2::new(100., 200.);
        let end_p1 = glm::Vec2::new(700., 300.);
        env_objects.push(Box::new(DottedLine::new(
            start_p1.clone(),
            end_p1.clone(),
            gap_amount,
        )));

        // let start_p2 = glm::Vec2::new(200., 100.);
        // let end_p2 = glm::Vec2::new(300., 700.);
        // env_objects.push(Box::new(DottedLine::new(
        //     start_p2.clone(),
        //     end_p2.clone(),
        //     gap_amount,
        // )));

        // let start_p3 = glm::Vec2::new(300., 100.);
        // let end_p3 = glm::Vec2::new(400., 700.);
        // env_objects.push(Box::new(DottedLine::new(
        //     start_p3.clone(),
        //     end_p3.clone(),
        //     gap_amount,
        // )));

        // let size = (end_p - start_p).magnitude() / (gap_amount as f32 * 2. + 1.);
        // for i in 0..gap_amount {
        //     let i_f32 = i as f32;
        //     let offset = i_f32 * size * 2.;
        //     let offset_0 = glm::Vec2::new(offset, 0.);
        //     let offset_1 = glm::Vec2::new(offset + size, 0.);
        //     env_objects.push(Box::new(Line::new(
        //         start_p + offset_0,
        //         start_p + offset_1,
        //     )));
        // }
        // env_objects.push(
        //     Box::new(Line::new(
        //         end_p - glm::Vec2::new(size, 0.),
        //         end_p,
        //     ))
        // );
        
        let mut lights = vec![
            Light::new(glm::Vec3::new(0.2, 0.1, 0.7), mouse_pos.clone(), 300., 6.),
            Light::new(glm::Vec3::new(0.5, 0.7, 0.1), glm::Vec2::new(100., 500.), 200., 3.),
        ];
        // let mut light = Light::new(glm::Vec3::new(0.2, 0.1, 0.7), mouse_pos.clone(), 300., 6.);

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
                    (&mut lights[0]).center = mouse_pos;
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
                        framebuffers = window_size_dependent_setup(
                            &new_images,
                            render_pass.clone(),
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


                    let mut vertices = Vec::new();
                    let mut indices = Vec::new();
                    let mut index_offset = 0;

                    let (vertices_local, indices_local ) = create_vertices(&env_objects, &lights[0], 0, index_offset);

                    index_offset = vertices_local.len() as u32;
                    vertices.extend(vertices_local);
                    indices.extend(indices_local);

                    let (vertices_local, indices_local) = create_vertices(&env_objects, &lights[1], 0, index_offset);

                    vertices.extend(vertices_local);
                    indices.extend(indices_local);

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

                    let dim = surface.window().inner_size();
                    let res = [dim.width as f32, dim.height as f32];

                    let push_constants = PushConstants {
                        mouse_pos: mouse_pos.into(),
                        resolution: res,
                        dimensions: [dimensions.width as f32, dimensions.height as f32],
                        time_passed,
                        amount_of_lights: lights.len() as u32,
                    };
                    time_passed += 0.01;

                    let light_buffer = {
                        let data_iter = lights.iter().map(|l| l.get_buffer_data());
                        
                        CpuAccessibleBuffer::from_iter(
                            device.clone(),
                            BufferUsage::all(),
                            false,
                            data_iter,
                        )
                        .unwrap()
                    };

                    let set = PersistentDescriptorSet::new(
                        pipeline.layout().set_layouts().get(0).unwrap().clone(),
                        [WriteDescriptorSet::buffer(0, light_buffer)],
                    )
                    .unwrap();

                    builder
                        .begin_render_pass(
                            RenderPassBeginInfo {
                                clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                                ..RenderPassBeginInfo::framebuffer(framebuffers[image_num].clone())
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
                            set,
                        )
                        .bind_vertex_buffers(0, vertex_buffer.clone())
                        .bind_index_buffer(index_buffer.clone())
                        .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
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
    }
}

// TODO: Draw a concave polygon more efficiently
fn create_vertices(
    env_objects: &Vec<Box<dyn EnvironmentObject>>,
    light: &Light,
    light_index: usize,
    index_offset: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let pos_z = light_index as f32 + 1.;
    let light_polygon = light.calculate_light_polygon(env_objects);
    let light_vertices = light_polygon.iter().map(|p| Vertex {
        position: [p.x, p.y, pos_z],
    });

    let mut vertices = vec![Vertex {
        position: [light.center.x, light.center.y, pos_z],
    }];
    vertices.extend(light_vertices);
    while vertices.len() < 3 {
        vertices.push(Vertex { position: [0., 0., pos_z] });
    }

    let indices = calculate_indices_polygon(vertices.len() - 1, index_offset);

    (vertices, indices)
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
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
        .collect::<Vec<_>>()
}
