use std::path::PathBuf;

use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

use crate::config::common::SplitDirection;

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetsConfig {
    pub default: Option<String>,
    pub list: Vec<PresetConfig>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PresetConfig {
    pub name: String,
    pub tabs: Vec<TabConfig>,
}

#[derive(SmartDefault, Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct TabConfig {
    pub name: Option<String>,
    pub pane: PaneConfig,
    #[serde(alias = "floating")]
    pub floating_pane: Option<PaneConfig>,
}

#[derive(SmartDefault, Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneConfig {
    #[serde(alias = "pwd")]
    pub working_directory: Option<PathBuf>,
    pub program: Option<ProgramConfig>,

    pub split: Option<PaneSplitConfig>,
}

#[derive(SmartDefault, Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct ProgramConfig {
    #[serde(alias = "cmd")]
    pub command: String,
    pub args: Vec<String>,
}

#[derive(SmartDefault, Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(default)]
pub struct PaneSplitConfig {
    #[serde(alias = "dir")]
    pub direction: SplitDirection,
    #[default(0.5.into())]
    pub ratio: OrderedFloat<f32>,

    pub child: Box<PaneConfig>,
}
