use dioxus::desktop::{use_muda_event_handler, window};
use dioxus::prelude::*;
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::assets::MAIN_SCRIPT;
use crate::components::viewer_hooks::use_zoom_update_handler;

/// Props for ImageWindow component
#[derive(Props, Clone, PartialEq)]
pub struct ImageWindowProps {
    /// Window title (filename or fallback)
    pub title: String,
}

/// Compute deduplication key for image windows.
/// Uses original file path when available (fast, small string),
/// URL for HTTP images, or SHA256 of first 1024 bytes as fallback.
pub fn compute_image_dedup_key(src: &str, original_src: Option<&str>) -> String {
    if let Some(path) = original_src {
        if !path.is_empty() {
            // Local images: use canonical path
            return format!("image:{path}");
        }
    }
    if src.starts_with("http://") || src.starts_with("https://") {
        // HTTP images: use URL directly
        format!("image:{}", src)
    } else {
        // Fallback: SHA256 prefix of first 1024 bytes
        let hash_input = &src.as_bytes()[..src.len().min(1024)];
        let mut hasher = Sha256::new();
        hasher.update(hash_input);
        let result = hasher.finalize();
        let hex = format!("{:x}", result);
        format!("image:{}", &hex[..16])
    }
}

/// Extract a display title from original_src or src
pub fn extract_image_title(src: &str, original_src: Option<&str>) -> String {
    // Try original path first
    if let Some(path) = original_src {
        if let Some(filename) = Path::new(path).file_name().and_then(|f| f.to_str()) {
            return format!("{} - Image Viewer", filename);
        }
    }

    // Try HTTP URL filename
    if src.starts_with("http://") || src.starts_with("https://") {
        if let Some(filename) = src.rsplit('/').next() {
            let filename = filename.split('?').next().unwrap_or(filename);
            if !filename.is_empty() {
                return format!("{} - Image Viewer", filename);
            }
        }
    }

    // Fallback
    "Image Viewer - Arto".to_string()
}

/// Image Window Component
#[component]
pub fn ImageWindow(props: ImageWindowProps) -> Element {
    let zoom_level = use_signal(|| 100);

    // Load viewer script on mount (image data is embedded in HTML index)
    use_viewer_script_loader();

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
            class: "image-window-container",

            // Header with title
            div {
                class: "image-window-header",

                div {
                    class: "image-window-title",
                    "{props.title}"
                }
            }

            // Canvas container for image
            div {
                id: "image-window-canvas",
                class: "image-window-canvas",

                // Wrapper for positioning (translate)
                div {
                    id: "image-wrapper",
                    class: "image-wrapper",

                    // Inner container for zoom
                    div {
                        id: "image-container",
                        class: "image-container",
                        // JS controller creates <img> here programmatically
                    }
                }
            }

            // Status bar
            div {
                class: "image-window-status",
                "Zoom: {zoom_level}% | Scroll to zoom, drag to pan, double-click to fit"
            }
        }
    }
}

/// Hook to load viewer script and initialize.
/// The image data URL is embedded in the HTML index as `window._imageDataUrl`
/// (set by `build_image_window_index`), so we only need to import the module
/// and call the init function — no data transfer through eval IPC.
fn use_viewer_script_loader() {
    use_effect(|| {
        spawn(async move {
            let eval_result = document::eval(&indoc::formatdoc! {r#"
                (async () => {{
                    try {{
                        const {{ initImageWindow }} = await import("{MAIN_SCRIPT}");
                        await initImageWindow(window._imageDataUrl);
                    }} catch (error) {{
                        console.error("Failed to load image window module:", error);
                    }}
                }})();
            "#});

            if let Err(e) = eval_result.await {
                tracing::error!("Failed to initialize image window: {}", e);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_image_dedup_key_with_path() {
        let key = compute_image_dedup_key("data:image/png;base64,abc", Some("/path/to/image.png"));
        assert_eq!(key, "image:/path/to/image.png");
    }

    #[test]
    fn test_compute_image_dedup_key_with_http() {
        let key = compute_image_dedup_key("https://example.com/photo.jpg", None);
        assert_eq!(key, "image:https://example.com/photo.jpg");
    }

    #[test]
    fn test_compute_image_dedup_key_fallback() {
        let key = compute_image_dedup_key("data:image/png;base64,abc123", None);
        assert!(key.starts_with("image:"));
        // "image:" prefix (6 chars) + 16 hex chars = 22
        assert_eq!(key.len(), 6 + 16);
    }

    #[test]
    fn test_extract_image_title_from_path() {
        let title = extract_image_title("data:image/png;base64,abc", Some("/path/to/photo.png"));
        assert_eq!(title, "photo.png - Image Viewer");
    }

    #[test]
    fn test_extract_image_title_from_url() {
        let title = extract_image_title("https://example.com/images/photo.jpg", None);
        assert_eq!(title, "photo.jpg - Image Viewer");
    }

    #[test]
    fn test_extract_image_title_from_url_with_query() {
        let title = extract_image_title("https://example.com/photo.jpg?w=100", None);
        assert_eq!(title, "photo.jpg - Image Viewer");
    }

    #[test]
    fn test_extract_image_title_fallback() {
        let title = extract_image_title("data:image/png;base64,abc", None);
        assert_eq!(title, "Image Viewer - Arto");
    }

    #[test]
    fn test_compute_image_dedup_key_empty_original_src() {
        // When original_src is Some(""), it should fall through to the SHA256 fallback
        // instead of producing the ambiguous key "image:".
        let key = compute_image_dedup_key("data:image/png;base64,abc123", Some(""));
        assert!(key.starts_with("image:"));
        // Should use SHA256 fallback: "image:" prefix (6 chars) + 16 hex chars = 22
        assert_eq!(key.len(), 6 + 16);
    }

    #[test]
    fn test_compute_image_dedup_key_long_data_url() {
        // Build a data URL longer than 1024 bytes using ASCII base64 characters
        let long_base64 = "A".repeat(2000);
        let src = format!("data:image/png;base64,{}", long_base64);
        assert!(src.len() > 1024);

        let key = compute_image_dedup_key(&src, None);
        // Must start with "image:" prefix
        assert!(key.starts_with("image:"));
        // SHA256 prefix must be exactly 16 hex characters
        let hex_part = &key["image:".len()..];
        assert_eq!(hex_part.len(), 16);
        assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_image_dedup_key_unicode_in_path() {
        let key = compute_image_dedup_key("data:image/png;base64,abc", Some("/path/to/画像.png"));
        assert_eq!(key, "image:/path/to/画像.png");
    }

    #[test]
    fn test_extract_image_title_empty_original_src() {
        // Path::new("").file_name() returns None, so it falls through to the
        // data URL fallback path rather than using the empty string.
        let title = extract_image_title("data:image/png;base64,abc", Some(""));
        assert_eq!(title, "Image Viewer - Arto");
    }

    #[test]
    fn test_extract_image_title_url_with_encoded_chars() {
        // Percent-encoded characters in URL are not decoded; the raw segment
        // "my%20photo.jpg" is used as the title filename.
        let title = extract_image_title("https://example.com/my%20photo.jpg", None);
        assert_eq!(title, "my%20photo.jpg - Image Viewer");
    }

    #[test]
    fn test_compute_image_dedup_key_deterministic() {
        let src = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg";
        let key1 = compute_image_dedup_key(src, None);
        let key2 = compute_image_dedup_key(src, None);
        assert_eq!(key1, key2, "Same input should produce identical dedup keys");
    }

    #[test]
    fn test_compute_image_dedup_key_different_inputs() {
        let key1 = compute_image_dedup_key("data:image/png;base64,abc", None);
        let key2 = compute_image_dedup_key("data:image/png;base64,xyz", None);
        assert_ne!(
            key1, key2,
            "Different inputs should produce different dedup keys"
        );
    }

    #[test]
    fn test_extract_image_title_http_url() {
        let title = extract_image_title("http://example.com/photo.png", None);
        assert_eq!(title, "photo.png - Image Viewer");
    }
}
