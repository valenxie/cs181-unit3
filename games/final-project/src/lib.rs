use cgmath::prelude::*;
use rand;
use std::{f32::consts::PI, iter};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use engine::logic::{collision,geom::*};
use engine::graphics::{camera::*,texture,vertex,gpu::Uniforms,graphics::GpuState};




const NUM_MARBLES: usize = 50;

const DT: f32 = 1.0 / 60.0;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    #[allow(dead_code)]
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Marble {
    pub body: Sphere,
    pub velocity: Vec3,
}

impl Marble {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from_translation(self.body.c.to_vec()) * Mat4::from_scale(self.body.r))
                .into(),
        }
    }
    fn update(&mut self, g: f32) {
        self.velocity += Vec3::new(0.0, -g * (1.0+(self.body.r - 0.1)*0.5), 0.0) * DT;
        self.body.c += self.velocity * DT;
    }

    fn mass(&self,density:f32) -> f32{
        //V=4/3pi*r^3
        let volume = (4.0/3.0)*PI*(self.body.r*self.body.r*self.body.r);
        volume*density
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wall {
    pub body: Plane,
    control: (i8, i8),
}

impl Wall {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Mat4::from(cgmath::Quaternion::between_vectors(
                Vec3::new(0.0, 1.0, 0.0),
                self.body.n,
            )) * Mat4::from_translation(Vec3::new(0.0, -0.025, 0.0))
                * Mat4::from_nonuniform_scale(0.5, 0.05, 0.5))
            .into(),
        }
    }
    fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    VirtualKeyCode::W => {
                        self.control.1 = if is_pressed { -1 } else { 0 };
                        true
                    }
                    VirtualKeyCode::S => {
                        self.control.1 = if is_pressed { 1 } else { 0 };
                        true
                    }
                    VirtualKeyCode::A => {
                        self.control.0 = if is_pressed { -1 } else { 0 };
                        true
                    }
                    VirtualKeyCode::D => {
                        self.control.0 = if is_pressed { 1 } else { 0 };
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
    fn update(&mut self) {
        self.body.n += Vec3::new(
            self.control.0 as f32 * 0.4 * DT,
            0.0,
            self.control.1 as f32 * 0.4 * DT,
        );
        self.body.n = self.body.n.normalize();
    }
}


fn main() {
    use std::time::Instant;
    env_logger::init();
    let event_loop = EventLoop::new();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();
    use futures::executor::block_on;
    let mut gpu_state = block_on(GpuState::new(&window,engine::graphics::graphics::GraphicsMethod::WGPUDefault));
    //let mut state = block_on(State::new(&window));

    // How many frames have we simulated?
    #[allow(unused_variables)]
    let mut frame_count: usize = 0;
    // How many unsimulated frames have we saved up?
    let mut available_time: f32 = 0.0;
    let mut since = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !gpu_state.camera_controller.process_events(event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        },
                        WindowEvent::Resized(physical_size) => {
                            gpu_state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            gpu_state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(_) => {
                gpu_state.update();
                match gpu_state.render() {
                    Ok(_) => {}
                    // Recreate the swap_chain if lost
                    Err(wgpu::SwapChainError::Lost) => gpu_state.resize(gpu_state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
                // The renderer "produces" time...
                available_time += since.elapsed().as_secs_f32();
                since = Instant::now();
            }
            _ => {}
        }
        // And the simulation "consumes" it
        while available_time >= DT {
            // Eat up one frame worth of time
            available_time -= DT;

            gpu_state.update();

            // Increment the frame counter
            frame_count += 1;
        }
    });
}
