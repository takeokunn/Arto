use dioxus::prelude::*;

/// Hook to listen for zoom updates from JavaScript.
/// Shared between ImageWindow and MermaidWindow.
pub(crate) fn use_zoom_update_handler(zoom_level: Signal<i32>) {
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
