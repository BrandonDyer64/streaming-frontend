use glfw::Context as _;
use glyph_brush::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder};
use luminance::{context::GraphicsContext, pipeline::PipelineState};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::process::exit;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

mod api;
mod input;
mod model;
mod state;
mod tex;
mod text;
mod tile;
mod vertex;

use state::*;
use vertex::*;

use crate::{tex::TextureHost, text::TextRenderer, tile::TileRenderer};

// TODO: Use dynamic width and height
pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 540;

#[tokio::main]
async fn main() {
    // our graphics surface
    let surface = GlfwSurface::new_gl33(
        "Disney+",
        WindowOpt::default()
            .set_num_samples(16)
            .set_dim(WindowDim::Windowed {
                width: WIDTH,
                height: HEIGHT,
            }),
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

    let state = Arc::new(RwLock::new(State::new()));

    let font = FontArc::try_from_slice(include_bytes!("OpenSans-Regular.ttf")).expect("font");
    let mut glyph_brush: GlyphBrush<TextInstance> = GlyphBrushBuilder::using_font(font).build();

    let mut text_renderer = TextRenderer::new(&mut ctxt, &mut glyph_brush);
    let mut tile_renderer = TileRenderer::new(&mut ctxt);
    let mut texture_host = TextureHost::new();

    let tex_uid = AtomicU32::new(1u32);

    tokio::spawn(api::load_home(
        state.clone(),
        texture_host.queued_images.clone(),
    ));

    'app: loop {
        // Handle events
        ctxt.window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            if !input::handle_event(event, state.clone()).await {
                break 'app;
            }
        }

        // Timing
        let t = start_t.elapsed().as_secs_f32();
        let delta_t = t - last_t;
        last_t = t;

        // Process what's to be rendered
        texture_host.process_queued(&mut ctxt).await;
        tile_renderer
            .update_tiles(
                delta_t,
                state.clone(),
                &mut glyph_brush,
                &tex_uid,
                texture_host.queued_images.clone(),
            )
            .await;
        text_renderer.process_queued(&mut ctxt, &mut glyph_brush);

        // Render pipeline
        let render = ctxt
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default().set_clear_color([0.01, 0.01, 0.01, 1.]),
                |pipeline, mut shd_gate| {
                    tile_renderer.render(
                        &pipeline,
                        &mut shd_gate,
                        &mut texture_host.bindable_textures,
                    )?;
                    text_renderer.render(&pipeline, &mut shd_gate)?;
                    Ok(())
                },
            )
            .assume();

        // Swap buffer chains
        if render.is_ok() {
            ctxt.window.swap_buffers();
        } else {
            break 'app;
        }
    }
    println!("Done.");
}
