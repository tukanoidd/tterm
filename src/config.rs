pub mod common;
pub mod general;
pub mod keybinds;
pub mod presets;
pub mod terminal;
pub mod webview;

use derive_more::AsRef;
use directories::ProjectDirs;
use rootcause::{Result, option_ext::OptionExt};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use tokio::io::AsyncWriteExt;

use crate::{
    app::mode::{terminal::TerminalModeConfig, webview::WebViewModeConfig},
    config::{general::GeneralConfig, presets::PresetsConfig},
};

#[derive(SmartDefault, Debug, Clone, AsRef, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub presets: PresetsConfig,

    #[as_ref]
    pub terminal_mode: TerminalModeConfig,
    #[as_ref]
    pub webview_mode: WebViewModeConfig,
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

        let options = Self::ron_options();

        let config = match config_path.exists() {
            true => options.from_str(&tokio::fs::read_to_string(config_path).await?)?,
            false => {
                tracing::warn!("Config at {config_path:?} was not found, creating default...");

                let config = Config::default();
                let config_str = options.to_string_pretty(&config, Self::ron_pretty_config())?;

                let mut file = tokio::fs::File::create(config_path).await?;
                file.write_all(config_str.as_bytes()).await?;

                config
            }
        };

        Ok(config)
    }

    pub fn ron_options() -> ron::Options {
        ron::options::Options::default().with_default_extension(
            ron::extensions::Extensions::UNWRAP_NEWTYPES
                | ron::extensions::Extensions::UNWRAP_VARIANT_NEWTYPES
                | ron::extensions::Extensions::IMPLICIT_SOME,
        )
    }

    pub fn ron_pretty_config() -> ron::ser::PrettyConfig {
        ron::ser::PrettyConfig::new().depth_limit(4)
    }
}
