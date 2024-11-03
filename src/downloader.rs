use crate::audio;
use crate::error::Result;
use crate::soundcloud::{model::Track, SoundcloudClient};
use futures::stream::{FuturesUnordered, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

const MAX_CONCURRENT_DOWNLOADS: usize = 3;

pub struct Downloader {
    client: SoundcloudClient,
    output_dir: PathBuf,
    semaphore: Arc<Semaphore>,
}

impl Downloader {
    pub fn new(oauth_token: String, output: &PathBuf) -> Result<Self> {
        let client = SoundcloudClient::new(oauth_token);

        std::fs::create_dir_all(&output)?;
        tracing::info!("Using output directory: {:?}", output);

        Ok(Self {
            client,
            output_dir: output.clone(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS)),
        })
    }

    pub async fn download_track(&self, url: &str) -> Result<()> {
        tracing::info!("Fetching track from: {}", url);
        let track = self.client.track_from_url(url).await?;

        let path = self.process_track(&track).await?;
        tracing::info!("Downloaded track {} to: {}", track.permalink_url, path.display());

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
        let audio = self.client.download_track(track).await?;
        let artwork = self.client.download_cover(track).await?;

        let path = self.prepare_file_path(track, &audio.file_ext);

        audio::process_audio(&path, audio.data, &audio.file_ext, artwork)?;

        Ok(path)
    }

    fn prepare_file_path(&self, track: &Track, ext: &str) -> PathBuf {

        let username = sanitize(&track.user.username);
        let artist = if is_empty(&username) {
            track.user.permalink.clone()
        } else {
            track.user.username.clone()
        };

        let title = if is_empty(&track.title) {
            track.permalink.clone()
        } else {
            track.title.clone()
        };

        let filename = format!("{} - {}.{}", artist, title, ext);
        let safe_filename = sanitize(&filename);
        self.output_dir.join(safe_filename)
    }
}

fn is_empty(s: &str) -> bool {
    s.replace('_', "").trim().is_empty()
}

fn sanitize(name: &str) -> String {
    const INVALID_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    let mut filename = name
        .chars()
        .map(|c| {
            if INVALID_CHARS.contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect::<String>();

    #[cfg(target_os = "windows")]
    {
        const RESERVED_NAMES: &[&str] = &[
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        if RESERVED_NAMES.contains(&filename.as_str()) {
            filename.push('_');
        }
    }

    if filename.len() > 255 {
        filename.truncate(255);
    }

    filename
}
