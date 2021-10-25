use glfw::{Action, Context as _, Key, WindowEvent};
use glyph_brush::{
    ab_glyph::{point, FontArc, Rect},
    BrushAction, BrushError, GlyphBrush, GlyphBrushBuilder, Section, Text,
};
use image::GenericImageView;
use luminance::{
    context::GraphicsContext,
    pipeline::{PipelineState, TextureBinding},
    pixel::{NormR8UI, NormUnsigned},
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

mod model;
mod state;
mod vertex;

use state::*;
use vertex::*;

use crate::model::collection::{Data, Home, RefSet};

const VS_STR: &str = include_str!("shader.vert.glsl");
const FS_STR: &str = include_str!("shader.frag.glsl");

const VS_FONT_STR: &str = include_str!("text.vert.glsl");
const FS_FONT_STR: &str = include_str!("text.frag.glsl");

#[derive(UniformInterface)]
struct ShaderInterface {
    position: Uniform<[f32; 2]>,
    weight: Uniform<f32>,
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

#[derive(UniformInterface)]
pub struct FontShaderInterface {
    pub tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
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
    let mut font_program = ctxt
        .new_shader_program::<VertexSemantics, (), FontShaderInterface>()
        .from_strings(VS_FONT_STR, None, None, FS_FONT_STR)
        .expect("Program creation")
        .ignore_warnings();
    let state = Arc::new(RwLock::new(State {
        rows: Vec::new(),
        selected_card: (0, 0),
        show_modal: false,
        scroll: 0.,
        scroll_target: 0.,
    }));

    let font = FontArc::try_from_slice(include_bytes!("OpenSans-Regular.ttf")).expect("font");
    let mut glyph_brush: GlyphBrush<Instance> = GlyphBrushBuilder::using_font(font).build();

    let mut font_tess = None;
    let mut font_tex: Texture<Dim2, NormR8UI> = Texture::new(
        &mut ctxt,
        [
            glyph_brush.texture_dimensions().0,
            glyph_brush.texture_dimensions().1,
        ],
        0,
        Sampler::default(),
        GenMipmaps::No,
        &vec![
            0u8;
            glyph_brush.texture_dimensions().0 as usize
                * glyph_brush.texture_dimensions().1 as usize
        ],
    )
    .expect("luminance texture creation");

    let tex_uid = AtomicU32::new(1u32);

    let mut bindable_textures: HashMap<u32, RGBTexture> = HashMap::new();

    let loaded_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>> =
        Arc::new(RwLock::new(Vec::new()));

    let async_state = Arc::clone(&state);
    let async_images = Arc::clone(&loaded_images);
    tokio::spawn(async move {
        let loaded_images = async_images;
        let body = reqwest::get("https://cd-static.bamgrid.com/dp-117731241344/home.json")
            .await
            .unwrap();
        let x = body.json::<Home>().await.unwrap();
        #[allow(irrefutable_let_patterns)]
        if let Data::StandardCollection {
            collection_id: _,
            containers,
        } = x.data
        {
            for container in &containers {
                let mut _state = async_state.clone();
                let row: Option<Row> = match &container.set {
                    model::collection::Set::CuratedSet {
                        set_id: _,
                        text: _,
                        items: _,
                    } => (&container.set).into(),
                    model::collection::Set::SetRef {
                        ref_id,
                        ref_id_type: _,
                        ref_type: _,
                    } => {
                        let ref_id = ref_id.clone();
                        let async_state = Arc::clone(&_state);
                        tokio::spawn(async move {
                            let state = async_state;
                            let body = reqwest::get(format!(
                                "https://cd-static.bamgrid.com/dp-117731241344/sets/{}.json",
                                ref_id
                            ))
                            .await
                            .unwrap();
                            let x = body
                                .json::<RefSet>()
                                .await
                                .map_err(|e| println!("{} {}", ref_id, e))
                                .unwrap();
                            let row: Option<Row> = (&x.data.set).into();
                            if let Some(row) = row {
                                let mut state = state.write().await;
                                state.rows.push(row);
                            }
                        });
                        None
                    }
                };
                if let Some(row) = row {
                    let mut state = async_state.write().await;
                    state.rows.push(row);
                }
            }
        }
        let response = reqwest::get("https://prod-ripcut-delivery.disney-plus.net/v1/variant/disney/CD3FC43E25A8722F8264FD65BB0F534FAAD5312DE01E5E949875E2AFB316022B/scale?format=jpeg&quality=90&scalingAlgorithm=lanczos3&width=500").await.unwrap();
        let cursor = Cursor::new(response.bytes().await.unwrap());
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        {
            let mut loaded_images = loaded_images.write().await;
            loaded_images.push((0u32, img));
        }
    });

    'app: loop {
        // handle events
        ctxt.window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            let mut state = state.write().await;
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                WindowEvent::Key(Key::Right, _, Action::Press | Action::Repeat, _) => {
                    let new = state.selected_card.0.saturating_add(1);
                    if new < state.rows[state.selected_card.1].cards.len() {
                        state.selected_card.0 = new;
                    }
                }
                WindowEvent::Key(Key::Left, _, Action::Press | Action::Repeat, _) => {
                    state.selected_card.0 = state.selected_card.0.saturating_sub(1);
                }
                WindowEvent::Key(Key::Up, _, Action::Press, _) => {
                    let new = state.selected_card.1.saturating_sub(1);
                    if new < state.rows.len() {
                        let scroll_old = state.rows[state.selected_card.1].scroll.round() as isize;
                        let scroll_new = state.rows[new].scroll.round() as isize;
                        let mut new_scrl = state.selected_card.0 as isize;
                        new_scrl += scroll_new - scroll_old;
                        state.selected_card.0 = new_scrl as usize;
                        state.selected_card.1 = new;
                    }
                }
                WindowEvent::Key(Key::Down, _, Action::Press, _) => {
                    let new = state.selected_card.1.saturating_add(1);
                    if new < state.rows.len() {
                        let scroll_old = state.rows[state.selected_card.1].scroll.round() as isize;
                        let scroll_new = state.rows[new].scroll.round() as isize;
                        let mut new_scrl = state.selected_card.0 as isize;
                        new_scrl += scroll_new - scroll_old;
                        state.selected_card.0 = new_scrl as usize;
                        state.selected_card.1 = new;
                    }
                }
                _ => (),
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

        let action = glyph_brush.process_queued(
            |rect, tex_data| {
                // Update part of gpu texture with new glyph alpha values
                font_tex
                    .upload_part_raw(
                        GenMipmaps::No,
                        [rect.min[0] as u32, rect.min[1] as u32],
                        [rect.width() as u32, rect.height() as u32],
                        tex_data,
                    )
                    .expect("Cannot upload part of texture");
            },
            |vertex_data| to_vertex(WIDTH as f32 * 2., HEIGHT as f32 * 2., vertex_data),
        );

        if let Err(e) = action {
            let BrushError::TextureTooSmall { suggested } = e;
            glyph_brush.resize_texture(suggested.0, suggested.1);
            return;
        }
        let action = action.unwrap();
        match action {
            BrushAction::Draw(v) => {
                let tess = ctxt
                    .new_tess()
                    .set_render_vertex_nb(4)
                    .set_instances(v)
                    .set_mode(Mode::TriangleStrip)
                    .build()
                    .unwrap();
                font_tess = Some(tess);
            }
            BrushAction::ReDraw => (),
        };

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
                    if let Some(tess) = font_tess.as_ref() {
                        shd_gate.shade(&mut font_program, |mut iface, uni, mut rdr_gate| {
                            let bound_tex = pipeline.bind_texture(&mut font_tex)?;
                            iface.set(&uni.tex, bound_tex.binding());
                            //iface.set(&uni.transform, proj);
                            rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                                tess_gate.render(tess)
                            })
                        })?;
                    }
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

#[inline]
fn to_vertex(
    width: f32,
    height: f32,
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        ..
    }: glyph_brush::GlyphVertex,
) -> Instance {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
    };
    // println!("GL_RECT = {:?}", gl_rect);

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    let to_view_space = |x: f32, y: f32| -> [f32; 2] {
        let pos_x = (x / width) * 2.0 - 1.0;
        let pos_y = (1.0 - y / height) * 2.0 - 1.0;
        [pos_x, pos_y]
    };

    let left_top = to_view_space(gl_rect.min.x, gl_rect.max.y);

    let v = Instance {
        left_top: VertexLeftTop::new([left_top[0], left_top[1], 0.]),
        right_bottom: VertexRightBottom::new(to_view_space(gl_rect.max.x, gl_rect.min.y)),
        tex_left_top: TextureLeftTop::new([tex_coords.min.x, tex_coords.max.y]),
        tex_right_bottom: TextureRightBottom::new([tex_coords.max.x, tex_coords.min.y]),
        color: TextColor::new([1., 1., 1., 1.]),
    };

    // println!("vertex -> {:?}", v);
    v
}
