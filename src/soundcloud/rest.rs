use crate::error::{AppError, Result};
use crate::soundcloud::model::{AudioResponse, GetLikesResponse, Like, Track, User};
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

use super::model::Playlist;
use super::{DownloadedFile, SoundcloudClient};

const API_BASE: &str = "https://api-v2.soundcloud.com/";
const ME_URL: &str = "https://api-v2.soundcloud.com/me";
const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(30);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(500);

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
    pub fn new(oauth: String) -> Self {
        Self {
            oauth,
            http_client: Client::new(),
        }
    }

    /// Makes an HTTP request with rate limiting and retries
    ///
    /// # Arguments
    /// * `req` - A reqwest request builder
    ///
    /// # Returns
    /// Result containing the response or an error
    async fn make_request(&self, req: reqwest::RequestBuilder) -> Result<Response> {
        let mut retries = 0;
        let mut delay = INITIAL_RETRY_DELAY;

        loop {
            match req
                .try_clone()
                .expect("request should be cloneable")
                .send()
                .await
            {
                Ok(resp) => {
                    match resp.status() {
                        StatusCode::TOO_MANY_REQUESTS => {
                            if retries >= MAX_RETRIES {
                                return Err(AppError::RateLimited);
                            }

                            tracing::warn!("Rate limited, waiting {:?} before retry", delay);
                            sleep(delay).await;

                            // Exponential backoff with jitter
                            delay = std::cmp::min(
                                delay * 2 + Duration::from_millis(rand::random::<u64>() % 1000),
                                MAX_RETRY_DELAY,
                            );
                            retries += 1;
                            continue;
                        }
                        _ => return Ok(resp),
                    }
                }
                Err(e) => return Err(AppError::Network(e)),
            }
        }
    }

    /// Fetches the current user's profile information
    ///
    /// # Returns
    /// Result containing [`User`] data or an error
    pub async fn get_me(&self) -> Result<User> {
        let resp = self
            .make_request(
                self.http_client
                    .get(ME_URL)
                    .header("Authorization", &self.oauth),
            )
            .await?;

        Ok(resp.json::<User>().await?)
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
    pub async fn get_likes(&self, user_id: u64, limit: u32, chunk_size: u32) -> Result<Vec<Like>> {
        let mut likes = Vec::new();
        let mut next_href = Some(make_track_likes_url(user_id, chunk_size));

        while let Some(url) = next_href {
            let res = self
                .make_request(
                    self.http_client
                        .get(&url)
                        .header("Authorization", &self.oauth),
                )
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

    /// Fetches track metadata from a SoundCloud URL
    ///
    /// # Arguments
    /// * `url` - A SoundCloud track URL
    ///
    /// # Returns
    /// Result containing [`Track`] metadata or an error. Errors can occur if:
    /// * The URL is invalid or inaccessible
    /// * The page doesn't contain valid hydration data
    /// * The track data cannot be parsed
    pub async fn track_from_url(&self, url: &str) -> Result<Track> {
        let resp = self
            .make_request(self.http_client.get(url))
            .await?
            .text()
            .await?;

        let hydration_data = resp
            .split("window.__sc_hydration = ")
            .nth(1)
            .and_then(|s| s.split(";</script>").next())
            .ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not find hydration data",
                ))
            })?;

        let hydration: serde_json::Value = serde_json::from_str(hydration_data)?;

        if let Some(track_data) = hydration
            .as_array()
            .and_then(|arr| arr.iter().find(|item| item["hydratable"] == "sound"))
            .and_then(|item| item.get("data"))
        {
            Ok(serde_json::from_value(track_data.clone())?)
        } else {
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not find track data",
            )))
        }
    }

    /// Fetches playlist metadata from a SoundCloud URL
    ///
    /// # Arguments
    /// * `url` - A SoundCloud playlist URL
    ///
    /// # Returns
    /// Result containing [`Playlist`] metadata or an error. Errors can occur if:
    /// * The URL is invalid or inaccessible
    /// * The page doesn't contain valid hydration data
    /// * The playlist data cannot be parsed
    pub async fn playlist_from_url(&self, url: &str) -> Result<Playlist> {
        let resp = self
            .make_request(self.http_client.get(url))
            .await?
            .text()
            .await?;

        let hydration_data = resp
            .split("window.__sc_hydration = ")
            .nth(1)
            .and_then(|s| s.split(";</script>").next())
            .ok_or_else(|| {
                AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not find hydration data",
                ))
            })?;

        let hydration: serde_json::Value = serde_json::from_str(hydration_data)?;

        if let Some(playlist_data) = hydration
            .as_array()
            .and_then(|arr| arr.iter().find(|item| item["hydratable"] == "playlist"))
            .and_then(|item| item.get("data"))
        {
            println!("{}", serde_json::to_string_pretty(&playlist_data).unwrap());
            Ok(serde_json::from_value(playlist_data.clone())?)
        } else {
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not find playlist data",
            )))
        }
    }

    pub async fn fetch_track(&self, id: u64) -> Result<Track> {
        let url = format!("{}tracks/{}", API_BASE, id);
        let resp = self
            .make_request(
                self.http_client
                    .get(&url)
                    .header("Authorization", &self.oauth),
            )
            .await?;

        Ok(resp.json::<Track>().await?)
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
            .make_request(
                self.http_client
                    .get(&transcoding.url)
                    .header("Authorization", format!("OAuth {}", self.oauth)),
            )
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
            .make_request(self.http_client.get(&resp.url))
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
    /// Result containing an optional DownloadedFile, None if no cover exists
    pub async fn download_cover(&self, track: &Track) -> Result<Option<DownloadedFile>> {
        match &track.artwork_url {
            Some(cover_url) => {
                let cover_url = cover_url.replace("-large", "-original");

                let file_ext = cover_url
                    .rsplit('/')
                    .next()
                    .and_then(|s| s.split('.').last())
                    .and_then(|s| s.split('?').next())
                    .unwrap_or("")
                    .to_string();

                let cover_bytes = self
                    .make_request(self.http_client.get(&cover_url))
                    .await?
                    .bytes()
                    .await?;

                Ok(Some(DownloadedFile {
                    data: cover_bytes,
                    file_ext,
                }))
            }
            None => Ok(None),
        }
    }
}
