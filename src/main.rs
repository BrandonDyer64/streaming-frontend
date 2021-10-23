use glfw::{Action, Context as _, Key, WindowEvent};
use image::GenericImageView;
use luminance::pixel::NormRGB8UI;
use luminance::tess::{TessView, View};
use luminance::texture::{Dimensionable, GenMipmaps, Sampler};
use luminance_front::texture::Texture;
use luminance_front::{
    context::GraphicsContext as _, pipeline::PipelineState, render_state::RenderState, tess::Mode,
    texture::Dim2,
};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use serde_json::Value;
use std::io::Cursor;
use std::process::exit;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

mod state;
mod vertex;

use state::*;
use vertex::*;

const VERTICES: [Vertex; 4] = [
    Vertex::new(
        VertexPosition::new([-0.5, -0.5]),
        VertexRGB::new([255, 0, 0]),
    ),
    Vertex::new(
        VertexPosition::new([0.5, -0.5]),
        VertexRGB::new([0, 255, 0]),
    ),
    Vertex::new(VertexPosition::new([0.5, 0.5]), VertexRGB::new([0, 0, 255])),
    Vertex::new(
        VertexPosition::new([-0.5, 0.5]),
        VertexRGB::new([255, 255, 0]),
    ),
];

const VS_STR: &str = include_str!("shader.vert.glsl");
const FS_STR: &str = include_str!("shader.frag.glsl");

#[tokio::main]
async fn main() {
    // our graphics surface
    let dim = WindowDim::Windowed {
        width: 960,
        height: 540,
    };
    let surface = GlfwSurface::new_gl33("Hello, world!", WindowOpt::default().set_dim(dim));

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
    let start_t = Instant::now();
    let mut last_t = 0.;
    let mut ctxt = surface.context;
    let events = surface.events_rx;
    let back_buffer = ctxt.back_buffer().expect("back buffer");
    let default_instances: Vec<Instance> = vec![
        Instance {
            pos: VertexInstancePosition::new([0., 0.]),
            w: VertexWeight::new(0.5),
        };
        50
    ];

    let mut triangle = ctxt
        .new_tess()
        .set_vertices(&VERTICES[..])
        .set_instances(&default_instances[..])
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();
    let mut program = ctxt
        .new_shader_program::<VertexSemantics, (), ()>()
        .from_strings(VS_STR, None, None, FS_STR)
        .unwrap()
        .ignore_warnings();

    let image: Option<RGBTexture> = None;

    let mut state = State {
        rows: vec![
            Row {
                scroll: 0.0,
                scroll_target: 0.0,
                title: String::from("Test row"),
                cards: vec![
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                    Card {
                        title: String::from("Michael Rodent"),
                        image: None,
                    },
                    Card {
                        title: String::from("Goof Moov"),
                        image: None,
                    },
                ],
            },
            Row {
                scroll: 0.0,
                scroll_target: 0.0,
                title: String::from("Test row 2"),
                cards: vec![
                    Card {
                        title: String::from("Middle School Musical"),
                        image: None,
                    },
                    Card {
                        title: String::from("Sorcerers of Street St"),
                        image: None,
                    },
                ],
            },
        ],
        selected_card: (0, 0),
        show_modal: false,
    };

    let state = Arc::new(RwLock::new(state.clone()));

    // let a = Arc::new("bye");

    // let b = Arc::clone(&a);

    // ex.spawn(async move {
    //     smol::Timer::after(Duration::from_secs(1)).await;
    //     println!("Hello{}", b);
    //     smol::Timer::after(Duration::from_secs(1)).await;
    //     println!("Okay");
    // })
    // .detach();
    // ex.spawn(async move {
    //     smol::Timer::after(Duration::from_secs(1)).await;
    //     println!("Hello{}", a);
    //     smol::Timer::after(Duration::from_secs(1)).await;
    //     println!("Okay");
    // })
    // .detach();

    let loaded_image: Arc<RwLock<(Option<image::DynamicImage>,)>> = Arc::new(RwLock::new((None,)));

    let async_state = Arc::clone(&state);
    let async_image = Arc::clone(&loaded_image);
    tokio::spawn(async move {
        let loaded_image = async_image;
        let body = reqwest::get("https://cd-static.bamgrid.com/dp-117731241344/home.json")
            .await
            .unwrap();
        // let response = Request::builder()
        //     .uri("https://cd-static.bamgrid.com/dp-117731241344/home.json")
        //     .call()
        //     .await;
        // let mut body = response.unwrap().into_body();
        // let text = body.read_to_string().await.unwrap();
        // println!("{}", text);
        let x = body.json::<Value>().await.unwrap();
        println!(
            "{:#?}",
            x.as_object()
                .unwrap()
                .get("data")
                .unwrap()
                .as_object()
                .unwrap()
                .get("StandardCollection")
                .unwrap()
                .as_object()
                .unwrap()
                .get("collectionId")
                .unwrap()
                .as_str()
        );
        let response = reqwest::get("https://httpbin.org/image/png").await.unwrap();
        let cursor = Cursor::new(response.bytes().await.unwrap());
        let img = image::io::Reader::new(cursor)
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        {
            let mut loaded_image = loaded_image.write().await;
            loaded_image.0 = Some(img);
        }
        let mut state = async_state.write().await;
        state.rows.push(Row {
            scroll: 0.0,
            scroll_target: 0.0,
            title: String::from("Test row 2"),
            cards: vec![
                Card {
                    title: String::from("Middle School Musical"),
                    image: None,
                },
                Card {
                    title: String::from("Sorcerers of Street St"),
                    image: None,
                },
            ],
        });
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
                WindowEvent::Key(Key::Right, _, Action::Press, _) => {
                    state.selected_card.0 = state.selected_card.0.saturating_add(1);
                }
                WindowEvent::Key(Key::Left, _, Action::Press, _) => {
                    state.selected_card.0 = state.selected_card.0.saturating_sub(1);
                }
                WindowEvent::Key(Key::Up, _, Action::Press, _) => {
                    state.selected_card.1 = state.selected_card.1.saturating_sub(1);
                }
                WindowEvent::Key(Key::Down, _, Action::Press, _) => {
                    state.selected_card.1 = state.selected_card.1.saturating_add(1);
                }
                _ => (),
            }
        }

        {
            let mut loaded_image = loaded_image.write().await;
            if let Some(img) = &loaded_image.0 {
                let (width, height) = img.dimensions();
                let texels = img.as_bytes();
                let tex: RGBTexture = Texture::new_raw(
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
                loaded_image.0 = None;
                println!("did the image thing");
            }
        }

        // rendering code goes here
        // get the current time and create a color based on the time
        let t = start_t.elapsed().as_secs_f32();
        let delta_t = t - last_t;
        last_t = t;
        let color = [0., 0., 0.2, 1.];

        let mut panels = Vec::new();

        {
            let mut state = state.write().await;
            let selected_card = state.selected_card;
            for (y, row) in state.rows.iter_mut().enumerate() {
                row.scroll += (row.scroll_target - row.scroll) * (1. - (1. - delta_t) * 0.9);
                for (x, _card) in row.cards.iter().enumerate() {
                    let is_selected = selected_card == (x, y);
                    let size = if is_selected { 0.4 } else { 0.32 };
                    let x_pos = x as f32 * 0.4 - 1. + 0.2;
                    panels.push((x_pos - row.scroll, 0.8 - y as f32 * 0.6, size, size));
                    if is_selected {
                        if x_pos - row.scroll_target > 0.8 {
                            row.scroll_target += 0.5;
                        } else if x_pos - row.scroll_target < -0.8 {
                            row.scroll_target -= 0.5;
                        }
                    }
                }
            }
        }

        // make instances go boop boop by changing their weight dynamically
        {
            let mut instances = triangle.instances_mut().expect("instances");

            for (i, instance) in instances.iter_mut().enumerate() {
                if let Some(panel) = panels.get(i) {
                    instance.pos = VertexInstancePosition::new([panel.0, panel.1]);
                    instance.w = VertexWeight::new(panel.2);
                }
            }
        }

        let render = ctxt
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default().set_clear_color(color),
                |_pipeline, mut shd_gate| {
                    shd_gate.shade(&mut program, |mut iface, _, mut rdr_gate| {
                        if let Ok(ref time_u) = iface.query().unwrap().ask::<f32, &str>("t") {
                            iface.set(time_u, t);
                        }
                        rdr_gate.render(&RenderState::default(), |mut tess_gate| {
                            // let view = TessView::inst_whole(&triangle, panels.len());
                            tess_gate.render(triangle.inst_view(.., panels.len()).unwrap())
                        })
                    })
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
}
