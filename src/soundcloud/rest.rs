use crate::error::{AppError, Result};
use crate::soundcloud::model::{AudioResponse, GetLikesResponse, Like, Track, User};
use reqwest::Client;

use super::{DownloadedFile, SoundcloudClient};

// Remove this line since we're using our custom Result type
// type Result<T> = std::result::Result<T, Box<dyn Error>>;

const API_BASE: &str = "https://api-v2.soundcloud.com/";
const ME_URL: &str = "https://api-v2.soundcloud.com/me";

/// Creates a URL for fetching a user's track likes
///
/// # Arguments
/// * `user_id` - The ID of the user
/// * `limit` - Maximum number of likes to fetch
///
/// # Returns
/// A formatted URL string for the track likes endpoint
fn make_track_likes_url(user_id: u64, limit: u32) -> String {
    format!("{}users/{}/track_likes?limit={}", API_BASE, user_id, limit)
}

impl SoundcloudClient {
    /// Creates a new SoundCloud client instance
    ///
    /// # Arguments
    /// * `oauth` - Optional OAuth token for authentication
    ///
    /// # Returns
    /// Some([`SoundcloudClient`]) if OAuth token is provided, None otherwise
    pub fn new(oauth: Option<String>) -> Option<Self> {
        oauth.map(|oauth| Self {
            http_client: Client::new(),
            oauth,
        })
    }

    /// Fetches the current user's profile information
    ///
    /// # Returns
    /// Result containing [`User`] data or an error
    pub async fn get_me(&self) -> Result<User> {
        println!("Getting user with oauth: {}", self.oauth);
        let resp = self
            .http_client
            .get(ME_URL)
            .header("Authorization", &self.oauth)
            .send()
            .await?;

        let user = resp.json::<User>().await?;

        Ok(user)
    }

    /// Fetches a user's liked tracks
    ///
    /// # Arguments
    /// * `user_id` - The ID of the user
    /// * `limit` - Maximum number of [`Like`]s to fetch
    /// * `chunk_size` - Number of [`Like`]s to fetch per request
    ///
    /// # Returns
    /// Result containing a vector of [`Like`]s or an error
    pub async fn get_likes(&self, user_id: u64, limit: i32, chunk_size: u32) -> Result<Vec<Like>> {
        let mut likes = Vec::new();
        let mut next_href = Some(make_track_likes_url(user_id, chunk_size));

        while let Some(url) = next_href {
            let res = self
                .http_client
                .get(&url)
                .header("Authorization", &self.oauth)
                .send()
                .await?
                .json::<GetLikesResponse>()
                .await?;
            likes.extend(res.collection);

            next_href = res.next_href;

            if likes.len() >= limit as usize {
                likes.truncate(limit as usize);
                break;
            }

            if next_href.is_some() {
                let remaining = limit as usize - likes.len();
                if remaining < chunk_size as usize {
                    next_href = Some(make_track_likes_url(user_id, remaining as u32));
                }
            }
        }

        Ok(likes)
    }

    /// Downloads a track's audio file
    ///
    /// # Arguments
    /// * `track` - [`Track`] metadata containing download information
    ///
    /// # Returns
    /// Result containing a tuple of (audio bytes, file extension) or an error
    pub async fn download_track(&self, track: &Track) -> Result<DownloadedFile> {
        let transcoding = track
            .media
            .transcodings
            .iter()
            .find(|t| t.format.protocol == "progressive" && t.quality == "hq")
            .or_else(|| {
                track
                    .media
                    .transcodings
                    .iter()
                    .find(|t| t.format.protocol == "progressive" && t.quality == "sq")
            })
            .ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "No progressive transcoding found",
                ))
            })?;

        let resp = self
            .http_client
            .get(&transcoding.url)
            .header("Authorization", format!("OAuth {}", self.oauth))
            .send()
            .await?
            .json::<AudioResponse>()
            .await?;

        let file_ext = resp
            .url
            .rsplit('/')
            .next()
            .and_then(|s| s.split('.').last())
            .and_then(|s| s.split('?').next())
            .unwrap_or("")
            .to_string();

        let audio_file = self
            .http_client
            .get(&resp.url)
            .send()
            .await?
            .bytes()
            .await?;

        Ok(DownloadedFile {
            data: audio_file,
            file_ext,
        })
    }

    /// Downloads a track's cover artwork
    ///
    /// # Arguments
    /// * `track` - [`Track`] metadata containing artwork information
    ///
    /// # Returns
    /// Result containing a tuple of (image bytes, file extension) or an error
    pub async fn download_cover(&self, track: &Track) -> Result<DownloadedFile> {
        if let Some(cover_url) = &track.artwork_url {
            let cover_url = cover_url.replace("-large", "-original");

            let file_ext = cover_url
                .rsplit('/')
                .next()
                .and_then(|s| s.split('.').last())
                .and_then(|s| s.split('?').next())
                .unwrap_or("")
                .to_string();

            let cover_bytes = self
                .http_client
                .get(&cover_url)
                .send()
                .await?
                .bytes()
                .await?;

            Ok(DownloadedFile {
                data: cover_bytes,
                file_ext,
            })
        } else {
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No cover found",
            )))
        }
    }
}
