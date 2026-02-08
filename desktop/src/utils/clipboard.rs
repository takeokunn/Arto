//! Unified clipboard operations using arboard
//!
//! This module provides cross-platform clipboard functionality for both text and images.
//! The clipboard instance is held for the application's lifetime to ensure proper
//! clipboard ownership on Linux.

use arboard::{Clipboard, ImageData};
use base64::Engine;
use std::sync::LazyLock;
use std::sync::Mutex;

use super::image::{download_image, extract_base64_from_data_url, extract_mime_type_from_data_url};

/// Global clipboard instance held for the application lifetime.
///
/// On Linux, clipboard contents are owned by the application that placed them,
/// so we keep the clipboard alive to prevent data loss.
static CLIPBOARD: LazyLock<Mutex<Clipboard>> =
    LazyLock::new(|| Mutex::new(Clipboard::new().expect("Failed to initialize clipboard")));

/// Copy text to the system clipboard.
///
/// # Examples
///
/// ```rust,ignore
/// copy_text("Hello, world!");
/// ```
pub fn copy_text(text: impl AsRef<str>) {
    let mut clipboard = CLIPBOARD.lock().unwrap();
    if let Err(e) = clipboard.set_text(text.as_ref()) {
        tracing::error!(%e, "Failed to copy text to clipboard");
    }
}

/// Copy an image to the system clipboard from any supported source.
///
/// Supports:
/// - Data URLs: `data:image/png;base64,<base64-data>`
/// - HTTP/HTTPS URLs: `https://example.com/image.png`
pub fn copy_image(src: impl AsRef<str>) {
    let src = src.as_ref();

    let image_bytes = if src.starts_with("data:") {
        // Reject SVG data URLs (vector format, cannot be rasterized without resvg)
        if extract_mime_type_from_data_url(src) == Some("image/svg+xml") {
            tracing::warn!("Cannot copy SVG image to clipboard (vector format not supported)");
            return;
        }

        // Data URL: extract and decode base64
        let base64_data = match extract_base64_from_data_url(src) {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(%e, "Failed to extract base64 data from data URL");
                return;
            }
        };
        match base64::prelude::BASE64_STANDARD.decode(base64_data) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(%e, "Failed to decode base64 image data");
                return;
            }
        }
    } else if src.starts_with("http://") || src.starts_with("https://") {
        // Reject SVG URLs by extension (best-effort check before download)
        if src.split('?').next().is_some_and(|p| p.ends_with(".svg")) {
            tracing::warn!("Cannot copy SVG image to clipboard (vector format not supported)");
            return;
        }

        // HTTP URL: download image bytes
        match download_image(src) {
            Ok((bytes, content_type)) => {
                // Reject SVG by content-type after download
                if content_type.as_deref() == Some("image/svg+xml") {
                    tracing::warn!(
                        "Cannot copy SVG image to clipboard (vector format not supported)"
                    );
                    return;
                }
                bytes
            }
            Err(e) => {
                tracing::error!(%e, %src, "Failed to download image for copy");
                return;
            }
        }
    } else {
        tracing::error!(%src, "Unsupported image source for clipboard copy");
        return;
    };

    copy_image_bytes(&image_bytes);
}

/// Copy raw image bytes to the system clipboard.
fn copy_image_bytes(image_bytes: &[u8]) {
    // Load image and convert to RGBA
    let img = match image::load_from_memory(image_bytes) {
        Ok(img) => img,
        Err(e) => {
            tracing::error!(%e, "Failed to load image from bytes");
            return;
        }
    };

    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // Create ImageData for arboard
    let image_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: rgba_img.into_raw().into(),
    };

    // Copy to clipboard
    let mut clipboard = CLIPBOARD.lock().unwrap();
    if let Err(e) = clipboard.set_image(image_data) {
        tracing::error!(%e, "Failed to copy image to clipboard");
    }
}
