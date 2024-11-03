use thiserror::Error;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFmpeg error: {0}")]
    FFmpeg(String),

    #[error("Audio processing error: {0}")]
    Audio(String),

    #[error("Rate limited by SoundCloud API")]
    RateLimited,

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("ID3 tag error: {0}")]
    Id3(#[from] id3::Error),
}
