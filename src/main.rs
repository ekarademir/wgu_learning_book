use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use log::{debug, info};
use winit::window::Window;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
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

        Ok(
            Self {
                surface,
                device,
                queue,
                sc_desc,
                swap_chain,
                size,
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
        false
    }

    fn update(&mut self) {
    }

    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        todo!()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

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

            _ => {}
        }
    );
}

enum ExitReason {
    Escape,
    CloseRequest,
}

fn handle_exit(why: ExitReason) -> ControlFlow {
    match why {
        ExitReason::CloseRequest => debug!("Close request received."),
        ExitReason::Escape => debug!("Escape received"),
    }

    info!("Bye");
    ControlFlow::Exit
}
