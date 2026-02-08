use dioxus::desktop::{use_muda_event_handler, window};
use dioxus::prelude::*;
use sha2::{Digest, Sha256};

use crate::assets::MAIN_SCRIPT;
use crate::components::icon::{Icon, IconName};
use crate::components::theme_selector::ThemeSelector;
use crate::theme::Theme;

/// Props for MathWindow component
#[derive(Props, Clone, Debug, PartialEq)]
pub struct MathWindowProps {
    /// LaTeX source
    pub source: String,
    /// Unique math identifier (prefixed hash)
    pub math_id: String,
    /// Initial theme
    pub theme: Theme,
}

/// Generate unique ID from LaTeX source with "math:" prefix
pub fn generate_math_id(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let result = hasher.finalize();
    let hex_hash = format!("{:x}", result);
    format!("math:{}", &hex_hash[..16])
}

/// Math Window Component
#[component]
pub fn MathWindow(props: MathWindowProps) -> Element {
    let current_theme = use_signal(|| props.theme);
    let zoom_level = use_signal(|| 100);

    // Load viewer script on mount
    use_viewer_script_loader(props.source.clone(), props.math_id.clone());

    // Setup zoom update handler
    use_zoom_update_handler(zoom_level);

    // Handle Cmd+W and Cmd+Shift+W to close this child window
    use_muda_event_handler(move |event| {
        if !window().is_focused() {
            return;
        }
        if crate::menu::is_close_action(event) {
            window().close();
        }
    });

    rsx! {
        div {
            class: "math-window-container",

            // Header with controls
            div {
                class: "math-window-header",

                // Empty spacer on left
                div {
                    class: "math-window-title",
                }

                div {
                    class: "math-window-controls",
                    CopyLaTeXButton { source: props.source.clone() }
                    ThemeSelector { current_theme }
                }
            }

            // Canvas container for math expression
            div {
                id: "math-window-canvas",
                class: "math-window-canvas",

                // Wrapper for positioning (translate)
                div {
                    id: "math-content-wrapper",
                    class: "math-content-wrapper",

                    // Inner container for zoom
                    div {
                        id: "math-content-container",
                        class: "math-content-container",
                        // Placeholder for KaTeX rendered HTML
                    }
                }
            }

            // Status bar
            div {
                class: "math-window-status",
                "Zoom: {zoom_level}% | Scroll to zoom, drag to pan, double-click to fit"
            }
        }
    }
}

/// Hook to load viewer script and initialize
fn use_viewer_script_loader(source: String, math_id: String) {
    use_effect(move || {
        let source = source.clone();
        let math_id = math_id.clone();

        spawn(async move {
            // Escape source for JavaScript (handle backticks, backslashes, dollar signs)
            let escaped_source = source
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace('$', "\\$");

            let eval_result = document::eval(&indoc::formatdoc! {r#"
                (async () => {{
                    try {{
                        const {{ initMathWindow }} = await import("{MAIN_SCRIPT}");
                        await initMathWindow(`{escaped_source}`, '{math_id}');
                    }} catch (error) {{
                        console.error("Failed to load math window module:", error);
                    }}
                }})();
            "#});

            if let Err(e) = eval_result.await {
                tracing::error!("Failed to initialize math window: {}", e);
            }
        });
    });
}

/// Hook to listen for zoom updates from JavaScript
fn use_zoom_update_handler(zoom_level: Signal<i32>) {
    use_effect(move || {
        let mut zoom_level = zoom_level;

        spawn(async move {
            let mut eval_provider = document::eval(indoc::indoc! {r#"
                window.updateZoomLevel = (zoom) => {
                    dioxus.send({ zoom: Math.round(zoom) });
                };
            "#});

            while let Ok(data) = eval_provider.recv::<serde_json::Value>().await {
                if let Some(zoom) = data.get("zoom").and_then(|v| v.as_i64()) {
                    zoom_level.set(zoom as i32);
                }
            }
        });
    });
}

/// Copy status for visual feedback
#[derive(Clone, Copy, PartialEq, Default)]
enum CopyStatus {
    #[default]
    Idle,
    Success,
    Error,
}

/// Copy LaTeX source button component
#[component]
fn CopyLaTeXButton(source: String) -> Element {
    let mut copy_status = use_signal(|| CopyStatus::Idle);

    let handle_click = move |_| {
        let source = source.clone();
        spawn(async move {
            let mut eval = document::eval(indoc::indoc! {r#"
                const data = await dioxus.recv();
                try {
                    await navigator.clipboard.writeText(data);
                    dioxus.send(true);
                } catch {
                    dioxus.send(false);
                }
            "#});

            // Send the source text to JavaScript (no escaping needed)
            eval.send(source).ok();

            // Receive the result from JavaScript
            match eval.recv::<bool>().await {
                Ok(true) => {
                    copy_status.set(CopyStatus::Success);
                }
                _ => {
                    copy_status.set(CopyStatus::Error);
                }
            }

            // Reset after 2 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            copy_status.set(CopyStatus::Idle);
        });
    };

    let (icon, extra_class) = match *copy_status.read() {
        CopyStatus::Idle => (IconName::Copy, ""),
        CopyStatus::Success => (IconName::Check, "copied"),
        CopyStatus::Error => (IconName::Close, "error"),
    };

    rsx! {
        button {
            class: "viewer-control-btn {extra_class}",
            "aria-label": "Copy LaTeX source",
            title: "Copy LaTeX source",
            onclick: handle_click,
            Icon { name: icon, size: 18 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_math_id_consistent() {
        let source = r"E = mc^2";
        let id1 = generate_math_id(source);
        let id2 = generate_math_id(source);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_generate_math_id_different_sources() {
        let id1 = generate_math_id(r"E = mc^2");
        let id2 = generate_math_id(r"\int_0^\infty e^{-x} dx");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_generate_math_id_has_prefix() {
        let id = generate_math_id(r"E = mc^2");
        assert!(id.starts_with("math:"));
    }

    #[test]
    fn test_generate_math_id_idempotent() {
        let source = r"\frac{a}{b} + \sqrt{c}";
        let results: Vec<String> = (0..10).map(|_| generate_math_id(source)).collect();
        assert!(results.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn test_math_window_props_clone_partial_eq() {
        let props = MathWindowProps {
            source: "x^2".to_string(),
            math_id: generate_math_id("x^2"),
            theme: Theme::Dark,
        };
        let cloned = props.clone();
        assert_eq!(props, cloned);
    }
}
