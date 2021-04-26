use std::{error::Error};
use wgpu::SwapChainError;

use winit::{event::WindowEvent, platform::run_return::EventLoopExtRunReturn};
use winit::window::WindowBuilder;
use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};
use winit_input_helper::WinitInputHelper;

use graphics::{graphics::{GpuState, GraphicalDisplay, GraphicsMethod}, resources::Resources};

pub mod graphics;
pub mod logic;
pub mod audio;

const DT: f64 = 1.0 / 60.0;

pub fn run<Rule, State>(
    _screen_width: usize,
    _screen_height: usize,
    window_builder: WindowBuilder,
    rsrc: Resources,
    mut rules: Rule,
    mut state: State,
    graphics_method: GraphicsMethod,
    init: impl Fn(&Resources, &mut Rule, &mut GraphicalDisplay, &State) -> Result<(), Box<dyn Error>> + 'static,
    draw: impl Fn(&Resources, &Rule, &State, &mut GraphicalDisplay, usize) -> Result<(), SwapChainError> + 'static,
    update: impl Fn(&mut Rule, &mut State, &WinitInputHelper, usize) -> bool + 'static,
) {
    use std::time::Instant;

    let mut event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = window_builder.build(&event_loop).unwrap();
    use futures::executor::block_on;

    // Since main can't be async, we're going to need to block
    let mut render_target = match graphics_method {
        _ => GraphicalDisplay::Gpu(block_on(GpuState::new(&window, graphics_method))),
    };
    
    init(&rsrc, &mut rules, &mut render_target, &state).unwrap();
    // How many frames have we simulated?
    let mut frame_count: usize = 0;
    // How many unsimulated frames have we saved up?
    let mut available_time = 0.0;
    let mut since = Instant::now();
    event_loop.run_return(|event, _, control_flow| {
        match event {
            // Handle window events
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match &mut render_target {
                    GraphicalDisplay::Gpu(gpu_state) => {
                        match event {
                            WindowEvent::Resized(physical_size) => {
                                gpu_state.resize(*physical_size);
                            },
                            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                                gpu_state.resize(**new_inner_size);
                            },
                            _ => {},
                        }
                    }
                }
            },
            // Draw new frame
            Event::RedrawRequested(_) => {
                match (draw(&rsrc, &rules, &state, &mut render_target, frame_count), &mut render_target) {
                    (Ok(_), _) => {}
                    // Recreate the swap_chain if lost
                    (Err(wgpu::SwapChainError::Lost), GraphicalDisplay::Gpu(gpu_state)) => gpu_state.recreate_swapchain(),
                    // The system is out of memory, we should probably quit
                    (Err(wgpu::SwapChainError::OutOfMemory), GraphicalDisplay::Gpu(_gpu_state)) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    (Err(e), _) => eprintln!("{:?}", e),
                }
                available_time += since.elapsed().as_secs_f64();
            },
            _ => {}
        }
        // Handle input events
        if input.update(&event) {
            // Close events
            if input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }
    
        // And the simulation "consumes" it
        while available_time >= DT {
            // Eat up one frame worth of time
            available_time -= DT;

            // Exit if update says to quit
            if update(&mut rules, &mut state, &input, frame_count) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Increment the frame counter
            frame_count += 1;
        }
        // Request redraw
        window.request_redraw();
        // When did the last frame end?
        since = Instant::now();
    });
}
