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
    /// Will append binary name if path is a directory
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
    /// Handles temporary file creation and cleanup automatically
    pub fn reformat_m4a(
        &self,
        m4a: Bytes,
        thumbnail: Option<DownloadedFile>,
        output_path: P,
    ) -> Result<()> {
        let tmp_audio = self.create_temp_file(&m4a)?;

        let mut cmd = Command::new(self.path().as_ref());
        cmd.args(&["-y", "-i", tmp_audio.path().to_str().unwrap()]);

        if let Some(thumb) = thumbnail {
            self.add_thumbnail_args(&mut cmd, &thumb)?;
        } else {
            cmd.args(&["-c", "copy"]);
        }

        self.run_command(cmd, output_path)
    }

    fn create_temp_file(&self, data: &Bytes) -> Result<NamedTempFile> {
        let tmp = NamedTempFile::new()?;
        File::create(&tmp)?.write_all(data)?;
        Ok(tmp)
    }

    fn add_thumbnail_args(&self, cmd: &mut Command, thumb: &DownloadedFile) -> Result<()> {
        let tmp_thumb = NamedTempFile::new()?
            .into_temp_path()
            .with_extension(&thumb.file_ext);

        File::create(&tmp_thumb)?.write_all(&thumb.data)?;

        cmd.args(&[
            "-i",
            tmp_thumb.to_str().unwrap(),
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

        Ok(())
    }

    fn run_command(&self, mut cmd: Command, output_path: P) -> Result<()> {
        cmd.args(&[
            "-f",
            "mp4",
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
