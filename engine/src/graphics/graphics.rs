use std::{error::Error, iter, rc::Rc};

use wgpu::{
    BindGroupLayout, BlendFactor, BlendOperation, BlendState, CommandBuffer, SwapChainTexture,
};

use rand::Rng;

// main.rs
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::{camera::Camera, camera_control::CameraController, gpu::InstanceRaw, gpu::Uniforms, model, texture::{CpuTexture, TextureHandle}, vertex::SpriteVertex, vertex::Vertex};
use crate::{graphics::model::DrawModel, logic::{geom::*, types::*}};

const NUM_MARBLES: i32 = 10;

pub enum GraphicalDisplay {
    Gpu(State),
}

pub enum GraphicsMethod {
    OpenGL,
    WGPUDefault,
}

pub struct State {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub clear_color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
    pub camera: Camera,
    pub camera_controller: CameraController,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    //Area for actual models like marbles & walls.
    marbles: Vec<Marble>,
    walls: Vec<Wall>,
    marble_model: model::Model,
    wall_model: model::Model,
    g: f32,
    #[allow(dead_code)]
    marbles_buffer: wgpu::Buffer,
    walls_buffer: wgpu::Buffer,
    texture_bind_group_layout: BindGroupLayout,
    depth_texture: TextureHandle,
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window, render_mode: GraphicsMethod) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let backend = match render_mode {
            GraphicsMethod::WGPUDefault => wgpu::BackendBit::PRIMARY,
            GraphicsMethod::OpenGL => wgpu::BackendBit::GL,
        };
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
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
            });

        let depth_texture = TextureHandle::create_depth_texture(&device, &sc_desc, "depth_texture");
        let camera = Camera {
            eye: (0.0, 5.0, -10.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 200.0,
        };

        let camera = Camera {
            eye: (0.0, 5.0, -10.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: sc_desc.width as f32 / sc_desc.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 200.0,
        };

        let camera_controller = CameraController::new(0.2);

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        //Marbles and Walls
        let wall = Wall {
            body: Plane {
                n: Vec3::new(0.0, 1.0, 0.0),
                d: 0.0,
            },
            distructable:false,
        };
        let walls = vec![wall];

        let mut rng = rand::thread_rng();
        let marbles = (0..NUM_MARBLES)
            .map(move |_x| {
                let x = rng.gen_range(-5.0, 5.0);
                let y = rng.gen_range(1.0, 5.0);
                let z = rng.gen_range(-5.0, 5.0);
                let r = rng.gen_range(0.1, 1.0);
                Marble {
                    body: Sphere {
                        c: Pos3::new(x, y, z),
                        r,
                    },
                    velocity: Vec3::zero(),
                }
            })
            .collect::<Vec<_>>();

        let marbles_data = marbles.iter().map(Marble::to_raw).collect::<Vec<_>>();
        let marbles_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Marbles Buffer"),
            contents: bytemuck::cast_slice(&marbles_data),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let wall_data = vec![wall.to_raw()];
        let walls_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Walls Buffer"),
            contents: bytemuck::cast_slice(&wall_data),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        //changed.
        let res_dir = std::path::Path::new("content");
        let wall_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout,
            res_dir.join("floor.obj"),
        )
        .unwrap();
        let marble_model = model::Model::load(
            &device,
            &queue,
            &texture_bind_group_layout, // It's shaded the same as the floor
            res_dir.join("sphere.obj"),
        )
        .unwrap();

        // Shaders
        let shader_module =
            device.create_shader_module(&wgpu::include_spirv!(env!("sprite_shader.spv")));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "main_vs",
                buffers: &[SpriteVertex::desc(), InstanceRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "main_fs",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    alpha_blend: BlendState {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    color_blend: BlendState {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: wgpu::CullMode::Back,
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: TextureHandle::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
                // Setting this to true requires Features::DEPTH_CLAMPING
                clamp_depth: false,
            }),
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
        });

        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            size,
            clear_color: wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            camera,
            camera_controller,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            depth_texture,
            marbles,
            walls,
            marbles_buffer,
            walls_buffer,
            marble_model,
            wall_model,
            g: 10.0,
           
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.depth_texture =
            TextureHandle::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
    }

    pub fn update(&mut self) {
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        // Update buffers based on dynamics
        self.queue.write_buffer(
            &self.walls_buffer,
            0,
            bytemuck::cast_slice(&vec![self.walls[0].to_raw()]),
        );
        // TODO avoid reallocating every frame
        let marbles_data = self.marbles.iter().map(Marble::to_raw).collect::<Vec<_>>();
        self.queue
            .write_buffer(&self.marbles_buffer, 0, bytemuck::cast_slice(&marbles_data));
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );

        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_vertex_buffer(1, self.marbles_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.marble_model,
                0..self.marbles.len() as u32,
                &self.uniform_bind_group,
            );
            render_pass.set_vertex_buffer(1, self.walls_buffer.slice(..));
            render_pass.draw_model_instanced(&self.wall_model, 0..1, &self.uniform_bind_group);
        }

        self.queue.submit(iter::once(encoder.finish()));

        Ok(())
    }

    pub fn clear_screen(
        &mut self,
    ) -> Result<(CommandBuffer, SwapChainTexture), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screen Clear Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen Clear Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
        }
        Ok((encoder.finish(), frame))
    }

    pub fn recreate_swapchain(&mut self) {
        self.resize(self.size);
    }
}
