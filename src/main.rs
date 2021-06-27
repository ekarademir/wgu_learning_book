use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use log::{debug, info, error};
use winit::window::Window;

#[macro_use]
extern crate bitflags;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ]
        }
    }
}

const VERTICES: &[Vertex] = &[
    // Changed
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397057], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732911], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E
];


const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
    // WGPU requires 4 bytes buffer alignment (packing)
    // Above there are 9 u16 numbers which is 9 x 2 bytes
    // We add one more u16 to square this
    /* padding */ 0,
];

const SECOND_INDICES: &[u16] = &[
    0, 1, 4,
    2, 3, 4,
    // WGPU requires 4 bytes buffer alignment (packing)
    // Above there are 9 u16 numbers which is 9 x 2 bytes
    // We add one more u16 to square this
    /* padding */ 0,
];

bitflags! {
    struct Levers: u32 {
        const LEVER1 = 0b00000001;
        const LEVER2 = 0b00000010;
    }
}


struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    mouse_pos: cgmath::Point2<f64>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    second_index_buffer: wgpu::Buffer,
    second_num_indices: u32,
    levers: Levers,
    diffuse_bind_group: wgpu::BindGroup,
}

impl State {
    async fn new(window: &Window) -> Result<Self, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // instance holds the handle to the GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU  (they are all ORed)
        // TODO: Try BackendBit::VULKAN
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        // This is unsafe because on some Linux systems lifetime of the window might not be as long
        // as the lifetime of the program. See: https://github.com/gfx-rs/wgpu/issues/1463
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            }
        ).await.expect("Can't initialize adapter with the surface.");

        let format = adapter.get_swap_chain_preferred_format(&surface).expect(
            "Can't get surface prefered texture format."
        );

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                // Features are the capabilities of the API and the GPU
                // They are not universal.
                // See all features here: https://docs.rs/wgpu/0.7.0/wgpu/struct.Features.html
                features: wgpu::Features::empty(),
                // Limits are resource limits that can be imposed.
                // They are device dependent
                // See all limits here: https://docs.rs/wgpu/0.7.0/wgpu/struct.Limits.html
                limits: wgpu::Limits::default(),
                label: None,  // Debug label for the device
            },
            None, // Trace path used for tracing API calls if `trace` features is enabled.
        ).await?;

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,  // Framerate will be capped with `VSync` frequency
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let diffuse_bytes = include_bytes!("tree.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes)?;
        let diffuse_rgba = diffuse_image.as_rgba8().expect("Can't transform image info");

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // All textures are stored as 3D, 2D textures have depth of 1.
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                // All textures are stored as 3D, 2D textures have depth of 1.
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                // SAMPLED tells WGPU to use the texture in shaders
                // COPY_DST tells WGPU that we want to copy data to this texture
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                label: Some("diffuse_texture"),
            }
        );

        queue.write_texture(
            // Where to copy the pixel data
            wgpu::ImageCopyTexture {
                texture: &&diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            // The pixel data
            diffuse_rgba,
            // Layout of the texture
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            texture_size
        );

        let diffuse_texture_view = diffuse_texture.create_view(
            &wgpu::TextureViewDescriptor::default()
        );

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float {filterable: true},
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("diffuse_bind_group"),
                layout: &&texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                    }
                ],
            }
        );

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Enabling this requires Features::DEPTH_CLAMPING to be enabled.
                clamp_depth: false,
                // Enabling this requires Features::CONSERVATIVE_RASTERIZATION to be enabled.
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            }
        );

        let num_indices = INDICES.len() as u32;

        let second_index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Second Index Buffer"),
                contents: bytemuck::cast_slice(SECOND_INDICES),
                usage: wgpu::BufferUsage::INDEX,
            }
        );

        let second_num_indices = SECOND_INDICES.len() as u32;

        let levers = Levers::empty();

        Ok(
            Self {
                surface,
                device,
                queue,
                sc_desc,
                swap_chain,
                size,
                mouse_pos: cgmath::Point2 {x: 0.0, y: 0.0},
                render_pipeline,
                vertex_buffer,
                index_buffer,
                second_index_buffer,
                num_indices,
                second_num_indices,
                levers,
                diffuse_bind_group,
            }
        )
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved {position, ..} => {
                self.mouse_pos.x = position.x;
                self.mouse_pos.y = position.y;
                // debug!("Mouse moved to point: {:?}", self.mouse_pos);
                true
            },
            WindowEvent::KeyboardInput { input, .. } => match input {
                KeyboardInput {
                    state,
                    virtual_keycode: Some(VirtualKeyCode::Space),
                    ..
                } => match state {
                    ElementState::Pressed => {
                        self.levers = self.levers | Levers::LEVER1;
                        true
                    },
                    ElementState::Released => {
                        self.levers = self.levers & !Levers::LEVER1;
                        true
                    },
                },
                _ => false
            },
            _ => false
        }
    }

    fn update(&mut self) {
    }

    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain
            .get_current_frame()?
            .output;

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            }
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[
                        // This is what [[location(0)]] in the fragment shader targets
                        wgpu::RenderPassColorAttachment {
                            view: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0,
                                }),
                                store: true,
                            }
                        }
                    ],
                    depth_stencil_attachment: None,
                }
            );

            let data = {
                if self.levers.contains(Levers::LEVER1) {
                    (&self.second_index_buffer, self.second_num_indices)
                } else {
                    (&self.index_buffer, self.num_indices)
                }
            };

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(data.0.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(
                0..data.1,
                0,
                0..1
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    // env_logger::Builder::new()
    //     .filter_module(
    //         "learn_wgpu_book", log::LevelFilter::Debug
    //     )
    //     .init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)?;

    let mut state = futures::executor::block_on(State::new(&window))?;


    event_loop.run(move |event, _, control_flow|
        match event {
            Event::WindowEvent {
                ref event,
                window_id
            } if window_id == window.id() => if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested => *control_flow = handle_exit(ExitReason::CloseRequest),
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = handle_exit(ExitReason::Escape),
                        _ => {}
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size)
                    }
                    WindowEvent::ScaleFactorChanged {new_inner_size, ..} => {
                        // new_inner_size is &&mut so we have to dereference it twice
                        state.resize(**new_inner_size);
                    }

                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                state.update();
                match state.render() {
                    Ok(_) => {},
                    // Recreate the swap chain if lost
                    Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                    // If the system is OOM, we should quit.
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = handle_exit(ExitReason::OOM),
                    // The other swap chain errors will be fixed in the next cycle.
                    Err(e) => error!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    );
}

enum ExitReason {
    Escape,
    CloseRequest,
    OOM,
}

fn handle_exit(why: ExitReason) -> ControlFlow {
    let reason = match why {
        ExitReason::CloseRequest => "Close request received.",
        ExitReason::Escape => "Escape received",
        ExitReason::OOM => "System is OOM",
    };

    debug!("{}", reason);
    info!("Bye");
    ControlFlow::Exit
}
