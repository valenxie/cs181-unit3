
use image::{self, GenericImageView, RgbaImage};
use wgpu::BindGroup;
use std::{env, error::Error, path::Path};

use crate::logic::types::Rect;

pub type Dimensions = (u32, u32);
pub struct TextureHandle {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl TextureHandle {
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage, 
        label: &str,
    ) -> Result<(Self, Dimensions), Box<dyn Error>> {
        Self::from_bytes(device, queue, &img.to_bytes(), img.dimensions(), label)
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        dimensions: Dimensions,
        label: &str
    ) -> Result<(Self, Dimensions), Box<dyn Error>> {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some(label),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            }
        );

        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &bytes,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * dimensions.0,
                rows_per_image: dimensions.1,
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        
        Ok((Self { texture, view, sampler }, dimensions))
    }

    pub fn load<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: P,
    ) -> Result<(Self, Dimensions), Box<dyn Error>> {
        // Needed to appease the borrow checker
        let path_copy = path.as_ref().to_path_buf();
        let label = path_copy.to_str();
        
        let img = image::open(path)?;
        Self::from_image(device, queue, &img, label.unwrap())
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float; // 1.
    
    pub fn create_depth_texture(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        let size = wgpu::Extent3d { // 2.
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT // 3.
                | wgpu::TextureUsage::SAMPLED,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor { // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual), // 5.
                lod_min_clamp: -100.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self { texture, view, sampler }
    }
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: TextureHandle,
    pub bind_group:BindGroup,
}

pub struct CpuTexture {
    image: Vec<u8>,
    width: usize,
    height: usize,
    depth: usize,
}

#[allow(dead_code)]
enum AlphaChannel {
    First,
    Last,
}
impl CpuTexture {
    pub fn with_file(path: &Path) -> Self {
        let pathbuf = env::current_dir().unwrap();
        println!("The image is {}/{}", pathbuf.display(), path.display());

        Self::new(image::open(path).expect("Couldn't load image").into_rgba8())
    }
    pub fn new(image: RgbaImage) -> Self {
        let (width, height) = image.dimensions();
        let mut image = image.into_vec();
        premultiply(&mut image, 4, AlphaChannel::Last);
        Self {
            width: width as usize,
            height: height as usize,
            depth: 4,
            image,
        }
    }
    pub fn depth(&self) -> usize {
        self.depth
    }
    pub fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
    pub fn pitch(&self) -> usize {
        self.width * self.depth
    }
    pub fn buffer(&self) -> &[u8] {
        &self.image
    }
    pub fn valid_frame(&self, frame: Rect) -> bool {
        0 <= frame.x
            && (frame.x + frame.w as i32) <= (self.width as i32)
            && 0 <= frame.y
            && (frame.y + frame.h as i32) <= (self.height as i32)
    }
}

fn premultiply(img: &mut [u8], depth: usize, alpha: AlphaChannel) {
    match alpha {
        AlphaChannel::First => {
            for px in img.chunks_exact_mut(depth) {
                let a = px[0] as f32 / 255.0;
                for component in px[1..].iter_mut() {
                    *component = (*component as f32 * a).round() as u8;
                }
                // swap around to rgba8888
                let a = px[0];
                px[0] = px[1];
                px[1] = px[2];
                px[2] = px[3];
                px[3] = a;
            }
        }
        AlphaChannel::Last => {
            for px in img.chunks_exact_mut(depth) {
                let a = *px.last().unwrap() as f32 / 255.0;
                for component in px[0..(depth - 1)].iter_mut() {
                    *component = (*component as f32 * a) as u8;
                }
                // already rgba8888
            }
        }
    }
}
