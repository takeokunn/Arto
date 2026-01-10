//! Image utility functions for saving and processing images.
//!
//! This module provides utilities for:
//! - Saving images from data URLs or HTTP/HTTPS URLs to files
//! - Extracting information from data URLs (MIME type, base64 data)
//! - Downloading images from external URLs

use base64::Engine;

/// Save an image from a URL (data URL or HTTP/HTTPS) to a file using a native save dialog.
///
/// Opens a file save dialog and writes the image to the selected path.
/// Supports:
/// - Data URLs: `data:image/png;base64,<base64-data>`
/// - HTTP/HTTPS URLs: `https://example.com/image.png`
///
/// # Examples
///
/// ```rust,ignore
/// // Data URL
/// save_image("data:image/png;base64,iVBORw0KGgo...");
///
/// // External URL
/// save_image("https://example.com/image.png");
/// ```
pub fn save_image(src: impl AsRef<str>) {
    use rfd::FileDialog;

    let src = src.as_ref();

    // Determine image source type and get bytes + file info
    let (image_bytes, filter_name, extensions, default_filename) = if src.starts_with("data:") {
        // Data URL: extract MIME type and decode base64
        let mime_type = extract_mime_type_from_data_url(src);
        let (filter_name, extensions, ext) = get_file_info_from_mime_type(mime_type);

        let base64_data = match extract_base64_from_data_url(src) {
            Ok(data) => data,
            Err(e) => {
                tracing::error!(%e, "Failed to extract base64 data from data URL");
                return;
            }
        };

        let bytes = match base64::prelude::BASE64_STANDARD.decode(base64_data) {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!(%e, "Failed to decode base64 image data");
                return;
            }
        };

        (bytes, filter_name, extensions, format!("image.{}", ext))
    } else if src.starts_with("http://") || src.starts_with("https://") {
        // External URL: download the image
        let (bytes, content_type) = match download_image(src) {
            Ok(result) => result,
            Err(e) => {
                tracing::error!(%e, %src, "Failed to download image");
                return;
            }
        };

        // Determine file info from content type or URL extension
        let (filter_name, extensions, ext) = if content_type.is_some() {
            get_file_info_from_mime_type(content_type.as_deref())
        } else {
            // Fall back to URL extension
            let url_ext = extract_extension_from_url(src);
            get_file_info_from_mime_type(url_ext.map(|e| match e {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                "svg" => "image/svg+xml",
                "bmp" => "image/bmp",
                _ => "",
            }))
        };

        // Extract filename from URL or use default
        let filename = extract_filename_from_url(src).unwrap_or_else(|| format!("image.{}", ext));

        (bytes, filter_name, extensions, filename)
    } else {
        tracing::error!(%src, "Unsupported image source format");
        return;
    };

    // Show save dialog
    let Some(path) = FileDialog::new()
        .add_filter(filter_name, &extensions)
        .set_file_name(default_filename)
        .save_file()
    else {
        return; // User cancelled
    };

    // Write to file
    if let Err(e) = std::fs::write(&path, image_bytes) {
        tracing::error!(%e, ?path, "Failed to save image to file");
    }
}

/// Get file filter info (filter name, extensions, default extension) from MIME type.
///
/// Returns a tuple of (filter_name, extensions, default_extension).
fn get_file_info_from_mime_type(
    mime_type: Option<&str>,
) -> (&'static str, Vec<&'static str>, &'static str) {
    match mime_type {
        Some("image/png") => ("PNG Image", vec!["png"], "png"),
        Some("image/jpeg") => ("JPEG Image", vec!["jpg", "jpeg"], "jpg"),
        Some("image/gif") => ("GIF Image", vec!["gif"], "gif"),
        Some("image/webp") => ("WebP Image", vec!["webp"], "webp"),
        Some("image/svg+xml") => ("SVG Image", vec!["svg"], "svg"),
        Some("image/bmp") => ("BMP Image", vec!["bmp"], "bmp"),
        _ => ("Image", vec!["png", "jpg", "gif", "webp"], "png"),
    }
}

/// Download an image from an HTTP/HTTPS URL.
///
/// Returns the image bytes and the content-type header if available.
fn download_image(url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_string());

    let bytes = response
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok((bytes, content_type))
}

/// Extract the filename from a URL path.
fn extract_filename_from_url(url: &str) -> Option<String> {
    let path = url.split('?').next()?; // Remove query string
    let filename = path.rsplit('/').next()?;
    if filename.is_empty() || !filename.contains('.') {
        return None;
    }
    Some(filename.to_string())
}

/// Extract the file extension from a URL path.
fn extract_extension_from_url(url: &str) -> Option<&str> {
    let path = url.split('?').next()?; // Remove query string
    let filename = path.rsplit('/').next()?;
    let ext = filename.rsplit('.').next()?;
    if ext == filename {
        return None; // No extension found
    }
    Some(ext)
}

/// Extract the MIME type from a data URL.
///
/// Expects format: `data:<mime-type>;base64,<base64-data>`
/// Returns the MIME type portion (e.g., "image/png").
pub fn extract_mime_type_from_data_url(data_url: &str) -> Option<&str> {
    // data:image/png;base64,<data>
    let stripped = data_url.strip_prefix("data:")?;
    let semicolon_pos = stripped.find(';')?;
    Some(&stripped[..semicolon_pos])
}

/// Extract base64 data from a data URL.
///
/// Expects format: `data:<mime-type>;base64,<base64-data>`
pub fn extract_base64_from_data_url(data_url: &str) -> Result<&str, &'static str> {
    // data:image/png;base64,<data>
    let Some(comma_pos) = data_url.find(',') else {
        return Err("Invalid data URL: missing comma separator");
    };

    let prefix = &data_url[..comma_pos];
    if !prefix.contains(";base64") {
        return Err("Invalid data URL: not base64 encoded");
    }

    Ok(&data_url[comma_pos + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base64_from_data_url() {
        let data_url = "data:image/png;base64,iVBORw0KGgo=";
        let base64 = extract_base64_from_data_url(data_url).unwrap();
        assert_eq!(base64, "iVBORw0KGgo=");
    }

    #[test]
    fn test_extract_base64_from_data_url_invalid() {
        let data_url = "not a data url";
        assert!(extract_base64_from_data_url(data_url).is_err());

        let data_url = "data:image/png,notbase64";
        assert!(extract_base64_from_data_url(data_url).is_err());
    }

    #[test]
    fn test_extract_mime_type_from_data_url() {
        assert_eq!(
            extract_mime_type_from_data_url("data:image/png;base64,abc"),
            Some("image/png")
        );
        assert_eq!(
            extract_mime_type_from_data_url("data:image/jpeg;base64,abc"),
            Some("image/jpeg")
        );
        assert_eq!(
            extract_mime_type_from_data_url("data:image/svg+xml;base64,abc"),
            Some("image/svg+xml")
        );
    }

    #[test]
    fn test_extract_mime_type_from_data_url_invalid() {
        assert_eq!(extract_mime_type_from_data_url("not a data url"), None);
        assert_eq!(extract_mime_type_from_data_url("data:image/png"), None);
    }

    #[test]
    fn test_extract_filename_from_url() {
        assert_eq!(
            extract_filename_from_url("https://example.com/images/photo.png"),
            Some("photo.png".to_string())
        );
        assert_eq!(
            extract_filename_from_url("https://example.com/image.jpg?size=large"),
            Some("image.jpg".to_string())
        );
        assert_eq!(extract_filename_from_url("https://example.com/path/"), None);
        assert_eq!(
            extract_filename_from_url("https://example.com/noextension"),
            None
        );
    }

    #[test]
    fn test_extract_extension_from_url() {
        assert_eq!(
            extract_extension_from_url("https://example.com/image.png"),
            Some("png")
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/photo.jpeg?q=80"),
            Some("jpeg")
        );
        assert_eq!(
            extract_extension_from_url("https://example.com/noext"),
            None
        );
    }
}
