use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Home {
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub enum Data {
    StandardCollection {
        #[serde(rename = "collectionId")]
        collection_id: String,
        containers: Vec<Container>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Container {
    pub set: Set,
    #[serde(rename = "type")]
    pub ttype: String,
    pub style: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Set {
    CuratedSet {
        #[serde(rename = "setId")]
        set_id: String,
        text: CuratedSetText,
        items: Vec<Item>,
    },
    SetRef {
        #[serde(rename = "refId")]
        ref_id: String,
        #[serde(rename = "refIdType")]
        ref_id_type: String,
        #[serde(rename = "refType")]
        ref_type: String,
    },
}

#[derive(Debug, Deserialize)]
pub struct CuratedSetText {
    title: Value,
}

impl CuratedSetText {
    pub fn get_name(&self) -> Option<String> {
        self.title["full"]["set"]["default"]["content"]
            .as_str()
            .map(|x| x.to_owned())
    }
}

#[derive(Debug, Deserialize)]
pub struct Item {
    #[serde(rename = "contentId")]
    content_id: Option<String>,
    text: Value,
    image: Image,
}

#[derive(Debug, Deserialize)]
pub struct Image {
    title: Option<TitleImageWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct TitleImageWrapper {
    #[serde(rename = "1.78")]
    image: TitleImage,
}

#[derive(Debug, Deserialize)]
pub struct TitleImage {}
