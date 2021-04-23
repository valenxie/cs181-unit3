use std::{error::Error, mem, ops::Range, rc::Rc};
use wgpu::{BindGroup, CommandBuffer, util::DeviceExt};
use super::{gpu::{Instance, GAME_HEIGHT, GAME_WIDTH}, graphics::GpuState, texture::{CpuTexture, Dimensions, Material, TextureHandle}, vertex::{SpriteVertex}};

use crate::logic::types::Vec2i;

use super::animation::{AnimationState};

const VERTICES: &[SpriteVertex] = &[
    SpriteVertex { position: [0.0, 0.0, 0.0], tex_pos: [0.0, 0.0]},
    SpriteVertex { position: [1.0, 0.0, 0.0], tex_pos: [1.0, 0.0]},
    SpriteVertex { position: [1.0, 1.0, 0.0], tex_pos: [1.0, 1.0]},
    SpriteVertex { position: [0.0, 1.0, 0.0], tex_pos: [0.0, 1.0]},
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0
];

pub struct Sprite {
    pub materials: Vec<Material>,
    pub dimensions: Dimensions,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Sprite {
    pub fn load(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        texture: Rc<CpuTexture>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut materials = Vec::new();
        let dimensions = texture.size();
        let dimensions = (dimensions.0 as u32, dimensions.1 as u32);
        let (diffuse_texture, dimensions) = TextureHandle::from_bytes(device, queue, texture.buffer(), dimensions, 
                                                                    "Sprite Texture")?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(Material {
            name: "Sprite Material".to_string(),
            diffuse_texture,
            bind_group,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", "Sprite")),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", "Sprite")),
                contents: bytemuck::cast_slice(&INDICES),
                usage: wgpu::BufferUsage::INDEX,
            }
        );

        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Sprite Instance Buffer"),
                contents: &[],
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            }
        );

        Ok(Self {materials, index_buffer, vertex_buffer, instance_buffer,
            dimensions, num_elements: 0 })
    }
}

pub trait DrawSprite<'a, 'b>
where
    'b: 'a,
{
    fn draw_sprite(&mut self, sprite: &'b Sprite, uniforms: &'b BindGroup);
    fn draw_sprite_instanced(
        &mut self,
        sprite: &'b Sprite,
        uniforms: &'b BindGroup,
        instances: Range<u32>,
    );
}
impl<'a, 'b> DrawSprite<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_sprite(&mut self, sprite: &'b Sprite, uniforms: &'b BindGroup,) {
        self.draw_sprite_instanced(sprite, uniforms, 0..1);
    }

    fn draw_sprite_instanced(
        &mut self,
        sprite: &'b Sprite,
        uniforms: &'b BindGroup,
        instances: Range<u32>,
    ){
        self.set_vertex_buffer(0, sprite.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, sprite.instance_buffer.slice(..));
        self.set_index_buffer(sprite.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.set_bind_group(0, &sprite.materials[0].bind_group, &[]);
        self.set_bind_group(1, uniforms, &[]);
        self.draw_indexed(0..INDICES.len() as u32, 0, instances);
    }
}

pub trait UpdateSprite<'a, 'b>
where
    'b: 'a,
{
    fn update_sprite_instances(
        &mut self,
        sprite: usize,
        positions: &[Vec2i],
        anim_states: &[AnimationState],
    ) -> Result<CommandBuffer, wgpu::SwapChainError> ;
}
impl<'a, 'b> UpdateSprite<'a, 'b> for GpuState
where
    'b: 'a,
{
    fn update_sprite_instances(&mut self, sprite_id: usize, positions: &[Vec2i], anim_states: &[AnimationState]) -> Result<CommandBuffer, wgpu::SwapChainError> {
        assert!(positions.len() == anim_states.len());
        let sprite = &mut self.sprites[sprite_id];
        let reuse_buffer = sprite.num_elements >= positions.len() as u32;
        let instances = positions.iter().zip(anim_states.iter()).map(|(p, a)| {
            let frame = a.frame();
            let width = sprite.dimensions.0 as f32;
            let height = sprite.dimensions.1 as f32;
            Instance::new(
                [p.0 as f32 / GAME_WIDTH, (GAME_HEIGHT - p.1 as f32) / GAME_HEIGHT, 0.0],
                [frame.w as f32 / GAME_WIDTH, frame.h as f32 / GAME_HEIGHT], 
                [frame.x as f32 / width, ((frame.y + frame.h as i32) as f32) / height],
                [frame.w as f32 / width,  frame.h as f32 / -height],  
            )
        }).collect::<Vec<Instance>>();
        let mut usage = wgpu::BufferUsage::VERTEX;
        usage.set(wgpu::BufferUsage::COPY_DST, !reuse_buffer);
        usage.set(wgpu::BufferUsage::COPY_SRC, reuse_buffer);
        let instance_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Sprite Instance Buffer"),
                contents: &bytemuck::cast_slice(&instances),
                usage,
            }
        );
        if reuse_buffer {
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sprite Update Encoder"),
            });
            encoder.copy_buffer_to_buffer(&instance_buffer, 0,
                 &sprite.instance_buffer, 0, (instances.len() * mem::size_of::<Instance>()) as u64);
            Ok(encoder.finish())
        }
        else {
            sprite.instance_buffer = instance_buffer;
            sprite.num_elements = instances.len() as u32;
            let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sprite Update Encoder"),
            });
            Ok(encoder.finish())
        }
    }
}