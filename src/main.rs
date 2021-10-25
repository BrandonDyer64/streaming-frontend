use glfw::Context as _;
use glyph_brush::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder};
use image::GenericImageView;
use luminance::{
    context::GraphicsContext,
    pipeline::PipelineState,
    texture::{GenMipmaps, Sampler},
};
use luminance_front::texture::Texture;
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::collections::HashMap;
use std::process::exit;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

mod api;
mod input;
mod model;
mod state;
mod text;
mod tile;
mod vertex;

use state::*;
use vertex::*;

use crate::{text::TextRenderer, tile::TileRenderer};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 540;

#[tokio::main]
async fn main() {
    // our graphics surface
    let dim = WindowDim::Windowed {
        width: WIDTH,
        height: HEIGHT,
    };
    let surface = GlfwSurface::new_gl33(
        "Disney+",
        WindowOpt::default().set_num_samples(16).set_dim(dim),
    );

    match surface {
        Ok(surface) => {
            eprintln!("graphics surface created");
            main_loop(surface).await;
        }

        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }
}

async fn main_loop(surface: GlfwSurface) {
    let mut ctxt = surface.context;
    let start_t = Instant::now();
    let mut last_t = 0.;
    let events = surface.events_rx;
    let back_buffer = ctxt.back_buffer().expect("back buffer");

    let state = Arc::new(RwLock::new(State {
        rows: Vec::new(),
        selected_card: (0, 0),
        show_modal: false,
        scroll: 0.,
        scroll_target: 0.,
    }));

    let font = FontArc::try_from_slice(include_bytes!("OpenSans-Regular.ttf")).expect("font");
    let mut glyph_brush: GlyphBrush<TextInstance> = GlyphBrushBuilder::using_font(font).build();

    let mut text_renderer = TextRenderer::new(&mut ctxt, &mut glyph_brush);
    let mut tile_renderer = TileRenderer::new(&mut ctxt);

    let tex_uid = AtomicU32::new(1u32);

    let mut bindable_textures: HashMap<u32, RGBTexture> = HashMap::new();

    let loaded_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>> =
        Arc::new(RwLock::new(Vec::new()));

    tokio::spawn(api::load_home(state.clone(), loaded_images.clone()));

    'app: loop {
        // handle events
        ctxt.window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            if !input::handle_event(event, state.clone()).await {
                break 'app;
            }
        }

        {
            let mut loaded_images = loaded_images.write().await;
            for loaded_image in loaded_images.iter() {
                let img = &loaded_image.1;
                let (width, height) = img.dimensions();
                let texels = img.as_bytes();
                let new_tex: RGBTexture = Texture::new_raw(
                    &mut ctxt,
                    [width, height],
                    0,
                    Sampler::default(),
                    GenMipmaps::No,
                    texels,
                )
                .map_err(|e| println!("error while creating texture: {}", e))
                .ok()
                .expect("load displacement map");
                bindable_textures.insert(loaded_image.0, new_tex);
            }
            loaded_images.clear();
        }

        let t = start_t.elapsed().as_secs_f32();
        let delta_t = t - last_t;
        last_t = t;

        tile_renderer
            .update_tiles(
                delta_t,
                state.clone(),
                &mut glyph_brush,
                &tex_uid,
                loaded_images.clone(),
            )
            .await;

        text_renderer.process_queued(&mut ctxt, &mut glyph_brush);

        let render = ctxt
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default().set_clear_color([0.01, 0.01, 0.01, 1.]),
                |pipeline, mut shd_gate| {
                    tile_renderer.render(&pipeline, &mut shd_gate, &mut bindable_textures)?;
                    text_renderer.render(&pipeline, &mut shd_gate)?;
                    Ok(())
                },
            )
            .assume();

        // swap buffer chains
        if render.is_ok() {
            ctxt.window.swap_buffers();
        } else {
            break 'app;
        }
    }
    println!("Done.");
}
