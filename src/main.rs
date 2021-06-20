use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use log::{debug, info, error};
use winit::window::Window;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    mouse_pos: cgmath::Point2<f64>,
    render_pipeline: wgpu::RenderPipeline,
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

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[],  // This is defined in the shader for now
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
            }
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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(
                // [[builtin(vertex_index)]] comes from here
                0..3,
                0..1,
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
