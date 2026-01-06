use dioxus::document;
use dioxus::prelude::*;

use crate::markdown::HeadingInfo;
use crate::state::{AppState, LAST_FOCUSED_STATE};

#[derive(Props, Clone, PartialEq)]
pub struct TocPanelProps {
    pub headings: Vec<HeadingInfo>,
}

#[component]
pub fn TocPanel(props: TocPanelProps) -> Element {
    let mut state = use_context::<AppState>();
    let is_visible = *state.toc_open.read();
    let width = *state.toc_width.read();

    let mut is_resizing = use_signal(|| false);

    // Clamp initial width to window size on mount
    use_effect(move || {
        spawn(async move {
            let result = document::eval(r#"window.innerWidth * 0.5"#)
                .await
                .ok()
                .and_then(|v| v.as_f64());

            if let Some(max_width) = result {
                let current_width = *state.toc_width.read();
                if current_width > max_width {
                    let clamped = current_width.clamp(150.0, max_width);
                    state.toc_width.set(clamped);
                    LAST_FOCUSED_STATE.write().toc_width = clamped;
                }
            }
        });
    });

    let style = if is_visible {
        format!("width: {}px;", width)
    } else {
        "width: 0;".to_string()
    };

    rsx! {
        div {
            class: "toc-panel",
            class: if is_visible { "visible" },
            class: if is_resizing() { "resizing" },
            style: "{style}",

            // Resize handle (left side, only when visible)
            if is_visible {
                div {
                    class: "toc-resize-handle",
                    class: if is_resizing() { "resizing" },
                    onmousedown: move |evt| {
                        evt.prevent_default();
                        is_resizing.set(true);
                        let start_x = evt.page_coordinates().x;
                        let start_width = *state.toc_width.read();

                        spawn(async move {
                            #[derive(serde::Deserialize)]
                            struct DragMessage {
                                r#type: String,
                                x: Option<f64>,
                                #[serde(rename = "maxWidth")]
                                max_width: Option<f64>,
                            }

                            let mut eval = document::eval(r#"
                                new Promise((resolve) => {
                                    const handleMouseMove = (e) => {
                                        const maxWidth = window.innerWidth * 0.5;
                                        dioxus.send({ type: 'move', x: e.pageX, maxWidth });
                                    };
                                    const handleMouseUp = () => {
                                        document.removeEventListener('mousemove', handleMouseMove);
                                        document.removeEventListener('mouseup', handleMouseUp);
                                        dioxus.send({ type: 'end' });
                                        resolve();
                                    };
                                    document.addEventListener('mousemove', handleMouseMove);
                                    document.addEventListener('mouseup', handleMouseUp);
                                })
                            "#);

                            while let Ok(msg) = eval.recv::<DragMessage>().await {
                                match msg.r#type.as_str() {
                                    "move" => {
                                        if let Some(x) = msg.x {
                                            // TOC resizes from left edge, so delta is inverted
                                            let delta = start_x - x;
                                            let max_width = msg.max_width.unwrap_or(400.0);
                                            let new_width = (start_width + delta).clamp(150.0, max_width);
                                            state.toc_width.set(new_width);
                                        }
                                    }
                                    "end" => {
                                        let final_width = *state.toc_width.read();
                                        LAST_FOCUSED_STATE.write().toc_width = final_width;
                                        is_resizing.set(false);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                }
            }

            // TOC content
            div {
                class: "toc-content",

                h3 { class: "toc-title", "Table of Contents" }

                if props.headings.is_empty() {
                    p { class: "toc-empty", "No headings found" }
                } else {
                    ul {
                        class: "toc-list",
                        for heading in props.headings.iter() {
                            TocItem { heading: heading.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TocItem(heading: HeadingInfo) -> Element {
    let id = heading.id.clone();

    rsx! {
        li {
            class: "toc-item",
            "data-level": "{heading.level}",
            onclick: move |_| {
                let id = id.clone();
                spawn(async move {
                    let js = format!(
                        r#"
                        (() => {{
                            const el = document.getElementById('{}');
                            if (el) {{
                                el.scrollIntoView({{ behavior: 'smooth', block: 'start' }});
                            }}
                        }})();
                        "#,
                        id
                    );
                    let _ = document::eval(&js).await;
                });
            },
            "{heading.text}"
        }
    }
}
