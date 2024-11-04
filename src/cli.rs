use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Your Soundcloud OAuth token (if not provided, will use stored token)
    #[arg(short, long)]
    pub auth: Option<String>,

    /// Save the provided OAuth token for future use
    #[arg(short = 't', long)]
    pub save_token: bool,

    /// Clear the stored OAuth token
    #[arg(long)]
    pub clear_token: bool,

    /// FFmpeg binary path (if not provided, will use `ffmpeg` from PATH or download it)
    #[arg(long)]
    pub ffmpeg_path: Option<String>,

    /// Assume yes to all prompts
    #[arg(short = 'y')]
    pub yes: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Track {
        /// Output directory for downloaded files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// URL of the track to download
        url: String,
    },
    Likes {
        /// Output directory for downloaded files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Number of likes to skip
        #[arg(short, long, default_value = "0")]
        skip: usize,

        /// Maximum number of likes to download
        #[arg(short, long, default_value = "10")]
        limit: u32,

        /// Number of likes to download in each chunk
        #[arg(long, default_value = "50")]
        chunk_size: u32,
    },
    Playlist {
        /// Output directory for downloaded files
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// URL of the playlist to download
        url: String,
    },
}

impl Cli {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn resolve_auth_token(&self) -> crate::error::Result<String> {
        use crate::config::Config;

        let mut config = Config::new()?;

        if self.clear_token {
            config.clear_oauth_token()?;
            tracing::info!("Cleared stored OAuth token");
        }

        let token = if let Some(token) = &self.auth {
            if self.save_token {
                config.save_oauth_token(token)?;
                tracing::info!("Saved OAuth token for future use");
            }
            token.clone()
        } else {
            config
                .get_oauth_token()?
                .ok_or_else(|| crate::error::AppError::Configuration(
                    "No OAuth token provided or stored. Use --auth to provide a token, or --auth with --save-token to store it".into()
                ))?
        };

        Ok(token)
    }
}
