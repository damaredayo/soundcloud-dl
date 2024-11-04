use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Like {
    pub track: Track,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Playlist {
    pub id: u64,
    pub permalink: String,
    pub permalink_url: String,
    pub title: String,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PlaylistTrack {
    pub id: u64,

    pub artwork_url: Option<String>,
    pub permalink: Option<String>,
    pub permalink_url: Option<String>,
    pub title: Option<String>,
    pub media: Option<Media>,
    pub user: Option<User>,
}

impl PlaylistTrack {
    pub fn into_track(self) -> Option<Track> {
        let PlaylistTrack {
            artwork_url,
            permalink,
            permalink_url,
            title,
            media,
            user,
            ..
        } = self;

        let media = media?;
        let user = user?;

        Some(Track {
            artwork_url,
            permalink: permalink?,
            permalink_url: permalink_url?,
            title: title?,
            media,
            user,
        })
    }
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
    pub mime_type: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub permalink: String,
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
