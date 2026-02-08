use crate::theme::{resolve_theme, Theme};

pub fn build_custom_index(theme: Theme) -> String {
    let resolved = resolve_theme(theme);
    indoc::formatdoc! {r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>Arto</title>
            <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
            <!-- CUSTOM HEAD -->
        </head>
        <body data-theme="{resolved}">
            <div id="main"></div>
            <!-- MODULE LOADER -->
        </body>
    </html>
    "#}
}

pub(crate) fn build_mermaid_window_index(theme: Theme) -> String {
    let resolved = resolve_theme(theme);
    indoc::formatdoc! {r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>Mermaid Viewer - Arto</title>
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <!-- CUSTOM HEAD -->
        </head>
        <body data-theme="{resolved}" class="mermaid-window-body">
            <div id="main"></div>
            <!-- MODULE LOADER -->
        </body>
    </html>
    "#}
}

pub(crate) fn build_image_window_index(theme: Theme, image_data_url: &str) -> String {
    let resolved = resolve_theme(theme);
    // Use serde_json for safe JS string escaping
    let escaped_data_url = serde_json::to_string(image_data_url).unwrap();
    indoc::formatdoc! {r#"
    <!DOCTYPE html>
    <html>
        <head>
            <title>Image Viewer - Arto</title>
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <!-- CUSTOM HEAD -->
        </head>
        <body data-theme="{resolved}" class="image-window-body">
            <div id="main"></div>
            <script>window._imageDataUrl = {escaped_data_url};</script>
            <!-- MODULE LOADER -->
        </body>
    </html>
    "#}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_image_window_index_basic() {
        let data_url = "data:image/png;base64,abc123";
        let result = build_image_window_index(Theme::Dark, data_url);

        // Contains the JavaScript variable assignment
        assert!(result.contains("_imageDataUrl"));

        // Contains the data URL properly quoted by serde_json
        assert!(result.contains(r#""data:image/png;base64,abc123""#));

        // Contains the image-window-body class identifying this window type
        assert!(result.contains("image-window-body"));

        // Contains the body tag with resolved theme
        assert!(result.contains(r#"data-theme="dark""#));

        // Contains the correct title
        assert!(result.contains("<title>Image Viewer - Arto</title>"));
    }

    #[test]
    fn test_build_image_window_index_escaping() {
        // Data URL containing double quotes that need JSON escaping
        let data_url_with_quotes = r#"data:image/png;base64,abc"def'ghi"#;
        let result = build_image_window_index(Theme::Light, data_url_with_quotes);

        // The function should not panic
        assert!(result.contains("_imageDataUrl"));

        // serde_json wraps in double quotes and escapes inner double quotes as \"
        // so the raw unescaped string must NOT appear in the JS assignment
        assert!(!result.contains(r#"window._imageDataUrl = "data:image/png;base64,abc"def"#));

        // Verify the exact serde_json output is embedded in the result
        let escaped = serde_json::to_string(data_url_with_quotes).unwrap();
        assert!(result.contains(&escaped));

        // The body tag should have the light theme
        assert!(result.contains(r#"data-theme="light""#));
    }
}
