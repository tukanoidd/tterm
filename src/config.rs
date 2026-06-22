pub mod common;
pub mod general;
pub mod keybinds;
pub mod presets;
pub mod terminal;

use directories::ProjectDirs;
use rootcause::{Result, option_ext::OptionExt};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use tokio::io::AsyncWriteExt;

use crate::config::{
    general::GeneralConfig, keybinds::KeyBindsConfig, presets::PresetsConfig,
    terminal::TerminalConfig,
};

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub presets: PresetsConfig,
    pub terminal: TerminalConfig,
    pub keybinds: KeyBindsConfig,
}

impl Config {
    pub async fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "tukanoid", "tterm").ok_or_report()?;
        let config_dir = project_dirs.config_local_dir();

        if !config_dir.exists() {
            tracing::error!("Config folder {config_dir:?} doesn't exist, creating...");
            tokio::fs::create_dir_all(config_dir).await?;
        }

        let config_path = config_dir.join("config.ron");

        let options = ron::options::Options::default().with_default_extension(
            ron::extensions::Extensions::UNWRAP_NEWTYPES
                | ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES
                | ron::extensions::Extensions::IMPLICIT_SOME,
        );

        let config = match config_path.exists() {
            true => options.from_str(&tokio::fs::read_to_string(config_path).await?)?,
            false => {
                tracing::warn!("Config at {config_path:?} was not found, creating default...");

                let config = Config::default();
                let config_str = options
                    .to_string_pretty(&config, ron::ser::PrettyConfig::new().depth_limit(4))?;

                let mut file = tokio::fs::File::create(config_path).await?;
                file.write_all(config_str.as_bytes()).await?;

                config
            }
        };

        Ok(config)
    }
}
