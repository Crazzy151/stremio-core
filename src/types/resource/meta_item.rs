use crate::types::resource::Stream;
use chrono::{DateTime, Utc};
use derivative::Derivative;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaItem {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub name: String,
    pub poster: Option<String>,
    pub background: Option<String>,
    pub logo: Option<String>,
    pub popularity: Option<f64>,
    pub description: Option<String>,
    pub release_info: Option<String>,
    pub runtime: Option<String>,
    pub released: Option<DateTime<Utc>>,
    #[serde(default)]
    pub poster_shape: PosterShape,
    #[serde(default)]
    pub videos: Vec<Video>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub trailers: Vec<Stream>,
    #[serde(default)]
    pub behavior_hints: MetaItemBehaviorHints,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaItemPreview {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default)]
    pub name: String,
    pub poster: Option<String>,
    pub logo: Option<String>,
    pub description: Option<String>,
    pub release_info: Option<String>,
    pub runtime: Option<String>,
    pub released: Option<DateTime<Utc>>,
    #[serde(default)]
    pub poster_shape: PosterShape,
    #[serde(default)]
    pub trailers: Vec<Stream>,
    #[serde(default)]
    pub behavior_hints: MetaItemBehaviorHints,
}

#[derive(Derivative, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[derivative(Default)]
#[serde(rename_all = "camelCase")]
pub enum PosterShape {
    Square,
    Landscape,
    #[derivative(Default)]
    #[serde(other)]
    Poster,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeriesInfo {
    pub season: u32,
    pub episode: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    #[serde(alias = "name")]
    pub title: String,
    pub released: DateTime<Utc>,
    pub overview: Option<String>,
    pub thumbnail: Option<String>,
    #[serde(default)]
    pub streams: Vec<Stream>,
    #[serde(flatten)]
    pub series_info: Option<SeriesInfo>,
    #[serde(default)]
    pub trailers: Vec<Stream>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Link {
    pub name: String,
    pub category: String,
    pub url: Url,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaItemBehaviorHints {
    pub default_video_id: Option<String>,
    pub featured_video_id: Option<String>,
}