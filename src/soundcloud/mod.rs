mod model;
mod rest;

#[derive(Debug, Clone)]
pub struct SoundcloudClient {
    http_client: reqwest::Client,
    oauth: String,
}

pub struct DownloadedFile {
    pub data: bytes::Bytes,
    pub file_ext: String,
}
