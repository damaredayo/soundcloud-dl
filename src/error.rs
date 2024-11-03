use thiserror::Error;

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

    #[error("ID3 tag error: {0}")]
    Id3(#[from] id3::Error),

    #[error("Recieved 429: Rate limited")]
    RateLimited,

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
