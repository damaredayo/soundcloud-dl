use bytes::Bytes;
use id3::frame::{Picture, PictureType};
use id3::{TagLike, Version};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::downloader::Downloader;
use crate::error::{AppError, Result};
use crate::soundcloud::DownloadedFile;

impl Downloader {
    /// Processes and saves an MP3 file with optional thumbnail metadata
    ///
    /// # Arguments
    /// * `path` - Output path for the file
    /// * `audio` - Audio file bytes
    /// * `thumbnail` - Thumbnail image bytes
    /// * `thumbnail_ext` - Thumbnail image file extension
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn process_mp3<P: AsRef<Path>>(
        &self,
        path: P,
        audio: Bytes,
        thumbnail: Option<DownloadedFile>,
    ) -> Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&audio)?;
        writer.flush()?;

        if let Some(thumbnail) = thumbnail {
            let mut tag = id3::Tag::new();

            // Use more specific mime type and ensure proper formatting
            let mime_type = match thumbnail.file_ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                _ => "image/jpeg", // default to jpeg
            };

            let picture = Picture {
                mime_type: mime_type.to_string(),
                picture_type: PictureType::CoverFront,
                description: "Front Cover".to_string(),
                data: thumbnail.data.to_vec(),
            };
            tag.add_frame(picture);

            // Write with ID3v2.4 which has better support for large artwork
            tag.write_to_path(&path.as_ref(), Version::Id3v24)?;
        }

        Ok(())
    }

    /// Processes and saves an M4A file with optional thumbnail metadata and duration
    ///
    /// # Arguments
    /// * `path` - Output path for the file
    /// * `audio` - Audio file bytes
    /// * `thumbnail` - Thumbnail image bytes
    /// * `thumbnail_ext` - Thumbnail image file extension
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn process_m4a<P: AsRef<Path>>(
        &self,
        path: P,
        audio: Bytes,
        thumbnail: Option<DownloadedFile>,
    ) -> Result<()> {
        self.ffmpeg
            .reformat_m4a(audio, thumbnail, path.as_ref().to_path_buf())
    }

    /// Processes and saves an OGG file, currently without any additional metadata
    /// This may be extended in the future to support album art
    /// 
    /// # Arguments
    /// * `path` - Output path for the file
    /// * `audio` - Audio file bytes
    /// * `thumbnail` - Thumbnail image bytes
    /// * `thumbnail_ext` - Thumbnail image file extension
    /// 
    /// # Returns
    /// Result indicating success or failure
    pub async fn process_ogg<P: AsRef<Path>> (
        &self,
        path: P,
        audio: Bytes,
        _thumbnail: Option<DownloadedFile>,
    ) -> Result<()> {
        let file = File::create(path.as_ref())?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&audio)?;
        writer.flush()?;

        Ok(())
    }

    pub async fn process_m3u8<P: AsRef<Path>>(
        &self,
        path: P,
        playlist_data: Bytes,
        thumbnail: Option<DownloadedFile>,
    ) -> Result<()> {       
        // Use FFmpeg to convert the concatenated segments to m4a
        self.ffmpeg.process_m3u8(
            Bytes::from(playlist_data),
            thumbnail,
            path.as_ref().to_path_buf(),
        )?;

        Ok(())
    }

    /// Processes and saves an audio file with the appropriate format handler
    ///
    /// # Arguments
    /// * `path` - Output path for the file
    /// * `audio` - Audio file bytes
    /// * `audio_ext` - Audio file extension
    /// * `thumbnail` - Thumbnail image bytes
    /// * `thumbnail_ext` - Thumbnail image file extension
    ///
    /// # Returns
    /// Result indicating success or failure
    pub async fn process_audio<P: AsRef<Path>>(
        &self,
        path: P,
        audio: DownloadedFile,
        audio_ext: &str,
        thumbnail: Option<DownloadedFile>,
    ) -> Result<()> {
        if audio.file_ext == "m3u8" {
            return self.process_m3u8(path, audio.data, thumbnail).await;
        }

        match audio_ext {
            "mp3" => self.process_mp3(path, audio.data, thumbnail).await,
            "m4a" => self.process_m4a(path, audio.data, thumbnail).await,
            "ogg" => self.process_ogg(path, audio.data, thumbnail).await,
            _ => Err(AppError::Audio(format!(
                "Unsupported audio format: {}",
                audio_ext
            ))),
        }
    }
}
