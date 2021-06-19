use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)?;

    event_loop.run(move |event, _, control_flow|
        match event {
            Event::WindowEvent {
                ref event,
                window_id
            } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = handle_exit(ExitReason::CloseRequest),
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    } => *control_flow = handle_exit(ExitReason::Escape),
                    _ => {}
                }

                _ => {}
            }

            _ => {}
        }
    );

    Ok(())
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
