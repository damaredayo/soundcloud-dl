use crate::error::{AppError, Result};
use crate::soundcloud::model::{AudioResponse, GetLikesResponse, Like, Track, User};
use reqwest::{Client, Response, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

use super::model::{Playlist, Transcoding};
use super::{DownloadedFile, SoundcloudClient};

const API_BASE: &str = "https://api-v2.soundcloud.com/";
const ME_URL: &str = "https://api-v2.soundcloud.com/me";
const MAX_RETRIES: u32 = 5;
const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(30);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(500);

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
        let mut next_href = Some(format!(
            "{}users/{}/track_likes?limit={}",
            API_BASE, user_id, limit
        ));

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
                    next_href = Some(format!(
                        "{}users/{}/track_likes?limit={}",
                        API_BASE, user_id, remaining
                    ));
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

    pub async fn fetch_playlist(&self, id: u64) -> Result<Playlist> {
        let url = format!("{}playlists/{}", API_BASE, id);
        let resp = self
            .make_request(
                self.http_client
                    .get(&url)
                    .header("Authorization", &self.oauth),
            )
            .await?;

        Ok(resp.json::<Playlist>().await?)
    }

    /// Downloads a track's audio file
    ///
    /// # Arguments
    /// * `track` - [`Track`] metadata containing download information
    ///
    /// # Returns
    /// Result containing a tuple of (audio bytes, file extension) or an error
    pub async fn download_track<'t>(
        &self,
        track: &'t Track,
    ) -> Result<(&'t Transcoding, DownloadedFile)> {
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
                    .find(|t| t.format.protocol == "hls" && t.quality == "hq")
            })
            .or_else(|| {
                track
                    .media
                    .transcodings
                    .iter()
                    .find(|t| t.format.protocol == "progressive" && t.quality == "sq")
            })
            .or_else(|| {
                track
                    .media
                    .transcodings
                    .iter()
                    .find(|t| t.format.protocol == "hls" && t.quality == "sq")
            })
            .ok_or_else(|| AppError::Audio("No suitable transcodings found".to_string()))?;

        let resp = self
            .make_request(
                self.http_client
                    .get(&transcoding.url)
                    .header("Authorization", format!("OAuth {}", self.oauth)),
            )
            .await?
            .json::<AudioResponse>()
            .await?;

        Ok((transcoding, self.download_bytes(&resp.url).await?))
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

                self.download_bytes(&cover_url).await.map(|file| Some(file))
            }
            None => Ok(None),
        }
    }

    pub async fn download_bytes(&self, url: &str) -> Result<DownloadedFile> {
        let file_ext = url
            .rsplit('/')
            .next()
            .and_then(|s| s.split('.').last())
            .and_then(|s| s.split('?').next())
            .unwrap_or("")
            .to_string();

        let bytes = self
            .make_request(
                self.http_client
                    .get(url)
                    .header("Authorization", &self.oauth),
            )
            .await?
            .bytes()
            .await?;

        Ok(DownloadedFile {
            data: bytes,
            file_ext,
        })
    }

    pub async fn resolve_user(&self, username: Option<String>) -> Result<User> {
        if username.is_none() {
            return self.get_me().await;
        }

        let url = format!("https://soundcloud.com/{}", username.unwrap());

        let resp = self
            .make_request(self.http_client.get(&url))
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

        if let Some(user_data) = hydration
            .as_array()
            .and_then(|arr| arr.iter().find(|item| item["hydratable"] == "user"))
            .and_then(|item| item.get("data"))
        {
            Ok(serde_json::from_value(user_data.clone())?)
        } else {
            Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not find user data",
            )))
        }
    }
}
