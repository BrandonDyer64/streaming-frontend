use tokio::sync::RwLock;

use crate::model::collection::{Item, Set};
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, Debug)]
pub struct State {
    pub rows: Vec<Row>,
    pub queued_rows: Vec<String>,
    pub is_loading_row: bool,
    pub selected_card: (usize, usize),
    pub show_modal: bool,
    pub scroll: f32,
    pub scroll_target: f32,
}

impl State {
    pub fn new() -> Self {
        State {
            rows: Vec::new(),
            queued_rows: Vec::new(),
            is_loading_row: false,
            selected_card: (0, 0),
            show_modal: false,
            scroll: 0.,
            scroll_target: 0.,
        }
    }
}

pub type AsyncState = Arc<RwLock<State>>;

#[derive(Clone, Debug)]
pub struct Row {
    pub scroll: f32,
    pub scroll_target: f32,
    pub text_height: f32,
    pub text_height_target: f32,
    pub title: String,
    pub cards: Vec<Card>,
}

impl From<&Set> for Option<Row> {
    fn from(set: &Set) -> Option<Row> {
        if let Set::CuratedSet {
            set_id: _,
            text,
            items,
        } = &set
        {
            Some(Row {
                scroll: 0.,
                scroll_target: 0.,
                text_height: 0.,
                text_height_target: 0.,
                title: text.get_name()?,
                cards: items.iter().filter_map(|item| item.into()).collect(),
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Card {
    pub title: String,
    pub image: CardImage,
    pub size: f32,
    pub ratings: Vec<String>,
    pub releases: Vec<String>,
}

impl From<&Item> for Option<Card> {
    fn from(item: &Item) -> Option<Card> {
        Some(Card {
            title: item.text.get_name()?,
            image: CardImage::URI(item.image.tile.as_ref()?.image.get_uri()?),
            size: 0.,
            ratings: item
                .ratings
                .as_ref()
                .map(|ratings| ratings.iter().filter_map(|r| r.value.clone()).collect())
                .unwrap_or(vec![]),
            releases: item
                .releases
                .as_ref()
                .map(|releases| {
                    releases
                        .iter()
                        .filter_map(|r| r.release_date.clone())
                        .collect()
                })
                .unwrap_or(vec![]),
        })
    }
}

#[derive(Clone)]
pub enum CardImage {
    URI(String),
    Loading(u32),
    Texture(u32),
    Failure,
}

impl Debug for CardImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::URI(arg0) => f.debug_tuple("URI").field(arg0).finish(),
            Self::Loading(_) => f.write_str("Loading"),
            Self::Texture(_) => f.write_str("Texture"),
            Self::Failure => f.write_str("failure"),
        }
    }
}
