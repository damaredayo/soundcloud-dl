mod audio;
mod error;
mod ffmpeg;
mod soundcloud;

use clap::Parser;
use error::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Your Soundcloud OAuth token
    #[arg(short, long)]
    auth: String,

    /// Output directory for downloaded files
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Number of tracks to skip
    #[arg(short, long, default_value = "0")]
    offset: i32,

    /// Number of tracks to download
    #[arg(short, long, default_value = "10")]
    limit: i32,

    /// Chunk size for API requests
    #[arg(long, default_value = "25")]
    chunk_size: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    // Check FFmpeg is available
    if !ffmpeg::is_ffmpeg_installed() {
        eprintln!("FFmpeg is not installed. Please install FFmpeg first - see README.md for instructions.");
        std::process::exit(1);
    }

    // Initialize FFmpeg
    ffmpeg::init().expect("Failed to initialize FFmpeg");
    tracing::info!("FFmpeg initialized successfully");

    let client = soundcloud::SoundcloudClient::new(Some(cli.auth.clone()))
        .expect("Failed to create Soundcloud client");

    let me = client.get_me().await.expect("Failed to get user info");
    tracing::info!("Fetched user info for {}", me.username);

    if let Some(ref output) = Some(cli.output.clone()) {
        std::fs::create_dir_all(&output).expect("Failed to create output directory");
        tracing::info!("Created output directory at {:?}", output);
    }

    let likes = client
        .get_likes(me.id, cli.limit, cli.chunk_size)
        .await
        .expect("Failed to get likes");

    for (i, like) in likes.into_iter().skip(cli.offset as usize).enumerate() {
        let audio = match client.download_track(&like.track).await {
            Ok(dl) => dl,
            Err(e) => {
                tracing::error!("Failed to download track {}: {}", like.track.title, e);
                continue;
            }
        };

        let artwork = client.download_cover(&like.track).await.ok();

        let artist = match like.track.user.username.as_str() {
            "" => like.track.user.id.to_string(),
            username => username.to_string(),
        };

        let title = match like.track.title.as_str() {
            "" => like.track.permalink,
            title => title.to_string(),
        };

        let filename =
            make_filename_os_friendly(format!("{} - {}.{}", artist, title, audio.file_ext));

        let path = match Some(cli.output.clone()) {
            Some(ref output) => output.join(filename),
            None => std::path::PathBuf::from(filename),
        };

        match audio::process_audio(&path, audio.data, &audio.file_ext, artwork) {
            Ok(_) => tracing::info!(
                "Downloaded track {} to {} | ({}/{})",
                like.track.permalink_url,
                path.display(),
                i + 1,
                cli.limit
            ),
            Err(e) => tracing::error!("Failed to write track {}: {}", like.track.permalink_url, e),
        }

        if i + 1 >= cli.limit as usize {
            break;
        }
    }

    tracing::info!("Done!");

    Ok(())
}

/// Makes a filename safe for use on the operating system by removing invalid characters
///
/// # Arguments
/// * `input` - The original filename
///
/// # Returns
/// A sanitized filename string
fn make_filename_os_friendly(input: String) -> String {
    const INVALID_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

    let mut filename: String = input
        .chars()
        .filter_map(|c| {
            if INVALID_CHARS.contains(&c) {
                Some('_')
            } else if c.is_whitespace() {
                Some('_')
            } else {
                Some(c)
            }
        })
        .collect();

    #[cfg(target_os = "windows")]
    {
        const RESERVED_NAMES: [&str; 22] = [
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
