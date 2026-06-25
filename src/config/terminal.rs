use iced::Font;
use iced_term::settings::{FontSettings, ThemeSettings};
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::fonts::EmbeddedFont;

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub font: TerminalFontConfig,
    pub theme: TerminalThemeConfig,
    pub shell: Option<String>,
    #[default = 3.0]
    pub scroll_acceleration: f32,
}

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalFontConfig {
    #[default = 20.0]
    pub size: f32,
    #[default = 1.0]
    pub scale_factor: f32,
    pub font_type: Option<FontTypeConfig>,
    #[default(EmbeddedFont::JetBrainsMonoNerdFontMonoBold)]
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
        FontSettings {
            size,
            scale_factor,
            font_type: font_type
                .map(Into::into)
                .unwrap_or_else(|| use_embedded_font.to_font()),
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

macro_rules! color_palette {
    (
        $($(@$opt:ident)? $name:ident: $ty:ty $(= $default:expr)?),+
        $(,)?
    ) => {
        #[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
        #[serde(default, remote = "iced_term::ColorPalette")]
        pub struct TerminalColorPalette {
            $($(#[default($default.into())])? pub $name: $ty),+
        }
    };
}

color_palette! {
    background: String = "#1a1b26",
    foreground: String = "#c0caf5",

    black: String = "#15161e",
    red: String = "#f7768e",
    green: String = "#9ece6a",
    yellow: String = "#e0af68",
    blue: String = "#7aa2f7",
    magenta: String = "#bb9af7",
    cyan: String = "#7dcfff",
    white: String = "#a9b1d6",

    dim_foreground: String = "#c0caf5",
    dim_black: String = "#15161e",
    dim_red: String = "#f7768e",
    dim_green: String = "#9ece6a",
    dim_yellow: String = "#e0af68",
    dim_blue: String = "#7aa2f7",
    dim_magenta: String = "#bb9af7",
    dim_cyan: String = "#7dcfff",
    dim_white: String = "#a9b1d6",

    bright_black: String = "#414868",
    bright_red: String = "#f7768e",
    bright_green: String = "#9ece6a",
    bright_yellow: String = "#e0af68",
    bright_blue: String = "#7aa2f7",
    bright_magenta: String = "#bb9af7",
    bright_cyan: String = "#7dcfff",
    bright_white: String = "#c0caf5",
    bright_foreground: Option<String> = Some("#c0caf5".into()),
}
