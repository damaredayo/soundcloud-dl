mod audio;
mod cli;
mod config;
mod downloader;
mod error;
mod ffmpeg;
mod soundcloud;

use cli::Cli;
use cli::Commands;
use downloader::Downloader;
use error::Result;

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

    if !ffmpeg::is_ffmpeg_installed() {
        tracing::error!("FFmpeg is not installed. Please install FFmpeg first - see README.md for instructions.");
        std::process::exit(1);
    }

    match &cli.command {
        Some(Commands::Track { url, output }) => {
            let oauth_token = cli.resolve_auth_token()?;
            let downloader = Downloader::new(oauth_token, &output)?;
            downloader.download_track(url).await?;
            tracing::info!("Track download completed successfully!");
        }
        Some(Commands::Likes {
            skip,
            limit,
            chunk_size,
            output,
        }) => {
            let oauth_token = cli.resolve_auth_token()?;
            let downloader = Downloader::new(oauth_token, &output)?;
            downloader
                .download_likes(*skip, *limit, *chunk_size)
                .await?;
            tracing::info!("Likes download completed successfully!");
        }
        None => {
            tracing::error!("No command specified. Use --help to see available commands.");
            std::process::exit(1);
        }
    }

    Ok(())
}
