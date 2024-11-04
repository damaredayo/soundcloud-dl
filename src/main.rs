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
use ffmpeg::FFmpeg;
use soundcloud::SoundcloudClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    let mut config = config::Config::new()?;

    if cli.command.is_none() && cli.config_init(&mut config)? {
        return Ok(());
    }

    let ffmpeg = cli.resolve_ffmpeg_path().await?;

    let oauth_token = cli.resolve_auth_token(&config)?;

    let client = SoundcloudClient::new(oauth_token);

    let output = cli
        .resolve_output_dir()
        .ok_or_else(|| error::AppError::Configuration("Output directory not set".to_string()))?;

    handle_command(&cli, output, client, ffmpeg).await?;

    Ok(())
}

async fn handle_command(
    cli: &Cli,
    output: PathBuf,
    client: SoundcloudClient,
    ffmpeg: FFmpeg<PathBuf>,
) -> Result<()> {
    match &cli.command {
        Some(Commands::Track { url, .. }) => {
            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader.download_track(url).await?;
            tracing::info!("Track download completed successfully!");
        }
        Some(Commands::Likes {
            skip,
            limit,
            chunk_size,
            ..
        }) => {
            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader
                .download_likes(*skip, *limit, *chunk_size)
                .await?;
            tracing::info!("Likes download completed successfully!");
        }
        Some(Commands::Playlist { url, .. }) => {
            let playlist = client.playlist_from_url(url).await?;

            let playlist_title = if playlist.title.is_empty() {
                playlist.permalink.clone()
            } else {
                playlist.title.clone()
            };

            let output = output.join(playlist_title);

            let downloader = Downloader::new(client, &output, ffmpeg)?;
            downloader.download_playlist(playlist.id).await?;

            tracing::info!("Playlist download completed successfully!");
        }
        None => {
            tracing::error!("No command specified. Use --help to see available commands.");
            std::process::exit(1);
        }
    };

    Ok(())
}
