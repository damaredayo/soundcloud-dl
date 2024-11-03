use bytes::Bytes;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

use crate::error::{AppError, Result};
use crate::soundcloud::DownloadedFile;

pub fn is_ffmpeg_installed() -> bool {
    Command::new("ffmpeg").arg("-version").output().is_ok()
}

pub fn init() -> Result<()> {
    if is_ffmpeg_installed() {
        Ok(())
    } else {
        Err(AppError::FFmpeg(
            "FFmpeg is not installed. Please install it first - see README.md for instructions"
                .to_string(),
        ))
    }
}

pub fn reformat_m4a<T: AsRef<Path>>(
    path: T,
    m4a: Bytes,
    thumbnail: Option<DownloadedFile>,
) -> Result<()> {
    let output_path = path
        .as_ref()
        .to_str()
        .ok_or_else(|| AppError::FFmpeg("Invalid output path".to_string()))?;

    // Save the audio data to a temporary file
    let tmp_audio_file = NamedTempFile::new()?;
    {
        let mut file = File::create(&tmp_audio_file)?;
        file.write_all(&m4a)?;
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.args(&["-y", "-i", tmp_audio_file.path().to_str().unwrap()]);

    if let Some(thumbnail) = thumbnail {
        // Save the thumbnail to a temporary file
        let tmp_thumbnail_file = NamedTempFile::new()?
            .into_temp_path()
            .with_extension(thumbnail.file_ext);
        {
            let mut file = File::create(&tmp_thumbnail_file)?;
            file.write_all(&thumbnail.data)?;
        }
        cmd.args(&[
            "-i",
            tmp_thumbnail_file.to_str().unwrap(),
            "-map",
            "0:a",
            "-map",
            "1:v",
            "-c:a",
            "copy",
            "-c:v",
            "copy",
            "-metadata:s:v",
            "title=Album cover",
            "-metadata:s:v",
            "comment=Cover (front)",
            "-disposition:v",
            "attached_pic",
        ]);
    } else {
        cmd.args(&["-c", "copy"]);
    }

    cmd.args(&[
        "-f",
        "mp4",
        "-movflags",
        "+faststart",
        "-loglevel",
        "error",
        output_path,
    ])
    .stdout(Stdio::null())
    .stderr(Stdio::inherit());

    let status = cmd.status()?;

    if !status.success() {
        return Err(AppError::FFmpeg(format!(
            "FFmpeg failed with exit code: {}",
            status.code().unwrap_or(1)
        )));
    }

    Ok(())
}
