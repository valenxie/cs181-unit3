use std::{error::Error, rc::Rc};

use pixels::{Pixels};
use wgpu::{BindGroupLayout, BlendFactor, BlendOperation, BlendState, CommandBuffer, SwapChainTexture};


// main.rs
use winit::window::Window;
use wgpu::util::DeviceExt;

use super::{camera::Camera, gpu::Instance, gpu::Uniforms, sprite::{DrawSprite, Sprite}, texture::{CpuTexture, TextureHandle}, tiles::{DrawTilemap, Tilemap, TilemapHandle}, vertex::SpriteVertex, vertex::Vertex};


pub enum GraphicalDisplay {
    Cpu(Pixels),
    Gpu(GpuState),
}

pub enum GraphicsMethod {
    Cpu,
    OpenGL,
    WGPUDefault,
}

pub struct GpuState {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub clear_color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
    pub camera: Camera,
    uniforms: Uniforms,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: BindGroupLayout,
    depth_texture: TextureHandle,
    pub sprites: Vec<Sprite>,
    pub tilemap: Option<TilemapHandle>,
}

impl GpuState {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window, render_mode: GraphicsMethod) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let backend = match render_mode {
            GraphicsMethod::Cpu => panic!("WGPU does not support render option: CPU"),
            GraphicsMethod::WGPUDefault => wgpu::BackendBit::PRIMARY,
            GraphicsMethod::OpenGL => wgpu::BackendBit::GL,
        };
        let instance = wgpu::Instance::new(backend);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let depth_texture = TextureHandle::create_depth_texture(&device, &sc_desc, "depth_texture");
        let camera = Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            pos: [0.0,0.0]
        };

        let mut uniforms = Uniforms::new();
        uniforms.update_view_proj(&camera);

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }
        );
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("uniform_bind_group_layout"),
        });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("uniform_bind_group"),
        });
        // Shaders
        let vs_module = device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.vert.spv"));
        let fs_module = device.create_shader_module(&wgpu::include_spirv!("../shaders/shader.frag.spv"));
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
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
            }
        );
        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }
        );
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[SpriteVertex::desc(), Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
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
                stencil: wgpu::StencilState::default(), // 2.
                bias: wgpu::DepthBiasState::default(),
                // Setting this to true requires Features::DEPTH_CLAMPING
                clamp_depth: false,
            }),
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
        });

        let sprites = Vec::new();
        let tilemap = None;
        Self {
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            render_pipeline,
            size,
            clear_color: wgpu::Color {r:0.0, g:0.0, b:0.0, a:1.0},
            camera,
            uniforms,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group_layout,
            depth_texture,
            sprites,
            tilemap,
        }
    }
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
        self.depth_texture = TextureHandle::create_depth_texture(&self.device, &self.sc_desc, "depth_texture");
    }

    pub fn update(&mut self) {
        self.uniforms.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniforms]));
    }

    pub fn render_sprites(&mut self, frame: &SwapChainTexture) -> Result<CommandBuffer, wgpu::SwapChainError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Sprite Render Encoder"),
        });
        {
            // 1.
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sprite Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: false,
                        }
                    }
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
            for sprite in &self.sprites {
                render_pass.draw_sprite_instanced(sprite, &self.uniform_bind_group, 0..sprite.num_elements as u32);
            }
        }
        // submit will accept anything that implements IntoIter
        Ok(encoder.finish())
    }

    pub fn render_tiles(&mut self, frame: &SwapChainTexture) -> Result<CommandBuffer, wgpu::SwapChainError> {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tile Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Tile Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: false,
                        }
                    }
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
            match &self.tilemap {
                Some(tilemap) => {
                    render_pass.draw_tilemap(&tilemap, &self.uniform_bind_group);
                },
                None => {},
            }
        }
        // submit will accept anything that implements IntoIter
        Ok(encoder.finish())
    }

    pub fn clear_screen(&mut self) -> Result<(CommandBuffer, SwapChainTexture), wgpu::SwapChainError> {
        let frame = self
            .swap_chain
            .get_current_frame()?
            .output;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Screen Clear Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen Clear Render Pass"),
                color_attachments: &[
                    wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(self.clear_color),
                            store: true,
                        }
                    }
                ],
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

    pub fn load_sprite(
        &mut self,
        texture: Rc<CpuTexture>,
    ) {
        self.sprites.push(Sprite::load(&self.device, &self.queue, &self.texture_bind_group_layout, texture).unwrap());
    }

    pub fn load_tilemap(
        &mut self,
        tilemap: &mut Tilemap
    ) -> Result<(), Box<dyn Error>> {
        self.tilemap = Some(tilemap.load(&self.device, &self.queue, &self.texture_bind_group_layout)?);
        Ok(())
    }

    pub fn recreate_swapchain(&mut self) {
        self.resize(self.size);
    }
}