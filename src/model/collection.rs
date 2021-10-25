use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Home {
    pub data: Data,
}

#[derive(Debug, Deserialize)]
pub struct RefSet {
    pub data: RefSetData,
}

#[derive(Debug, Deserialize)]
pub struct RefSetData {
    #[serde(alias = "CuratedSet")]
    #[serde(alias = "PersonalizedCuratedSet")]
    pub set: Set,
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
    pub ttype: Option<String>,
    pub style: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Set {
    #[serde(alias = "PersonalizedCuratedSet")]
    CuratedSet {
        #[serde(rename = "setId")]
        set_id: String,
        text: SetText,
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
pub struct SetText {
    pub title: Value,
}

impl SetText {
    pub fn get_name(&self) -> Option<String> {
        Some(
            self.title
                .get("full")?
                .get("set")?
                .get("default")?
                .get("content")?
                .as_str()?
                .to_owned(),
        )
    }
}

#[derive(Debug, Deserialize)]
pub struct Item {
    #[serde(rename = "contentId")]
    pub content_id: Option<String>,
    pub text: ItemText,
    pub image: Image,
}

#[derive(Debug, Deserialize)]
pub struct ItemText {
    pub title: ItemTextTitle,
}

#[derive(Debug, Deserialize)]
pub struct ItemTextTitle {
    pub full: ItemTextTitleFull,
}

#[derive(Debug, Deserialize)]
pub enum ItemTextTitleFull {
    #[serde(rename = "series")]
    Series { default: ItemTextTitleFullDefault },
    #[serde(rename = "program")]
    Program { default: ItemTextTitleFullDefault },
    #[serde(rename = "collection")]
    Collection { default: ItemTextTitleFullDefault },
}

impl ItemTextTitleFull {
    pub fn get_default(&self) -> &ItemTextTitleFullDefault {
        match self {
            ItemTextTitleFull::Series { default } => default,
            ItemTextTitleFull::Program { default } => default,
            ItemTextTitleFull::Collection { default } => default,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ItemTextTitleFullDefault {
    pub content: String,
}

impl ItemText {
    pub fn get_name(&self) -> Option<String> {
        Some(self.title.full.get_default().content.clone())
    }
}

#[derive(Debug, Deserialize)]
pub struct Image {
    pub tile: Option<TileImageWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct TileImageWrapper {
    #[serde(rename = "1.78")]
    pub image: TitleImage,
}

#[derive(Debug, Deserialize)]
pub enum TitleImage {
    #[serde(rename = "series")]
    Series { default: Value },
    #[serde(rename = "program")]
    Program { default: Value },
    #[serde(rename = "default")]
    Default { default: Value },
}

impl TitleImage {
    pub fn get_uri(&self) -> Option<String> {
        let default = match self {
            TitleImage::Series { default } => default,
            TitleImage::Program { default } => default,
            TitleImage::Default { default } => default,
        };
        Some(default.get("url")?.as_str()?.to_owned())
    }
}
