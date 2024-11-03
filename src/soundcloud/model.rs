use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Like {
    pub track: Track,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Track {
    pub artwork_url: Option<String>,
    pub permalink: String,
    pub permalink_url: String,
    pub title: String,
    pub media: Media,
    pub user: User,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Media {
    pub transcodings: Vec<Transcoding>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Transcoding {
    pub url: String,
    pub format: Format,
    pub quality: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Format {
    pub protocol: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetLikesResponse {
    pub collection: Vec<Like>,
    pub next_href: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AudioResponse {
    pub url: String, // url to audio to be downloaded
}
