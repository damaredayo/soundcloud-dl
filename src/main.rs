mod audio;
mod cli;
mod config;
mod downloader;
mod error;
mod ffmpeg;
mod soundcloud;
mod util;

use std::path::PathBuf;

use cli::Cli;
use cli::Commands;
use downloader::Downloader;
use error::Result;
use soundcloud::SoundcloudClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    let handled_token = if cli.clear_token {
        let config = config::Config::new()?;
        config.clear_oauth_token()?;
        tracing::info!("Cleared stored OAuth token");
        true
    } else {
        false
    };

    if let Some(token) = &cli.auth {
        if cli.save_token {
            let mut config = config::Config::new()?;
            config.save_oauth_token(token)?;
            tracing::info!("Saved OAuth token for future use");
            if cli.command.is_none() {
                return Ok(());
            }
        }
    }

    if handled_token && cli.command.is_none() {
        return Ok(());
    }

    let ffmpeg = match cli.ffmpeg_path.as_ref().map_or_else(
        || ffmpeg::FFmpeg::default(),
        |path| ffmpeg::FFmpeg::new(PathBuf::from(path)),
    ) {
        Ok(ffmpeg) => ffmpeg,
        Err(_) if cli.yes || prompt("FFmpeg is not installed. Do you want to install it?") => {
            let path = ffmpeg::download_ffmpeg(cli.ffmpeg_path.as_ref()).await?;
            ffmpeg::FFmpeg::new(path)?
        }
        _ => {
            tracing::error!("FFmpeg is required to run this program. Exiting.");
            std::process::exit(1);
        }
    };

    let oauth_token = cli.resolve_auth_token()?;

    let client = SoundcloudClient::new(oauth_token);

    match &cli.command {
        Some(Commands::Track { url, output }) => {
            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader.download_track(url).await?;
            tracing::info!("Track download completed successfully!");
        }
        Some(Commands::Likes {
            skip,
            limit,
            chunk_size,
            output,
        }) => {
            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader
                .download_likes(*skip, *limit, *chunk_size)
                .await?;
            tracing::info!("Likes download completed successfully!");
        }
        Some(Commands::Playlist { url, output }) => {
            let playlist = client.playlist_from_url(url).await?;

            let default_title = if playlist.title.is_empty() {
                playlist.permalink.clone()
            } else {
                playlist.title.clone()
            };

            let default_path = PathBuf::from(".").join(util::sanitize(&default_title));
            let output = output.as_ref().unwrap_or(&default_path);

            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader.download_playlist(playlist).await?;

            tracing::info!("Playlist download completed successfully!");
        }
        None => {
            tracing::error!("No command specified. Use --help to see available commands.");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn prompt(msg: &str) -> bool {
    use std::io::{self, Write};

    print!("{} [Y/n]: ", msg);
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().to_lowercase() == "y" || input.trim().is_empty()
}
