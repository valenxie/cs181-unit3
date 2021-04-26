use crate::graphics::texture::CpuTexture;
use std::path::Path;
use std::rc::Rc;
pub struct Resources();

impl Resources {
    pub fn new() -> Self {
        Self()
    }
    pub fn load_texture(&self, p: impl AsRef<Path>) -> Rc<CpuTexture> {
        Rc::new(CpuTexture::with_file(p.as_ref()))
    }
}
