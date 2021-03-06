use crate::{model::collection, state};
use std::{io::Cursor, sync::Arc};
use tokio::sync::RwLock;

pub async fn load_home(
    state: Arc<RwLock<state::State>>,
    queued_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>>,
) {
    let body = reqwest::get("https://cd-static.bamgrid.com/dp-117731241344/home.json")
        .await
        .unwrap();
    let x = body.json::<collection::Home>().await.unwrap();
    #[allow(irrefutable_let_patterns)]
    if let collection::Data::StandardCollection {
        collection_id: _,
        containers,
    } = x.data
    {
        for container in &containers {
            let row: Option<state::Row> = match &container.set {
                collection::Set::CuratedSet {
                    set_id: _,
                    text: _,
                    items: _,
                } => (&container.set).into(),
                collection::Set::SetRef {
                    ref_id,
                    ref_id_type: _,
                    ref_type: _,
                } => {
                    let ref_id = ref_id.clone();
                    let mut state = state.write().await;
                    state.queued_rows.push(ref_id);
                    None
                }
            };
            if let Some(row) = row {
                let mut state = state.write().await;
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
        let mut queued_images = queued_images.write().await;
        queued_images.push((0u32, img));
    }
}

pub async fn load_card_image(
    uri: String,
    state: state::AsyncState,
    queued_images: Arc<RwLock<Vec<(u32, image::DynamicImage)>>>,
    x: usize,
    y: usize,
    uid: u32,
) {
    let response = reqwest::get(uri).await.unwrap();
    let cursor = Cursor::new(response.bytes().await.unwrap());
    let img = image::io::Reader::new(cursor)
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    {
        let mut queued_images = queued_images.write().await;
        queued_images.push((uid, img));
    }
    {
        let mut state = state.write().await;
        state
            .rows
            .get_mut(y as usize)
            .and_then(|row| row.cards.get_mut(x as usize))
            .map(|card| card.image = state::CardImage::Texture(uid));
    }
}

pub async fn load_next_row(state: state::AsyncState) -> Option<()> {
    println!("Loading next row");
    let ref_id = {
        let mut state = state.write().await;
        state.queued_rows.pop()
    }?;
    let body = reqwest::get(format!(
        "https://cd-static.bamgrid.com/dp-117731241344/sets/{}.json",
        ref_id
    ))
    .await
    .unwrap();
    let x = body
        .json::<collection::RefSet>()
        .await
        .map_err(|e| println!("{} {}", ref_id, e))
        .unwrap();
    let row: Option<state::Row> = (&x.data.set).into();
    if let Some(row) = row {
        let mut state = state.write().await;
        state.rows.push(row);
    }
    println!("Done loading row");
    Some(())
}
