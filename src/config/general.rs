use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    #[default(true)]
    pub reactive_panels: bool,
}
