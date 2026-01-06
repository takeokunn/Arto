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

            // Basic operations
            if props.has_selection {
                ContextMenuItem {
                    label: "Copy",
                    shortcut: Some("⌘C"),
                    icon: Some(IconName::Copy),
                    on_click: {
                        let selected_text = props.selected_text.clone();
                        let on_close = props.on_close;
                        move |_| {
                            crate::utils::file_operations::copy_to_clipboard(&selected_text);
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

            // File operations (when viewing a file)
            if let Some(ref file) = props.current_file {
                ContextMenuSeparator {}

                ContextMenuItem {
                    label: "Open in External Editor",
                    shortcut: Some("⇧⌘E"),
                    icon: Some(IconName::ExternalLink),
                    on_click: {
                        let file = file.clone();
                        let on_close = props.on_close;
                        move |_| {
                            crate::utils::file_operations::open_in_external_editor(&file);
                            on_close.call(());
                        }
                    },
                }

                ContextMenuItem {
                    label: "Reveal in Finder",
                    shortcut: Some("⇧⌘R"),
                    icon: Some(IconName::Folder),
                    on_click: {
                        let file = file.clone();
                        let on_close = props.on_close;
                        move |_| {
                            crate::utils::file_operations::reveal_in_finder(&file);
                            on_close.call(());
                        }
                    },
                }

                ContextMenuItem {
                    label: "Copy File Path",
                    icon: Some(IconName::Copy),
                    on_click: {
                        let file = file.clone();
                        let on_close = props.on_close;
                        move |_| {
                            crate::utils::file_operations::copy_to_clipboard(
                                &file.to_string_lossy(),
                            );
                            on_close.call(());
                        }
                    },
                }
            }

            // Context-specific items
            match &props.context {
                ContentContext::Link { href } => rsx! {
                    ContextMenuSeparator {}
                    LinkContextItems {
                        href: href.clone(),
                        base_dir: props.base_dir.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::Image { src, .. } => rsx! {
                    ContextMenuSeparator {}
                    ImageContextItems {
                        src: src.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::CodeBlock { content, .. } => rsx! {
                    ContextMenuSeparator {}
                    CodeBlockContextItems {
                        content: content.clone(),
                        on_close: props.on_close,
                    }
                },
                ContentContext::Mermaid { source } => rsx! {
                    ContextMenuSeparator {}
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
                    crate::utils::file_operations::copy_to_clipboard(&href);
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
                    let src = src.clone();
                    spawn(async move {
                        // Use JSON encoding to safely escape the string for JavaScript
                        let json_src = serde_json::to_string(&src).unwrap_or_default();
                        let js = format!(
                            r#"
                            (async () => {{
                                const img = new Image();
                                img.crossOrigin = 'anonymous';
                                img.src = {};
                                await new Promise(r => img.onload = r);
                                const canvas = document.createElement('canvas');
                                canvas.width = img.naturalWidth;
                                canvas.height = img.naturalHeight;
                                const ctx = canvas.getContext('2d');
                                ctx.drawImage(img, 0, 0);
                                canvas.toBlob(async (blob) => {{
                                    await navigator.clipboard.write([
                                        new ClipboardItem({{ 'image/png': blob }})
                                    ]);
                                }}, 'image/png');
                            }})();
                            "#,
                            json_src
                        );
                        let _ = document::eval(&js).await;
                    });
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
                    let src = src.clone();
                    spawn(async move {
                        // Use JSON encoding to safely escape the string for JavaScript
                        let json_src = serde_json::to_string(&src).unwrap_or_default();
                        let js = format!(
                            r#"
                            (() => {{
                                const a = document.createElement('a');
                                a.href = {};
                                a.download = 'image.png';
                                document.body.appendChild(a);
                                a.click();
                                document.body.removeChild(a);
                            }})();
                            "#,
                            json_src
                        );
                        let _ = document::eval(&js).await;
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
                    crate::utils::file_operations::copy_to_clipboard(&src);
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
                    crate::utils::file_operations::copy_to_clipboard(&content);
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
                    crate::utils::file_operations::copy_to_clipboard(&source);
                    on_close.call(());
                }
            },
        }
    }
}
