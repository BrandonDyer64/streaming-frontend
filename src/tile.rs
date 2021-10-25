use std::{
    collections::HashMap,
    io::Cursor,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use glyph_brush::{GlyphBrush, Section, Text};
use luminance::{
    context::GraphicsContext,
    pipeline::{PipelineError, TextureBinding},
    pixel::NormUnsigned,
    render_state::RenderState,
    shader::Uniform,
    tess::Mode,
    texture::Dim2,
};
use luminance_derive::UniformInterface;
use luminance_front::{pipeline::Pipeline, shader::Program, shading_gate::ShadingGate, tess::Tess};
use luminance_glfw::GL33Context;
use tokio::sync::RwLock;

use crate::{state, vertex::*, HEIGHT, WIDTH};

const VS_STR: &str = include_str!("shader.vert.glsl");
const FS_STR: &str = include_str!("shader.frag.glsl");

#[derive(UniformInterface)]
struct ShaderInterface {
    position: Uniform<[f32; 2]>,
    weight: Uniform<f32>,
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

pub struct Tile {
    x: f32,
    y: f32,
    size: f32,
    tex_id: u32,
}

pub struct TileRenderer {
    pub tess: Tess<()>,
    program: Program<(), (), ShaderInterface>,
    tiles: Vec<Tile>,
}

impl TileRenderer {
    pub fn new(ctxt: &mut GL33Context) -> Self {
        let tess: Tess<()> = ctxt
            .new_tess()
            .set_render_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .unwrap();
        let program = ctxt
            .new_shader_program::<(), (), ShaderInterface>()
            .from_strings(VS_STR, None, None, FS_STR)
            .unwrap()
            .ignore_warnings();
        Self {
            tess,
            program,
            tiles: Vec::new(),
        }
    }

    pub async fn update_tiles(
        &mut self,
        delta_t: f32,
        state: state::AsyncState,
        glyph_brush: &mut GlyphBrush<TextInstance>,
        tex_uid: &AtomicU32,
        loaded_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>>,
    ) {
        self.tiles.clear();
        let state_ = Arc::clone(&state);
        let mut state = state.write().await;
        let selected_card = state.selected_card;
        let scroll = state.scroll;
        let mut scroll_target = state.scroll_target;

        if state.show_modal {
            let card = state
                .rows
                .get_mut(selected_card.1)
                .and_then(|row| row.cards.get_mut(selected_card.0));
            if let Some(card) = card {
                if let state::CardImage::Texture(tex_id) = card.image {
                    card.size += (0.75 - card.size) * (1. - (1. - delta_t) * 0.7);
                    self.tiles.push(Tile {
                        x: -0.5,
                        y: 0.,
                        size: card.size,
                        tex_id: tex_id,
                    });
                    glyph_brush.queue(
                        Section::default()
                            .add_text(Text::new(&card.title).with_scale(50.))
                            .with_screen_position((WIDTH as f32, HEIGHT as f32 - 50.)),
                    );
                    for (i, r) in card.ratings.iter().zip(card.releases.iter()).enumerate() {
                        glyph_brush.queue(
                            Section::default()
                                .add_text(
                                    Text::new(&format!("{}  |  {}", r.0, r.1)).with_scale(35.),
                                )
                                .with_screen_position((
                                    WIDTH as f32,
                                    HEIGHT as f32 + 50. * i as f32,
                                )),
                        );
                    }
                    return;
                }
            }
        }

        for (y, row) in state.rows.iter_mut().enumerate() {
            row.scroll += (row.scroll_target - row.scroll) * (1. - (1. - delta_t) * 0.9);
            row.text_height +=
                (row.text_height_target - row.text_height) * (1. - (1. - delta_t) * 0.7);
            let y_pos = 0.6 - y as f32 * 0.6;
            let y_ = y_pos - scroll * 0.6;
            if selected_card.1 == y && (selected_card.0 as f32 - row.scroll).round() as u32 == 0 {
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
                        state::CardImage::URI(uri) => {
                            let uri = uri.clone();
                            card.image = state::CardImage::Loading(1);
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
                                        .map(|card| card.image = state::CardImage::Texture(uid));
                                }
                            });
                            0
                        }
                        state::CardImage::Texture(x) => *x,
                        _ => 0,
                    };
                    self.tiles.push(Tile {
                        tex_id: img_id,
                        x: x_,
                        y: y_,
                        size: card.size,
                    });
                }
                card.size += (target_size - card.size) * (1. - (1. - delta_t) * 0.7);
                if is_selected {
                    if x_pos - row.scroll_target * 0.4 > 0.7 {
                        row.scroll_target += 1.;
                    } else if x_pos - row.scroll_target * 0.4 < -0.71 {
                        row.scroll_target -= 1.;
                    }
                    if y_pos - scroll_target * 0.6 > 0.7 {
                        scroll_target += 1.;
                    } else if y_pos - scroll_target * 0.6 < -0.7 {
                        scroll_target -= 1.;
                    }
                }
            }
        }
        state.scroll_target = scroll_target;
        state.scroll += (state.scroll_target - state.scroll) * (1. - (1. - delta_t) * 0.9);
    }

    pub fn render(
        &mut self,
        pipeline: &Pipeline,
        shd_gate: &mut ShadingGate,
        bindable_textures: &mut HashMap<u32, RGBTexture>,
    ) -> Result<(), PipelineError> {
        let program = &mut self.program;
        let tess = &self.tess;
        let tiles = &self.tiles;
        shd_gate.shade(program, |mut iface, uni, mut rdr_gate| {
            for tile in tiles {
                let bound_tex = bindable_textures
                    .get_mut(&tile.tex_id)
                    .map(|tex| pipeline.bind_texture(tex))
                    .transpose()?;
                if let Some(bound_tex) = bound_tex {
                    iface.set(&uni.tex, bound_tex.binding());
                    iface.set(&uni.weight, tile.size * 0.5);
                    iface.set(&uni.position, [tile.x, tile.y]);
                    rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                        // let view = TessView::inst_whole(&triangle, panels.len());
                        tess_gate.render(tess)
                    })?;
                }
            }
            Ok(())
        })
    }
}
