use glfw::Context as _;
use glyph_brush::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder, Section, Text};
use image::GenericImageView;
use luminance::{
    context::GraphicsContext,
    pipeline::{PipelineState, TextureBinding},
    pixel::NormUnsigned,
    render_state::RenderState,
    shader::Uniform,
    tess::Mode,
    texture::{Dim2, GenMipmaps, Sampler},
    UniformInterface,
};
use luminance_front::{tess::Tess, texture::Texture};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::collections::HashMap;
use std::io::Cursor;
use std::process::exit;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

mod api;
mod input;
mod model;
mod state;
mod text;
mod vertex;

use state::*;
use vertex::*;

use crate::text::TextRenderer;

const VS_STR: &str = include_str!("shader.vert.glsl");
const FS_STR: &str = include_str!("shader.frag.glsl");

#[derive(UniformInterface)]
struct ShaderInterface {
    position: Uniform<[f32; 2]>,
    weight: Uniform<f32>,
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

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

    let tess: Tess<()> = ctxt
        .new_tess()
        .set_render_vertex_nb(4)
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();
    let mut program = ctxt
        .new_shader_program::<(), (), ShaderInterface>()
        .from_strings(VS_STR, None, None, FS_STR)
        .unwrap()
        .ignore_warnings();

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

        let mut panels = Vec::new();

        {
            let state_ = Arc::clone(&state);
            let mut state = state.write().await;
            let selected_card = state.selected_card;
            let scroll = state.scroll;
            let mut scroll_target = state.scroll_target;
            for (y, row) in state.rows.iter_mut().enumerate() {
                row.scroll += (row.scroll_target - row.scroll) * (1. - (1. - delta_t) * 0.9);
                row.text_height +=
                    (row.text_height_target - row.text_height) * (1. - (1. - delta_t) * 0.7);
                let y_pos = 0.6 - y as f32 * 0.6;
                let y_ = y_pos - scroll * 0.4;
                if selected_card.1 == y && (selected_card.0 as f32 - row.scroll).round() as u32 == 0
                {
                    row.text_height_target = 0.3;
                } else {
                    row.text_height_target = 0.25;
                }
                glyph_brush.queue(
                    Section::default()
                        .add_text(Text::new(&row.title).with_scale(36.))
                        .with_screen_position((
                            135.,
                            HEIGHT as f32 - (y_ + row.text_height) * HEIGHT as f32,
                        )),
                );
                for (x, card) in row.cards.iter_mut().enumerate() {
                    let is_selected = selected_card == (x, y);
                    let target_size = if is_selected { 0.42 } else { 0.32 };
                    let x_pos = x as f32 * 0.4 - 1. + 0.3;
                    let x_ = x_pos - row.scroll * 0.4;
                    if x_ > -1.5 && x_ < 1.5 && y_ > -1.5 && y_ < 1.5 {
                        let img_id: u32 = match &card.image {
                            CardImage::URI(uri) => {
                                let uri = uri.clone();
                                card.image = CardImage::Loading(1);
                                let uid = tex_uid.fetch_add(1, Ordering::SeqCst);
                                let async_state = Arc::clone(&state_);
                                let async_images = Arc::clone(&loaded_images);
                                tokio::spawn(async move {
                                    let loaded_images = async_images;
                                    let state = async_state;
                                    let response = reqwest::get(uri).await.unwrap();
                                    let cursor = Cursor::new(response.bytes().await.unwrap());
                                    let img = image::io::Reader::new(cursor)
                                        .with_guessed_format()
                                        .unwrap()
                                        .decode()
                                        .unwrap();

                                    {
                                        let mut loaded_images = loaded_images.write().await;
                                        loaded_images.push((uid, img));
                                    }
                                    {
                                        let mut state = state.write().await;
                                        state
                                            .rows
                                            .get_mut(y as usize)
                                            .and_then(|row| row.cards.get_mut(x as usize))
                                            .map(|card| card.image = CardImage::Texture(uid));
                                    }
                                });
                                0
                            }
                            CardImage::Texture(x) => *x,
                            _ => 0,
                        };
                        // if x == 0 && y == 0 {
                        //     println!("{}", img_id);
                        // }
                        panels.push((img_id, x_, y_, card.size, card.size));
                    }
                    card.size += (target_size - card.size) * (1. - (1. - delta_t) * 0.7);
                    if is_selected {
                        if x_pos - row.scroll_target * 0.4 > 0.7 {
                            row.scroll_target += 1.;
                        } else if x_pos - row.scroll_target * 0.4 < -0.71 {
                            row.scroll_target -= 1.;
                        }
                        if y_pos - scroll_target * 0.4 > 0.7 {
                            scroll_target += 1.;
                        } else if y_pos - scroll_target * 0.4 < -0.7 {
                            scroll_target -= 1.;
                        }
                    }
                }
            }
            state.scroll_target = scroll_target;
            state.scroll += (state.scroll_target - state.scroll) * (1. - (1. - delta_t) * 0.9);
        }

        text_renderer.process_queued(&mut ctxt, &mut glyph_brush);

        let render = ctxt
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default().set_clear_color([0.01, 0.01, 0.01, 1.]),
                |pipeline, mut shd_gate| {
                    shd_gate.shade(&mut program, |mut iface, uni, mut rdr_gate| {
                        for panel in panels {
                            let bound_tex = bindable_textures
                                .get_mut(&panel.0)
                                .map(|tex| pipeline.bind_texture(tex))
                                .transpose()?;
                            if let Some(bound_tex) = bound_tex {
                                if let Ok(ref time_u) = iface.query().unwrap().ask::<f32, &str>("t")
                                {
                                    iface.set(time_u, t);
                                }
                                iface.set(&uni.tex, bound_tex.binding());
                                iface.set(&uni.weight, panel.3 * 0.5);
                                iface.set(&uni.position, [panel.1, panel.2]);
                                rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                                    // let view = TessView::inst_whole(&triangle, panels.len());
                                    tess_gate.render(&tess)
                                })?;
                            }
                        }
                        Ok(())
                    })?;
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
