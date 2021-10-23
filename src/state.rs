#[derive(Clone, Debug)]
pub struct State {
    pub rows: Vec<Row>,
    pub selected_card: (usize, usize),
    pub show_modal: bool,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub scroll: f32,
    pub scroll_target: f32,
    pub title: String,
    pub cards: Vec<Card>,
}

#[derive(Clone, Debug)]
pub struct Card {
    pub title: String,
    pub image: Option<bool>, // TODO
}
