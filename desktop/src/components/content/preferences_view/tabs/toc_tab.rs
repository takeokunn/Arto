use super::super::form_controls::{OptionCardItem, OptionCards, SliderInput};
use crate::config::{Config, NewWindowBehavior, StartupBehavior};
use dioxus::prelude::*;

#[component]
pub fn TocTab(
    config: Signal<Config>,
    has_changes: Signal<bool>,
    current_toc_width: f64,
) -> Element {
    // Extract values upfront to avoid holding read guard across closures
    let toc = config.read().toc.clone();

    rsx! {
        div {
            class: "preferences-pane",

            h3 { class: "preference-section-title", "Default Settings" }

            div {
                class: "preference-item",
                div {
                    class: "preference-item-header",
                    label { "Open by Default" }
                    p { class: "preference-description", "Whether the table of contents panel is open when starting." }
                }
                OptionCards {
                    name: "toc-default-open".to_string(),
                    options: vec![
                        OptionCardItem {
                            icon: None,
                            value: false,
                            title: "Closed".to_string(),
                            description: Some("TOC panel closed by default".to_string()),
                        },
                        OptionCardItem {
                            icon: None,
                            value: true,
                            title: "Open".to_string(),
                            description: Some("TOC panel open by default".to_string()),
                        },
                    ],
                    selected: toc.default_open,
                    on_change: move |new_state| {
                        config.write().toc.default_open = new_state;
                        has_changes.set(true);
                    },
                }
            }

            div {
                class: "preference-item",
                div {
                    class: "preference-item-header",
                    label { "Default Width" }
                    p { class: "preference-description", "The default TOC panel width in pixels." }
                }
                SliderInput {
                    value: toc.default_width,
                    min: 150.0,
                    max: 400.0,
                    step: 10.0,
                    unit: "px".to_string(),
                    on_change: move |new_width| {
                        config.write().toc.default_width = new_width;
                        has_changes.set(true);
                    },
                    current_value: Some(current_toc_width),
                }
            }

            h3 { class: "preference-section-title", "Behavior" }

            div {
                class: "preference-item",
                div {
                    class: "preference-item-header",
                    label { "On Startup" }
                    p { class: "preference-description", "TOC panel state when the application starts." }
                }
                OptionCards {
                    name: "toc-startup".to_string(),
                    options: vec![
                        OptionCardItem {
                            icon: None,
                            value: StartupBehavior::Default,
                            title: "Default".to_string(),
                            description: Some("Use default settings".to_string()),
                        },
                        OptionCardItem {
                            icon: None,
                            value: StartupBehavior::LastClosed,
                            title: "Last Closed".to_string(),
                            description: Some("Resume from last closed window".to_string()),
                        },
                    ],
                    selected: toc.on_startup,
                    on_change: move |new_behavior| {
                        config.write().toc.on_startup = new_behavior;
                        has_changes.set(true);
                    },
                }
            }

            div {
                class: "preference-item",
                div {
                    class: "preference-item-header",
                    label { "On New Window" }
                    p { class: "preference-description", "TOC panel state in new windows." }
                }
                OptionCards {
                    name: "toc-new-window".to_string(),
                    options: vec![
                        OptionCardItem {
                            icon: None,
                            value: NewWindowBehavior::Default,
                            title: "Default".to_string(),
                            description: Some("Use default settings".to_string()),
                        },
                        OptionCardItem {
                            icon: None,
                            value: NewWindowBehavior::LastFocused,
                            title: "Last Focused".to_string(),
                            description: Some("Same as current window".to_string()),
                        },
                    ],
                    selected: toc.on_new_window,
                    on_change: move |new_behavior| {
                        config.write().toc.on_new_window = new_behavior;
                        has_changes.set(true);
                    },
                }
            }
        }
    }
}
