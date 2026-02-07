use serde::{Deserialize, Serialize};

use super::behavior::{NewWindowBehavior, StartupBehavior};

fn default_zoom_level() -> f64 {
    1.0
}

/// Configuration for zoom-related settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoomConfig {
    /// Default zoom level (1.0 = 100%)
    #[serde(default = "default_zoom_level")]
    pub default_zoom_level: f64,
    /// Behavior on app startup: "default" or "last_closed"
    pub on_startup: StartupBehavior,
    /// Behavior when opening a new window: "default" or "last_focused"
    pub on_new_window: NewWindowBehavior,
}

// Manual Default because f64's default is 0.0, but zoom default should be 1.0
impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            default_zoom_level: 1.0,
            on_startup: StartupBehavior::default(),
            on_new_window: NewWindowBehavior::default(),
        }
    }
}
