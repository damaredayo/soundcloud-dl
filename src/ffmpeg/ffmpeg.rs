use bytes::Bytes;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

use super::download::get_default_ffmpeg_path;
use crate::error::{AppError, Result};
use crate::soundcloud::DownloadedFile;

#[cfg(target_os = "windows")]
const BINARY_NAME: &str = "ffmpeg.exe";
#[cfg(not(target_os = "windows"))]
const BINARY_NAME: &str = "ffmpeg";

/// FFmpeg wrapper for audio processing operations
pub struct FFmpeg<P>(P)
where
    P: AsRef<Path>;

impl FFmpeg<PathBuf> {
    /// Creates a new FFmpeg instance using the default installation path
    /// First checks PATH, then the default install location
    pub fn default() -> Result<Self> {
        which::which("ffmpeg").map(Self).or_else(|_| {
            let default = Self(get_default_ffmpeg_path().join(BINARY_NAME));
            if default.is_installed() {
                Ok(default)
            } else {
                Err(AppError::FFmpeg("FFmpeg not found".to_string()))
            }
        })
    }

    /// Creates a new FFmpeg instance from a specified path
    pub fn new(mut path: PathBuf) -> Result<Self> {
        if path.is_dir() {
            path.push(BINARY_NAME);
        }

        let ffmpeg = Self(path);
        if !ffmpeg.is_installed() {
            return Err(AppError::FFmpeg(format!(
                "FFmpeg not found at path: {}",
                ffmpeg.path().display()
            )));
        }
        Ok(ffmpeg)
    }
}

impl<P: AsRef<Path>> FFmpeg<P> {
    /// Returns reference to the FFmpeg binary path
    pub fn path(&self) -> &P {
        &self.0
    }

    /// Checks if FFmpeg is installed and callable
    pub fn is_installed(&self) -> bool {
        Command::new(self.path().as_ref())
            .arg("-version")
            .output()
            .is_ok()
    }

    /// Reformats M4A audio file with optional thumbnail
    pub fn reformat_m4a(
        &self,
        m4a: Bytes,
        thumbnail: Option<DownloadedFile>,
        output_path: P,
    ) -> Result<()> {
        let tmp_audio = NamedTempFile::with_suffix(".m4a")?;
        File::create(&tmp_audio)?.write_all(&m4a)?;

        let mut cmd = Command::new(self.path().as_ref());
        cmd.args(&["-y", "-i", tmp_audio.path().to_str().unwrap()])
            .args(&["-threads", "0"]); // Use all available CPU threads

        if let Some(thumb) = thumbnail {
            self.add_thumbnail_args(&mut cmd, &thumb)?;
        } else {
            cmd.args(&["-c", "copy"]);
        }

        self.run_command(cmd, output_path)
    }

    /// Processes M3U8 playlist data with optional thumbnail
    pub fn process_m3u8(
        &self,
        m3u8: Bytes,
        thumbnail: Option<DownloadedFile>,
        output_path: P,
    ) -> Result<()> {
        let tmp_playlist = NamedTempFile::with_suffix(".m3u8")?;
        File::create(&tmp_playlist)?.write_all(&m3u8)?;

        let mut cmd = Command::new(self.path().as_ref());
        cmd.arg("-y")
            .args(&["-protocol_whitelist", "file,http,https,tcp,tls"])
            .args(&["-threads", "0"])
            .args(&["-i", tmp_playlist.path().to_str().unwrap()]);

        if let Some(thumb) = thumbnail {
            self.add_thumbnail_args(&mut cmd, &thumb)?;
        } else {
            cmd.args(&["-c", "copy"]);
        }

        self.run_command(cmd, output_path)
    }

    /// Adds thumbnail metadata to FFmpeg command
    fn add_thumbnail_args(&self, cmd: &mut Command, thumb: &DownloadedFile) -> Result<()> {
        let tmp_thumb = NamedTempFile::new()?
            .into_temp_path()
            .with_extension(&thumb.file_ext);

        File::create(&tmp_thumb)?.write_all(&thumb.data)?;

        // Add thumbnail input
        cmd.args(&["-i", tmp_thumb.to_str().unwrap()]);

        // Specify which streams to include
        cmd.args(&[
            "-map", "0:a", // Audio from first input
            "-map", "1:v", // Video from second input
        ]);

        // Set codec options
        cmd.args(&[
            "-c:a", "copy", // Copy audio stream without re-encoding
            "-c:v", "copy", // Copy video stream without re-encoding
        ]);

        // Set metadata for the thumbnail
        cmd.args(&[
            "-metadata:s:v",
            "title=Album cover",
            "-metadata:s:v",
            "comment=Cover (front)",
            "-disposition:v",
            "attached_pic",
        ]);

        Ok(())
    }

    /// Runs FFmpeg command with common output arguments
    fn run_command(&self, mut cmd: Command, output_path: P) -> Result<()> {
        cmd.args(&[
            "-movflags",
            "+faststart",
            "-loglevel",
            "error",
            output_path.as_ref().to_str().unwrap(),
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
}
