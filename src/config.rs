use directories::ProjectDirs;
use iced::Font;
use iced_term::settings::{FontSettings, ThemeSettings};
use rootcause::{Result, option_ext::OptionExt};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;
use tokio::io::AsyncWriteExt;

use crate::{EmbeddedFont, IOSEVKA_FIXED_NORMAL_FONT};

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub terminal: TerminalConfig,
}

impl Config {
    pub async fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("com", "tukanoid", "tterm").ok_or_report()?;
        let config_dir = project_dirs.config_local_dir();

        if !config_dir.exists() {
            tracing::error!("Config folder {config_dir:?} doesn't exist, creating...");
            tokio::fs::create_dir_all(config_dir).await?;
        }

        let config_path = config_dir.join("config.toml");

        let config = match config_path.exists() {
            true => toml::from_str(&tokio::fs::read_to_string(config_path).await?)?,
            false => {
                tracing::warn!("Config at {config_path:?} was not found, creating default...");

                let config = Config::default();
                let config_str = toml::to_string_pretty(&config)?;

                let mut file = tokio::fs::File::create(config_path).await?;
                file.write_all(config_str.as_bytes()).await?;

                config
            }
        };

        Ok(config)
    }
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub font: TerminalFontConfig,
    pub theme: TerminalThemeConfig,
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalFontConfig {
    #[default = 20.0]
    pub size: f32,
    #[default = 1.0]
    pub scale_factor: f32,
    pub font_type: Option<FontTypeConfig>,
    #[default(EmbeddedFont::IosevkaFixed)]
    pub use_embedded_font: EmbeddedFont,
}

impl From<TerminalFontConfig> for FontSettings {
    fn from(
        TerminalFontConfig {
            size,
            scale_factor,
            font_type,
            use_embedded_font,
        }: TerminalFontConfig,
    ) -> Self {
        tracing::debug!("{size} {scale_factor} {font_type:?} {use_embedded_font:?}");

        FontSettings {
            size,
            scale_factor,
            font_type: font_type
                .map(Into::into)
                .unwrap_or_else(|| match use_embedded_font {
                    EmbeddedFont::IosevkaFixed => IOSEVKA_FIXED_NORMAL_FONT,
                }),
        }
    }
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontTypeConfig {
    pub family: FontFamily,
    #[serde(with = "FontWeight")]
    pub weight: iced::font::Weight,
    #[serde(with = "FontStretch")]
    pub stretch: iced::font::Stretch,
    #[serde(with = "FontStyle")]
    pub style: iced::font::Style,
}

impl From<FontTypeConfig> for Font {
    fn from(
        FontTypeConfig {
            family,
            weight,
            stretch,
            style,
        }: FontTypeConfig,
    ) -> Self {
        Font {
            family: family.into(),
            weight,
            stretch,
            style,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum FontFamily {
    // Name(&'static str),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    #[default]
    Monospace,
}

impl From<FontFamily> for iced::font::Family {
    fn from(value: FontFamily) -> Self {
        match value {
            FontFamily::Serif => iced::font::Family::Serif,
            FontFamily::SansSerif => iced::font::Family::SansSerif,
            FontFamily::Cursive => iced::font::Family::Cursive,
            FontFamily::Fantasy => iced::font::Family::Fantasy,
            FontFamily::Monospace => iced::font::Family::Monospace,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(remote = "iced::font::Weight")]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    #[default]
    Normal,
    Medium,
    Semibold,
    Bold,
    ExtraBold,
    Black,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(remote = "iced::font::Stretch")]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(remote = "iced::font::Style")]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalThemeConfig {
    #[serde(with = "TerminalColorPalette")]
    pub color_pallete: iced_term::ColorPalette,
}

impl From<TerminalThemeConfig> for ThemeSettings {
    fn from(TerminalThemeConfig { color_pallete }: TerminalThemeConfig) -> Self {
        ThemeSettings {
            color_pallete: Box::new(color_pallete),
        }
    }
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default, remote = "iced_term::ColorPalette")]
pub struct TerminalColorPalette {
    #[default = "#d8d8d8"]
    pub foreground: String,
    #[default = "#181818"]
    pub background: String,
    #[default = "#181818"]
    pub black: String,
    #[default = "#ac4242"]
    pub red: String,
    #[default = "#90a959"]
    pub green: String,
    #[default = "#f4bf75"]
    pub yellow: String,
    #[default = "#6a9fb5"]
    pub blue: String,
    #[default = "#aa759f"]
    pub magenta: String,
    #[default = "#75b5aa"]
    pub cyan: String,
    #[default = "#d8d8d8"]
    pub white: String,
    #[default = "#6b6b6b"]
    pub bright_black: String,
    #[default = "#c55555"]
    pub bright_red: String,
    #[default = "#aac474"]
    pub bright_green: String,
    #[default = "#feca88"]
    pub bright_yellow: String,
    #[default = "#82b8c8"]
    pub bright_blue: String,
    #[default = "#c28cb8"]
    pub bright_magenta: String,
    #[default = "#93d3c3"]
    pub bright_cyan: String,
    #[default = "#f8f8f8"]
    pub bright_white: String,
    pub bright_foreground: Option<String>,
    #[default = "#828482"]
    pub dim_foreground: String,
    #[default = "#0f0f0f"]
    pub dim_black: String,
    #[default = "#712b2b"]
    pub dim_red: String,
    #[default = "#5f6f3a"]
    pub dim_green: String,
    #[default = "#a17e4d"]
    pub dim_yellow: String,
    #[default = "#456877"]
    pub dim_blue: String,
    #[default = "#704d68"]
    pub dim_magenta: String,
    #[default = "#4d7770"]
    pub dim_cyan: String,
    #[default = "#8e8e8e"]
    pub dim_white: String,
}
