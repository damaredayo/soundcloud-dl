use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{
    config::Config,
    error::{AppError, Result},
    ffmpeg::{self, FFmpeg},
    util,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Your Soundcloud OAuth token (if not provided, will use stored token)
    #[arg(short, long)]
    pub auth: Option<String>,

    /// Config file path (default: $HOME/.config/soundcloud-dl.toml or %%APPDATA%%\damaredayo\soundcloud-dl.toml)
    #[arg(long)]
    pub config: Option<String>,

    /// Clear the stored OAuth token
    #[arg(long)]
    pub clear_token: bool,

    /// FFmpeg binary path (if not provided, will use `ffmpeg` from PATH or download it)
    #[arg(long)]
    pub ffmpeg_path: Option<String>,

    /// Save the provided OAuth token for future use
    #[arg(short = 't', long)]
    pub save_token: bool,

    /// Assume yes to all prompts
    #[arg(short = 'y')]
    pub yes: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download a single track
    Track {
        /// Output directory for downloaded files
        #[arg(short, long, default_value = ".")]
        output: Option<PathBuf>,

        /// URL of the track to download
        url: String,
    },
    /// Download liked tracks
    Likes {
        /// Output directory for downloaded files
        #[arg(short, long, default_value = ".")]
        output: Option<PathBuf>,

        /// Number of likes to skip
        #[arg(short, long, default_value = "0")]
        skip: usize,

        /// Maximum number of likes to download
        #[arg(short, long, default_value = "10")]
        limit: u32,

        /// Number of likes to download in each chunk
        #[arg(long, default_value = "50")]
        chunk_size: u32,

        /// Soundcloud username to download likes from
        user: Option<String>,
    },
    /// Download a playlist
    Playlist {
        /// Output directory for downloaded files
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// URL of the playlist to download
        url: String,
    },
}

impl Commands {
    pub fn output_dir(&self) -> Option<&PathBuf> {
        match self {
            Self::Track { output, .. } => output.as_ref(),
            Self::Likes { output, .. } => output.as_ref(),
            Self::Playlist { output, .. } => output.as_ref(),
        }
    }
}

impl Cli {
    pub fn parse() -> Self {
        Parser::parse()
    }

    pub fn resolve_auth_token(&self, config: &Config) -> Result<String> {
        match self
            .auth
            .as_ref()
            .map_or_else(|| config.get_oauth_token(), |token| Ok(Some(token.clone())))
        {
            Ok(Some(token)) => Ok(token),
            _ => Err(AppError::Configuration(
                "OAuth token is required to run this program. Exiting.".into(),
            )),
        }
    }

    pub async fn resolve_ffmpeg_path(&self) -> Result<FFmpeg<PathBuf>> {
        let ffmpeg = match self.ffmpeg_path.as_ref() {
            Some(path) => ffmpeg::FFmpeg::new(PathBuf::from(path)),
            None => ffmpeg::FFmpeg::default(),
        };

        match ffmpeg {
            Ok(ffmpeg) => Ok(ffmpeg),
            Err(_)
                if self.yes
                    || util::prompt("FFmpeg is not installed. Do you want to install it?") =>
            {
                let path = ffmpeg::download_ffmpeg(self.ffmpeg_path.as_ref()).await?;
                Ok(ffmpeg::FFmpeg::new(path)?)
            }
            Err(_) => Err(AppError::FFmpeg(
                "FFmpeg is required to run this program. Exiting.".into(),
            )),
        }
    }

    pub fn resolve_output_dir(&self) -> Option<PathBuf> {
        self.command
            .as_ref()
            .and_then(|c| c.output_dir())
            .map(|p| p.clone())
    }

    pub fn config_init(&self, config: &mut Config) -> Result<bool> {
        let mut action_performed = false;
        if let Some(auth) = &self.auth {
            if self.save_token {
                config.save_oauth_token(&auth)?;
                tracing::info!("OAuth token saved successfully!");

                action_performed = true;
            }
        }

        if self.clear_token {
            config.clear_oauth_token()?;
            tracing::info!("OAuth token cleared successfully!");

            action_performed = true;
        }

        Ok(action_performed)
    }
}
