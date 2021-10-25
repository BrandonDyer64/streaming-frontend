use std::{collections::HashMap, sync::Arc};

use image::GenericImageView;
use luminance::texture::{GenMipmaps, Sampler};
use luminance_front::texture::Texture;
use luminance_glfw::GL33Context;
use tokio::sync::RwLock;

use crate::vertex::RGBTexture;

pub struct TextureHost {
    pub bindable_textures: HashMap<u32, RGBTexture>,
    pub queued_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>>,
}

impl TextureHost {
    pub fn new() -> Self {
        let bindable_textures: HashMap<u32, RGBTexture> = HashMap::new();

        let queued_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>> =
            Arc::new(RwLock::new(Vec::new()));

        Self {
            bindable_textures,
            queued_images,
        }
    }

    pub async fn process_queued(&mut self, ctxt: &mut GL33Context) {
        let mut queued_images = self.queued_images.write().await;
        for loaded_image in queued_images.iter() {
            let img = &loaded_image.1;
            let (width, height) = img.dimensions();
            let texels = img.as_bytes();
            let new_tex: RGBTexture = Texture::new_raw(
                ctxt,
                [width, height],
                0,
                Sampler::default(),
                GenMipmaps::No,
                texels,
            )
            .map_err(|e| println!("error while creating texture: {}", e))
            .ok()
            .expect("load displacement map");
            self.bindable_textures.insert(loaded_image.0, new_tex);
        }
        queued_images.clear();
    }
}
