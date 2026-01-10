use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::components::icon::{Icon, IconName};
use crate::state::AppState;

/// Context type for right-click detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentContext {
    /// General content (no specific element)
    General,
    /// Link element
    Link { href: String },
    /// Image element
    Image { src: String, alt: Option<String> },
    /// Code block
    CodeBlock {
        content: String,
        language: Option<String>,
    },
    /// Mermaid diagram
    Mermaid { source: String },
}

/// Context menu data from JavaScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuData {
    pub context: ContentContext,
    pub x: i32,
    pub y: i32,
    /// Whether there is selected text
    pub has_selection: bool,
    /// The selected text (captured at context menu open time)
    #[serde(default)]
    pub selected_text: String,
}

#[derive(Props, Clone, PartialEq)]
pub struct ContentContextMenuProps {
    pub position: (i32, i32),
    pub context: ContentContext,
    pub has_selection: bool,
    pub selected_text: String,
    pub current_file: Option<PathBuf>,
    pub base_dir: PathBuf,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn ContentContextMenu(props: ContentContextMenuProps) -> Element {
    let has_context_specific = !matches!(props.context, ContentContext::General);

    rsx! {
        // Backdrop to close menu on outside click
        div {
            class: "context-menu-backdrop",
            // Prevent mousedown from clearing text selection
            onmousedown: move |evt| evt.prevent_default(),
            onclick: move |_| props.on_close.call(()),
        }

        // Context menu
        div {
            class: "context-menu content-context-menu",
            style: "left: {props.position.0}px; top: {props.position.1}px;",
            // Prevent mousedown from clearing text selection
            onmousedown: move |evt| evt.prevent_default(),
            onclick: move |evt| evt.stop_propagation(),

            // === Section 1: Basic text operations ===
            if props.has_selection {
                ContextMenuItem {
                    label: "Copy",
                    shortcut: Some("⌘C"),
                    icon: Some(IconName::Copy),
                    on_click: {
                        let selected_text = props.selected_text.clone();
                        let on_close = props.on_close;
                        move |_| {
                            crate::utils::clipboard::copy_text(&selected_text);
                            on_close.call(());
                        }
                    },
                }
            }

            ContextMenuItem {
                label: "Select All",
                shortcut: Some("⌘A"),
                icon: Some(IconName::SelectAll),
                on_click: {
                    let on_close = props.on_close;
                    move |_| {
                        // Inject JS that schedules itself with setTimeout
                        // This runs after menu closes without needing async in Rust
                        let _ = document::eval(r#"
                            setTimeout(() => {
                                const el = document.querySelector('.markdown-body');
                                if (el) {
                                    const range = document.createRange();
                                    range.selectNodeContents(el);
                                    const selection = window.getSelection();
                                    selection.removeAllRanges();
                                    selection.addRange(range);
                                }
                            }, 50);
                        "#);
                        on_close.call(());
                    }
                },
            }

            // === Section 2: Find and file path operations ===
            ContextMenuSeparator {}

            ContextMenuItem {
                label: "Find in Page",
                shortcut: Some("⌘F"),
                icon: Some(IconName::Search),
                on_click: {
                    let on_close = props.on_close;
                    let selected_text = props.selected_text.clone();
                    let has_selection = props.has_selection;
                    move |_| {
                        let mut state = use_context::<AppState>();
                        let text = if has_selection && !selected_text.is_empty() {
                            Some(selected_text.clone())
                        } else {
                            None
                        };
                        state.open_search_with_text(text);
                        on_close.call(());
                    }
                },
            }

            // === Section 3: Context-specific items ===
            if has_context_specific {
                ContextMenuSeparator {}
            }

            match &props.context {
                ContentContext::Link { href } => rsx! {
                    LinkContextItems {
                        href: href.clone(),
                        base_dir: props.base_dir.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::Image { src, .. } => rsx! {
                    ImageContextItems {
                        src: src.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::CodeBlock { content, .. } => rsx! {
                    CodeBlockContextItems {
                        content: content.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::Mermaid { source } => rsx! {
                    MermaidContextItems {
                        source: source.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::General => rsx! {},
            }
        }
    }
}

// ============================================================================
// Helper Components
// ============================================================================

#[derive(Props, Clone, PartialEq)]
struct ContextMenuItemProps {
    label: &'static str,
    #[props(default)]
    shortcut: Option<&'static str>,
    #[props(default)]
    icon: Option<IconName>,
    #[props(default = false)]
    disabled: bool,
    on_click: EventHandler<()>,
}

#[component]
fn ContextMenuItem(props: ContextMenuItemProps) -> Element {
    rsx! {
        div {
            class: if props.disabled { "context-menu-item disabled" } else { "context-menu-item" },
            onclick: move |_| {
                if !props.disabled {
                    props.on_click.call(());
                }
            },

            if let Some(icon) = props.icon {
                Icon {
                    name: icon,
                    size: 14,
                    class: "context-menu-icon",
                }
            }

            span { class: "context-menu-label", "{props.label}" }

            if let Some(shortcut) = props.shortcut {
                span { class: "context-menu-shortcut", "{shortcut}" }
            }
        }
    }
}

#[component]
fn ContextMenuSeparator() -> Element {
    rsx! {
        div { class: "context-menu-separator" }
    }
}

// ============================================================================
// Context-Specific Menu Items
// ============================================================================

#[component]
fn LinkContextItems(href: String, base_dir: PathBuf, on_close: EventHandler<()>) -> Element {
    let mut state = use_context::<AppState>();
    let target_path = base_dir.join(&href);

    rsx! {
        ContextMenuItem {
            label: "Open Link",
            icon: Some(IconName::ExternalLink),
            on_click: {
                let target_path = target_path.clone();
                let on_close = on_close;
                move |_| {
                    if let Ok(canonical) = target_path.canonicalize() {
                        state.navigate_to_file(canonical);
                    }
                    on_close.call(());
                }
            },
        }

        ContextMenuItem {
            label: "Open Link in New Tab",
            icon: Some(IconName::Add),
            on_click: {
                let target_path = target_path.clone();
                let on_close = on_close;
                move |_| {
                    if let Ok(canonical) = target_path.canonicalize() {
                        state.add_file_tab(canonical, true);
                    }
                    on_close.call(());
                }
            },
        }

        ContextMenuItem {
            label: "Copy Link Path",
            icon: Some(IconName::Copy),
            on_click: {
                let href = href.clone();
                let on_close = on_close;
                move |_| {
                    crate::utils::clipboard::copy_text(&href);
                    on_close.call(());
                }
            },
        }
    }
}

#[component]
fn ImageContextItems(src: String, on_close: EventHandler<()>) -> Element {
    rsx! {
        ContextMenuItem {
            label: "Copy Image",
            icon: Some(IconName::Photo),
            on_click: {
                let src = src.clone();
                let on_close = on_close;
                move |_| {
                    crate::utils::clipboard::copy_image_from_data_url(&src);
                    on_close.call(());
                }
            },
        }

        ContextMenuItem {
            label: "Save Image As...",
            icon: Some(IconName::Download),
            on_click: {
                let src = src.clone();
                let on_close = on_close;
                move |_| {
                    // Run in background thread to prevent UI blocking during HTTP download
                    let src = src.clone();
                    std::thread::spawn(move || {
                        crate::utils::image::save_image(&src);
                    });
                    on_close.call(());
                }
            },
        }

        ContextMenuItem {
            label: "Copy Image Path",
            icon: Some(IconName::Copy),
            on_click: {
                let src = src.clone();
                let on_close = on_close;
                move |_| {
                    // For data URLs, just copy the src (or original path if available)
                    crate::utils::clipboard::copy_text(&src);
                    on_close.call(());
                }
            },
        }
    }
}

#[component]
fn CodeBlockContextItems(content: String, on_close: EventHandler<()>) -> Element {
    rsx! {
        ContextMenuItem {
            label: "Copy Code",
            icon: Some(IconName::Copy),
            on_click: {
                let content = content.clone();
                let on_close = on_close;
                move |_| {
                    crate::utils::clipboard::copy_text(&content);
                    on_close.call(());
                }
            },
        }
    }
}

#[component]
fn MermaidContextItems(source: String, on_close: EventHandler<()>) -> Element {
    rsx! {
        ContextMenuItem {
            label: "Copy Code",
            icon: Some(IconName::Copy),
            on_click: {
                let source = source.clone();
                let on_close = on_close;
                move |_| {
                    crate::utils::clipboard::copy_text(&source);
                    on_close.call(());
                }
            },
        }
    }
}
