use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    #[default(true)]
    pub reactive_panels: bool,
    pub theme: BuiltinTheme,
}

macro_rules! builtin_theme {
    (
        $($(#[$default:ident])? $name:ident),+
        $(,)?
    ) => {
        #[derive(
            SmartDefault,
            Debug, derive_more::Display,
            Clone, Copy,
            serde::Serialize, serde::Deserialize
        )]
        pub enum BuiltinTheme {
            $($(#[$default])? $name),+
        }

        impl From<BuiltinTheme> for $crate::app::AppTheme {
            fn from(value: BuiltinTheme) -> Self {
                match value {
                    $(BuiltinTheme::$name => Self::$name),+
                }
            }
        }
    };
}

builtin_theme![
    Light,
    Dark,
    Dracula,
    Nord,
    SolarizedLight,
    SolarizedDark,
    GruvboxLight,
    GruvboxDark,
    CatppuccinLatte,
    CatppuccinFrappe,
    CatppuccinMacchiato,
    CatppuccinMocha,
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
    KanagawaWave,
    #[default]
    KanagawaDragon,
    KanagawaLotus,
    Moonfly,
    Nightfly,
    Oxocarbon,
    Ferra,
];
