use crate::error::Result;
use crate::soundcloud::model::Format;
use crate::soundcloud::{model::Track, SoundcloudClient};
use crate::{ffmpeg, util};
use futures::stream::{FuturesUnordered, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_DOWNLOADS: usize = 3;

pub struct Downloader {
    pub client: SoundcloudClient,
    pub ffmpeg: ffmpeg::FFmpeg<PathBuf>,
    output_dir: PathBuf,
    semaphore: Arc<Semaphore>,
}

impl Downloader {
    pub fn new(
        client: SoundcloudClient,
        output: &PathBuf,
        ffmpeg: ffmpeg::FFmpeg<PathBuf>,
    ) -> Result<Self> {
        std::fs::create_dir_all(&output)?;
        tracing::info!("Using output directory: {:?}", output);

        Ok(Self {
            client,
            output_dir: output.clone(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
            ffmpeg,
        })
    }

    pub async fn download_track(&self, url: &str) -> Result<()> {
        tracing::info!("Fetching track from: {}", url);
        let track = self.client.track_from_url(url).await?;

        let track = self.client.fetch_track(track.id).await?;

        let path = self.process_track(&track).await?;
        tracing::info!(
            "Downloaded track {} to: {}",
            track.permalink_url,
            path.display()
        );

        Ok(())
    }

    pub async fn download_playlist(&self, id: u64) -> Result<()> {
        let playlist = self.client.fetch_playlist(id).await?;

        tracing::info!("Fetching playlist from: {}", playlist.permalink_url);

        let tracks_len = playlist.tracks.len();

        let mut futures = FuturesUnordered::new();

        for (i, track) in playlist.tracks.into_iter().enumerate() {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let progress = i + 1;

            futures.push(tokio::spawn(async move {
                let _permit = permit; // Keep permit alive for scope of task
                (track, progress)
            }));
        }

        while let Some(result) = futures.next().await {
            let (track, progress) = result.unwrap();

            let track_id = track.id;

            let track = match track.into_track() {
                Some(track) => track,
                None => match self.client.fetch_track(track_id).await {
                    Ok(track) => track,
                    Err(e) => {
                        tracing::error!("Failed to fetch track: {}", e);
                        continue;
                    }
                },
            };

            match self.process_track(&track).await {
                Ok(path) => {
                    tracing::info!(
                        "Downloaded track {} to: {} | ({}/{})",
                        track.permalink_url,
                        path.display(),
                        progress,
                        tracks_len,
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to download track: {}", e);
                }
            }
        }

        Ok(())
    }

    pub async fn download_likes(&self, skip: usize, limit: u32, chunk_size: u32) -> Result<()> {
        let me = self.client.get_me().await?;
        tracing::info!("Fetching likes for user: {}", me.username);

        let likes = self.client.get_likes(me.id, limit, chunk_size).await?;
        let total = likes.len().min(limit as usize);

        let mut futures = FuturesUnordered::new();

        for (i, like) in likes.into_iter().skip(skip).enumerate() {
            if i >= total {
                break;
            }

            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let track = like.track;
            let progress = i + 1 + skip;

            futures.push(tokio::spawn(async move {
                let _permit = permit; // Keep permit alive for scope of task
                (track, progress)
            }));
        }

        while let Some(result) = futures.next().await {
            let (track, progress) = result.unwrap();
            match self.process_track(&track).await {
                Ok(path) => {
                    tracing::info!(
                        "Downloaded track {} to: {} | ({}/{})",
                        track.permalink_url,
                        path.display(),
                        progress,
                        total
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to download track: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn process_track(&self, track: &Track) -> Result<PathBuf> {
        let (transcoding, audio) = self.client.download_track(track).await?;
        let thumbnail = self.client.download_cover(track).await?;

        let audio_ext = Self::mime_type_to_ext(&transcoding.format);

        let path = self.prepare_file_path(track, &audio_ext);

        self.process_audio(&path, audio, &audio_ext, thumbnail)
            .await?;

        Ok(path)
    }

    fn mime_type_to_ext(format: &Format) -> String {
        match format.mime_type.as_str().split(';').next().unwrap() {
            "audio/mpeg" => "mp3",
            "audio/mp4" | "audio/x-m4a" => "m4a",
            "audio/ogg" => "ogg",
            _ => "m4a",
        }
        .to_string()
    }

    fn prepare_file_path(&self, track: &Track, ext: &str) -> PathBuf {
        let username = util::sanitize(&track.user.username);
        let artist = if util::is_empty(&username) {
            track.user.permalink.clone()
        } else {
            track.user.username.clone()
        };

        let title = if util::is_empty(&track.title) {
            track.permalink.clone()
        } else {
            track.title.clone()
        };

        let filename = format!("{} - {}.{}", artist, title, ext);
        let safe_filename = util::sanitize(&filename);
        self.output_dir.join(safe_filename)
    }
}
