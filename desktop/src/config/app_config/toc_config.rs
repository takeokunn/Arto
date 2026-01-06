use super::behavior::{NewWindowBehavior, StartupBehavior};
use serde::{Deserialize, Serialize};

/// Default TOC panel width in pixels
pub const DEFAULT_TOC_WIDTH: f64 = 220.0;

fn default_toc_width() -> f64 {
    DEFAULT_TOC_WIDTH
}

/// Configuration for table of contents panel settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocConfig {
    /// Whether TOC panel is open by default
    pub default_open: bool,
    /// Default TOC panel width in pixels
    #[serde(default = "default_toc_width")]
    pub default_width: f64,
    /// Behavior on app startup: "default" or "last_closed"
    pub on_startup: StartupBehavior,
    /// Behavior when opening a new window: "default" or "last_focused"
    pub on_new_window: NewWindowBehavior,
}

impl Default for TocConfig {
    fn default() -> Self {
        Self {
            default_open: false,
            default_width: default_toc_width(),
            on_startup: StartupBehavior::Default,
            on_new_window: NewWindowBehavior::Default,
        }
    }
}
