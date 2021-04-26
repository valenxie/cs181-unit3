use wgpu::{BindGroup, BindGroupLayout, CommandBuffer, Device, Queue, util::DeviceExt};

use super::{graphics::GpuState, screen::Screen, texture::{CpuTexture, Dimensions, Material}, vertex::SpriteVertex};
use super::texture::TextureHandle;
use super::gpu::{Instance, GAME_WIDTH, GAME_HEIGHT};
use crate::logic::types::*;

use std::{error::Error, mem, ops::Range, rc::Rc};

pub const TILE_SZ: usize = 16;
/// A graphical tile
#[derive(Clone, Copy)]
pub struct Tile {
    pub solid: bool, // ... any extra data like collision flags or other properties
    pub triangle: bool,
}
/// A set of tiles used in multiple Tilemaps
pub struct Tileset {
    // Tile size is a constant, so we can find the tile in the texture using math
    // (assuming the texture is a grid of tiles).
    pub tiles: Vec<Tile>,
    // Maybe a reference to a texture in a real program
    pub texture: Rc<CpuTexture>,
    // In this design, each tileset is a distinct image.
    // Maybe not always the best choice if there aren't many tiles in a tileset!
}
/// Indices into a Tileset
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TileID(usize);
/// Grab a tile with a given ID
impl std::ops::Index<TileID> for Tileset {
    type Output = Tile;
    fn index(&self, id: TileID) -> &Self::Output {
        &self.tiles[id.0]
    }
}
impl Tileset {
    pub fn new(tiles: Vec<Tile>, texture: &Rc<CpuTexture>) -> Self {
        Self {
            tiles,
            texture: Rc::clone(texture),
        }
    }
    fn get_rect(&self, id: TileID) -> Rect {
        let idx = id.0;
        let (w, _h) = self.texture.size();
        let tw = w / TILE_SZ;
        let row = idx / tw;
        let col = idx - (row * tw);
        Rect {
            x: col as i32 * TILE_SZ as i32,
            y: row as i32 * TILE_SZ as i32,
            w: TILE_SZ as u16,
            h: TILE_SZ as u16,
        }
    }
    fn contains(&self, id: TileID) -> bool {
        id.0 < self.tiles.len()
    }
}
/// An actual tilemap
pub struct Tilemap {
    /// Where the tilemap is in space, use your favorite number type here
    pub position: Vec2i,
    /// How big it is
    dims: (usize, usize),
    /// Which tileset is used for this tilemap
    pub tileset: Rc<Tileset>,
    /// A row-major grid of tile IDs in tileset
    map: Vec<TileID>,
}
impl Tilemap {
    pub fn new(
        position: Vec2i,
        dims: (usize, usize),
        tileset: &Rc<Tileset>,
        map: Vec<usize>,
    ) -> Self {
        assert_eq!(dims.0 * dims.1, map.len(), "Tilemap is the wrong size!");
        assert!(
            map.iter().all(|tid| tileset.contains(TileID(*tid))),
            "Tilemap refers to nonexistent tiles"
        );
        Self {
            position,
            dims,
            tileset: Rc::clone(tileset),
            map: map.into_iter().map(TileID).collect(),
        }
    }

    pub fn tile_id_at(&self, Vec2i(x, y): Vec2i) -> TileID {
        // Translate into map coordinates
        let x = (x - self.position.0) / TILE_SZ as i32;
        let y = (y - self.position.1) / TILE_SZ as i32;
        assert!(
            x >= 0 && x < self.dims.0 as i32,
            "Tile X coordinate {} out of bounds {}",
            x,
            self.dims.0
        );
        assert!(
            y >= 0 && y < self.dims.1 as i32,
            "Tile Y coordinate {} out of bounds {}",
            y,
            self.dims.1
        );
        self.map[y as usize * self.dims.0 + x as usize]
    }
    pub fn size(&self) -> (usize, usize) {
        self.dims
    }
    pub fn tile_at(&self, posn: Vec2i) -> Tile {
        self.tileset[self.tile_id_at(posn)]
    }
    pub fn contains(&self, Vec2i(x, y): Vec2i) -> bool {
        x > self.position.0
            && x < self.position.0 + (self.dims.0 * TILE_SZ) as i32
            && y > self.position.1
            && y < self.position.1 + (self.dims.1 * TILE_SZ) as i32
    }
    pub fn get_tile_rect(&self, Vec2i(x, y): Vec2i) -> Rect {
        // Translate into map coordinates
        let x = (x - self.position.0) / TILE_SZ as i32;
        let y = (y - self.position.1) / TILE_SZ as i32;
        assert!(
            x >= 0 && x < self.dims.0 as i32,
            "Tile X coordinate {} out of bounds {}",
            x,
            self.dims.0
        );
        assert!(
            y >= 0 && y < self.dims.1 as i32,
            "Tile Y coordinate {} out of bounds {}",
            y,
            self.dims.1
        );

        Rect {
            x: self.position.0 + x * TILE_SZ as i32,
            y: self.position.1 + y * TILE_SZ as i32,
            w: TILE_SZ as u16,
            h: TILE_SZ as u16,
        }
    }
    // ...
    /// Draws the portion of self appearing within screen.
    /// This could just as well be an extension trait on Screen defined in =tiles.rs= or something, like we did for =sprite.rs= and =draw_sprite=.
    pub fn draw(&self, screen: &mut Screen) {
        let Rect {
            x: sx,
            y: sy,
            w: sw,
            h: sh,
        } = screen.bounds();
        // We'll draw from the topmost/leftmost visible tile to the bottommost/rightmost visible tile.
        // The camera combined with out position and size tell us what's visible.
        // leftmost tile: get camera.x into our frame of reference, then divide down to tile units
        // Note that it's also forced inside of 0..self.size.0
        let left = ((sx - self.position.0) / TILE_SZ as i32)
            .max(0)
            .min(self.dims.0 as i32) as usize;
        // rightmost tile: same deal, but with screen.x + screen.w.
        let right = ((sx + (sw as i32) - self.position.0) / TILE_SZ as i32)
            .max(0)
            .min(self.dims.0 as i32) as usize;
        // ditto top and bot
        let top = ((sy - self.position.1) / TILE_SZ as i32)
            .max(0)
            .min(self.dims.1 as i32) as usize;
        let bot = ((sy + (sh as i32) - self.position.1) / TILE_SZ as i32)
            .max(0)
            .min(self.dims.1 as i32) as usize;
        // Now draw the tiles we need to draw where we need to draw them.
        // Note that we're zipping up the row index (y) with a slice of the map grid containing the necessary rows so we can avoid making a bounds check for each tile.
        for (y, row) in (top..bot)
            .zip(self.map[(top * self.dims.0)..(bot * self.dims.0)].chunks_exact(self.dims.0))
        {
            // We are in tile coordinates at this point so we'll need to translate back to pixel units and world coordinates to draw.
            let ypx = (y * TILE_SZ) as i32 + self.position.1;
            // Here we can iterate through the column index and the relevant slice of the row in parallel
            for (x, id) in (left..right).zip(row[left..right].iter()) {
                let xpx = (x * TILE_SZ) as i32 + self.position.0;
                let frame = self.tileset.get_rect(*id);
                screen.bitblt(&self.tileset.texture, frame, Vec2i(xpx, ypx));
            }
        }
    }

    pub fn load(&self, device: &Device, queue: &Queue, layout: &BindGroupLayout) -> Result<TilemapHandle, Box<dyn Error>> {
        let mut materials = Vec::new();
        let texture = self.tileset.texture.buffer();
        let dimensions = self.tileset.texture.size();
        let dimensions = (dimensions.0 as u32, dimensions.1 as u32);
        let (diffuse_texture, dimensions) = TextureHandle::from_bytes(device, queue, texture, dimensions, 
                                                                    "Tilemap Texture")?;
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
            name: "Tile Material".to_string(),
            diffuse_texture,
            bind_group,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", "Tilemap")),
                contents: bytemuck::cast_slice(&VERTICES),
                usage: wgpu::BufferUsage::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", "Tilemap")),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsage::INDEX,
            }
        );
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Tile Instance Buffer"),
                contents: &[],
                usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            }
        );

        Ok(TilemapHandle {materials, index_buffer, vertex_buffer, instance_buffer,
            dimensions, num_elements: 0})
    }
}

const VERTICES: &[SpriteVertex] = &[
    SpriteVertex { position: [0.0, 0.0, 0.8], tex_pos: [0.0, 0.0]},
    SpriteVertex { position: [1.0, 0.0, 0.8], tex_pos: [1.0, 0.0]},
    SpriteVertex { position: [1.0, 1.0, 0.8], tex_pos: [1.0, 1.0]},
    SpriteVertex { position: [0.0, 1.0, 0.8], tex_pos: [0.0, 1.0]},
];

const INDICES: &[u16] = &[
    0, 1, 2,
    2, 3, 0
];

pub struct TilemapHandle {
    pub materials: Vec<Material>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub dimensions: Dimensions,
}
pub trait DrawTilemap<'a, 'b>
where
    'b: 'a,
{
    fn draw_tilemap(&mut self, tileset: &'b TilemapHandle, uniforms: &'b BindGroup);
    fn draw_tilemap_instanced(
        &mut self,
        tilemap: &'b TilemapHandle,
        uniforms: &'b BindGroup,
        instances: Range<u32>,
    );
}
impl<'a, 'b> DrawTilemap<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_tilemap(&mut self, tileset: &'b TilemapHandle, uniforms: &'b BindGroup){
        self.draw_tilemap_instanced(tileset, uniforms, 0..tileset.num_elements);
    }

    fn draw_tilemap_instanced(
        &mut self,
        tilemap: &'b TilemapHandle,
        uniforms: &'b BindGroup,
        instances: Range<u32>,
    ){
        self.set_vertex_buffer(0, tilemap.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, tilemap.instance_buffer.slice(..));
        self.set_index_buffer(tilemap.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        self.set_bind_group(0, &tilemap.materials[0].bind_group, &[]);
        self.set_bind_group(1, uniforms, &[]);
        self.draw_indexed(0..INDICES.len() as u32, 0, instances);
    }
}

pub trait UpdateTilemap<'a, 'b>
where
    'b: 'a,
{
    fn update_tilemap_instances(
        &mut self,
        tiles: &Tilemap,
    ) -> Result<CommandBuffer, wgpu::SwapChainError>;
}
impl<'a, 'b> UpdateTilemap<'a, 'b> for GpuState
where
    'b: 'a,
{
    fn update_tilemap_instances(&mut self, tiles: &Tilemap) -> Result<CommandBuffer, wgpu::SwapChainError> {
        match &mut self.tilemap {
            Some(tilemap_handle) => {
                let mut instance_data = Vec::new();
                let (w, h) = tiles.dims;
                let width = tilemap_handle.dimensions.0 as f32;
                let height = tilemap_handle.dimensions.1 as f32;
                for y in 0..h {
                    for x in 0..w {
                        let pos = Vec2i( (x * TILE_SZ) as i32 + tiles.position.0, ((h - y - 1) * TILE_SZ) as i32 + tiles.position.1);
                        let tile_id = tiles.tile_id_at(pos);
                        
                        let tile = tiles.tileset.get_rect(tile_id);

                        instance_data.push(Instance::new(
                            [(x as i32 * TILE_SZ as i32 + tiles.position.0) as f32 / GAME_WIDTH, (y as i32 * TILE_SZ as i32 + tiles.position.1) as f32 / GAME_HEIGHT + 0.25, 0.0],
                            [TILE_SZ as f32 / GAME_WIDTH, TILE_SZ as f32 / GAME_HEIGHT],
                            [tile.x as f32 / width, ((tile.y + tile.h as i32) as f32) / height],
                [tile.w as f32 / width,  tile.h as f32 / -height],
                        ));
                    }
                }
                // Copy from this buffer if it is the same size,
                // Otherwise use it as the new instance buffer
                let reuse_buffer = tilemap_handle.num_elements >= instance_data.len() as u32;
                let mut usage = wgpu::BufferUsage::VERTEX;
                usage.set(wgpu::BufferUsage::COPY_DST, !reuse_buffer);
                usage.set(wgpu::BufferUsage::COPY_SRC, reuse_buffer);
                let instance_buffer = self.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("Tile Instance Buffer"),
                        contents: bytemuck::cast_slice(&instance_data),
                        usage,
                    }
                );
                if reuse_buffer {
                    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Tile Update Encoder"),
                    });
                    encoder.copy_buffer_to_buffer(&instance_buffer, 0,
                         &tilemap_handle.instance_buffer, 0, (instance_data.len() * mem::size_of::<Instance>()) as u64);
                    Ok(encoder.finish())
                }
                else {
                    if tilemap_handle.num_elements > 0 {
                        tilemap_handle.instance_buffer.unmap();
                    }
                    tilemap_handle.instance_buffer = instance_buffer;
                    tilemap_handle.num_elements = instance_data.len() as u32;
                    let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Tile Update Encoder"),
                    });
                    Ok(encoder.finish())
                }
            },
            None=> {
                let encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Tile Update Encoder"),
                });
                Ok(encoder.finish())
            },
        }
    }
}