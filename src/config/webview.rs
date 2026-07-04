use serde::{Deserialize, Serialize};
use smart_default::SmartDefault;

#[derive(SmartDefault, Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebViewConfig {
    #[default = "https://duckduckgo.com"]
    pub default_url: String,
    #[default = 10.0]
    pub scroll_acceleration: f32,
}
