use crate::model::collection::{Item, Set};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct State {
    pub rows: Vec<Row>,
    pub selected_card: (usize, usize),
    pub show_modal: bool,
    pub scroll: f32,
    pub scroll_target: f32,
}

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
}

impl From<&Item> for Option<Card> {
    fn from(item: &Item) -> Option<Card> {
        Some(Card {
            title: item.text.get_name()?,
            image: CardImage::URI(item.image.tile.as_ref()?.image.get_uri()?),
            size: 0.,
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
