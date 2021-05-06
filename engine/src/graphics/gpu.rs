use super::camera::Camera;
use cgmath::*;
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

impl Uniforms {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = (OPENGL_TO_WGPU_MATRIX * camera.build_view_projection_matrix()).into();
    }
}

pub const GAME_WIDTH: f32 = 480.0;
pub const GAME_HEIGHT: f32 = 320.0;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    position_offset: [f32; 3],
    position_scale: [f32; 2],
    texture_offset: [f32; 2],
    texture_scale: [f32; 2]
}

impl Instance {
    pub fn new(
        position_offset: [f32; 3],
        position_scale: [f32; 2],
        texture_offset: [f32; 2],
        texture_scale: [f32; 2]
    ) -> Self {
        Instance {
            position_offset,
            position_scale,
            texture_offset,
            texture_scale
        }
    }

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        const INSTANCELAYOUT : wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Instance>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::InputStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![2 => Float3, 3 => Float2, 4 => Float2, 5 => Float2],
        };
        INSTANCELAYOUT
    }
}