use crate::error::{AppError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const APP_NAME: &str = "soundcloud-dl";
const ORGANIZATION: &str = "damaredayo";

#[derive(Default, Deserialize, Serialize)]
struct ConfigFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    oauth_token: Option<String>,
}

pub struct Config {
    config_path: PathBuf,
    config: ConfigFile,
}

impl Config {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from("com", ORGANIZATION, APP_NAME).ok_or_else(|| {
            AppError::Configuration("Could not determine config directory".into())
        })?;

        // Ensure config directory exists
        fs::create_dir_all(proj_dirs.config_dir())?;

        let config_path = proj_dirs.config_dir().join("config.toml");
        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            toml::from_str(&content).unwrap_or_default()
        } else {
            ConfigFile::default()
        };

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn get_oauth_token(&self) -> Result<Option<String>> {
        Ok(self.config.oauth_token.clone())
    }

    pub fn save_oauth_token(&mut self, token: &str) -> Result<()> {
        self.config.oauth_token = Some(token.to_string());

        let toml = toml::to_string_pretty(&self.config)
            .map_err(|e| AppError::Configuration(format!("Failed to serialize config: {}", e)))?;

        fs::write(&self.config_path, toml)?;

        // Set appropriate permissions on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.config_path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    pub fn clear_oauth_token(&self) -> Result<()> {
        let config = ConfigFile::default();
        let toml = toml::to_string_pretty(&config)
            .map_err(|e| AppError::Configuration(format!("Failed to serialize config: {}", e)))?;
        fs::write(&self.config_path, toml)?;
        Ok(())
    }
}
