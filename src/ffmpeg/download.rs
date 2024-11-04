use std::path::{Path, PathBuf};

use crate::error::Result;

#[cfg(target_os = "windows")]
const FFMPEG_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip";
#[cfg(target_os = "linux")]
const FFMPEG_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-lgpl.tar.xz";
#[cfg(target_os = "macos")]
const FFMPEG_URL: &str = "https://evermeet.cx/ffmpeg/getrelease/zip";

#[cfg(target_os = "windows")]
mod windows {
    use bytes::Bytes;
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };
    use zip::ZipArchive;

    use crate::error::{AppError, Result};

    pub(crate) fn get_default_ffmpeg_path() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.data_local_dir().join("ffmpeg"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\ffmpeg"))
    }

    pub(crate) async fn platform_specific_install(target_dir: &Path, data: Bytes) -> Result<()> {
        let cursor = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(cursor).map_err(|e| AppError::FFmpeg(e.to_string()))?;

        let target_path = target_dir.join("ffmpeg.exe");

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::FFmpeg(e.to_string()))?;
            if file.name().contains("ffmpeg.exe") {
                let mut out = File::create(&target_path)?;
                std::io::copy(&mut file, &mut out)?;
                break;
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub(crate) use windows::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use bytes::Bytes;
    use flate2::read::GzDecoder;
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };
    use tar::Archive;

    use crate::error::Result;

    pub(crate) fn get_default_ffmpeg_path() -> PathBuf {
        PathBuf::from("/usr/local/bin")
    }

    pub(crate) async fn platform_specific_install(target_dir: &Path, data: Bytes) -> Result<()> {
        let gz = GzDecoder::new(std::io::Cursor::new(data));
        let mut archive = Archive::new(gz);
        let target_path = target_dir.join("ffmpeg");

        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.to_string_lossy().contains("ffmpeg") {
                let mut out = File::create(&target_path)?;
                std::io::copy(&mut entry, &mut out)?;
                break;
            }
        }

        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&target_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&target_path, perms)?;

        Ok(())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) use unix::*;

pub async fn download_ffmpeg<P: AsRef<Path>>(path: Option<P>) -> Result<PathBuf> {
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return Err(AppError::FFmpeg("Unsupported platform".to_string()));

    let (url, target_dir) = (
        FFMPEG_URL,
        path.map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(get_default_ffmpeg_path),
    );

    let response = reqwest::get(url).await?;
    let data = response.bytes().await?;

    std::fs::create_dir_all(&target_dir)?;
    platform_specific_install(&target_dir, data).await?;

    Ok(target_dir)
}
