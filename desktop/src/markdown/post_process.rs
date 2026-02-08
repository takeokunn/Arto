use base64::{engine::general_purpose, Engine as _};
use lol_html::{element, HtmlRewriter, Settings};
use std::path::Path;

use super::headings::HeadingInfo;

/// Infer MIME type from file extension
pub(super) fn get_mime_type(path: &Path) -> &'static str {
    crate::utils::image::get_mime_type_from_extension(path)
}

/// Post-process HTML to handle img, anchor, and table tags using lol_html
pub(super) fn post_process_html_tags(
    html_str: &str,
    base_dir: &Path,
    table_source_lines: &[(usize, usize)],
) -> String {
    let base_dir = base_dir.to_path_buf();
    let mut output = Vec::new();
    let table_index = std::cell::RefCell::new(0usize);
    let table_source_lines = table_source_lines.to_vec();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                // Process table tags: inject source line attributes
                element!("table", |el| {
                    let mut idx = table_index.borrow_mut();
                    if let Some(&(start, end)) = table_source_lines.get(*idx) {
                        el.set_attribute("data-source-line", &start.to_string())?;
                        el.set_attribute("data-source-line-end", &end.to_string())?;
                    }
                    *idx += 1;
                    Ok(())
                }),
                // Process img tags: convert relative paths to data URLs
                element!("img[src]", move |el| {
                    if let Some(src) = el.get_attribute("src") {
                        if !src.starts_with("http://")
                            && !src.starts_with("https://")
                            && !src.starts_with("data:")
                        {
                            let absolute_path = base_dir.join(&src);
                            if let Ok(canonical_path) = absolute_path.canonicalize() {
                                if let Ok(image_data) = std::fs::read(&canonical_path) {
                                    let mime_type = get_mime_type(&canonical_path);
                                    let base64_data = general_purpose::STANDARD.encode(&image_data);
                                    let data_url =
                                        format!("data:{};base64,{}", mime_type, base64_data);
                                    el.set_attribute(
                                        "data-original-src",
                                        &canonical_path.to_string_lossy(),
                                    )?;
                                    el.set_attribute("src", &data_url)?;
                                }
                            }
                        }
                    }
                    Ok(())
                }),
                // Process anchor tags: convert markdown links to spans
                element!("a[href]", |el| {
                    if let Some(href) = el.get_attribute("href") {
                        if !href.starts_with("http://") && !href.starts_with("https://") {
                            if let Some(ext) = std::path::Path::new(&href)
                                .extension()
                                .and_then(|e| e.to_str())
                            {
                                // Replace with span element
                                let escaped_href = href.replace('\'', "\\'");
                                let onclick = indoc::formatdoc! {r#"
                                        if (event.button === 0 || event.button === 1) {{
                                            event.preventDefault();
                                            window.handleMarkdownLinkClick('{escaped_href}', event.button);
                                        }}"#
                                };
                                el.set_tag_name("span")?;
                                el.remove_attribute("href");
                                if ext != "md" && ext != "markdown" {
                                    el.set_attribute("class", "md-link md-link-invalid")?;
                                } else {
                                    el.set_attribute("class", "md-link")?;
                                }
                                el.set_attribute("onmousedown", &onclick)?;
                            }
                        }
                    }
                    Ok(())
                }),
            ],
            ..Settings::default()
        },
        |chunk: &[u8]| {
            output.extend_from_slice(chunk);
        },
    );

    let _ = rewriter.write(html_str.as_bytes());
    let _ = rewriter.end();
    String::from_utf8(output).unwrap_or_else(|_| html_str.to_string())
}

/// Post-process HTML to handle img, anchor, table tags, and add heading IDs using lol_html
pub(super) fn post_process_html_with_headings(
    html_str: &str,
    base_dir: &Path,
    headings: &[HeadingInfo],
    table_source_lines: &[(usize, usize)],
) -> String {
    let base_dir = base_dir.to_path_buf();
    let mut output = Vec::new();
    let heading_index = std::cell::RefCell::new(0usize);
    let headings = headings.to_vec();
    let table_index = std::cell::RefCell::new(0usize);
    let table_source_lines = table_source_lines.to_vec();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                // Process table tags: inject source line attributes
                element!("table", |el| {
                    let mut idx = table_index.borrow_mut();
                    if let Some(&(start, end)) = table_source_lines.get(*idx) {
                        el.set_attribute("data-source-line", &start.to_string())?;
                        el.set_attribute("data-source-line-end", &end.to_string())?;
                    }
                    *idx += 1;
                    Ok(())
                }),
                // Process heading tags: add IDs for TOC navigation
                element!("h1, h2, h3, h4, h5, h6", |el| {
                    let mut idx = heading_index.borrow_mut();
                    if let Some(heading) = headings.get(*idx) {
                        el.set_attribute("id", &heading.id)?;
                    }
                    *idx += 1;
                    Ok(())
                }),
                // Process img tags: convert relative paths to data URLs
                element!("img[src]", move |el| {
                    if let Some(src) = el.get_attribute("src") {
                        if !src.starts_with("http://")
                            && !src.starts_with("https://")
                            && !src.starts_with("data:")
                        {
                            let absolute_path = base_dir.join(&src);
                            if let Ok(canonical_path) = absolute_path.canonicalize() {
                                if let Ok(image_data) = std::fs::read(&canonical_path) {
                                    let mime_type = get_mime_type(&canonical_path);
                                    let base64_data = general_purpose::STANDARD.encode(&image_data);
                                    let data_url =
                                        format!("data:{};base64,{}", mime_type, base64_data);
                                    el.set_attribute(
                                        "data-original-src",
                                        &canonical_path.to_string_lossy(),
                                    )?;
                                    el.set_attribute("src", &data_url)?;
                                }
                            }
                        }
                    }
                    Ok(())
                }),
                // Process anchor tags: convert markdown links to spans
                element!("a[href]", |el| {
                    if let Some(href) = el.get_attribute("href") {
                        if !href.starts_with("http://") && !href.starts_with("https://") {
                            if let Some(ext) = std::path::Path::new(&href)
                                .extension()
                                .and_then(|e| e.to_str())
                            {
                                // Replace with span element
                                let escaped_href = href.replace('\'', "\\'");
                                let onclick = indoc::formatdoc! {r#"
                                        if (event.button === 0 || event.button === 1) {{
                                            event.preventDefault();
                                            window.handleMarkdownLinkClick('{escaped_href}', event.button);
                                        }}"#
                                };
                                el.set_tag_name("span")?;
                                el.remove_attribute("href");
                                if ext != "md" && ext != "markdown" {
                                    el.set_attribute("class", "md-link md-link-invalid")?;
                                } else {
                                    el.set_attribute("class", "md-link")?;
                                }
                                el.set_attribute("onmousedown", &onclick)?;
                            }
                        }
                    }
                    Ok(())
                }),
            ],
            ..Settings::default()
        },
        |chunk: &[u8]| {
            output.extend_from_slice(chunk);
        },
    );

    let _ = rewriter.write(html_str.as_bytes());
    let _ = rewriter.end();
    String::from_utf8(output).unwrap_or_else(|_| html_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_mime_type() {
        assert_eq!(get_mime_type(Path::new("test.png")), "image/png");
        assert_eq!(get_mime_type(Path::new("test.jpg")), "image/jpeg");
        assert_eq!(get_mime_type(Path::new("test.jpeg")), "image/jpeg");
        assert_eq!(get_mime_type(Path::new("test.gif")), "image/gif");
        assert_eq!(get_mime_type(Path::new("test.svg")), "image/svg+xml");
        assert_eq!(get_mime_type(Path::new("test.webp")), "image/webp");
        assert_eq!(get_mime_type(Path::new("test.bmp")), "image/bmp");
        assert_eq!(get_mime_type(Path::new("test.ico")), "image/x-icon");
        assert_eq!(get_mime_type(Path::new("test.unknown")), "image/png");
    }

    #[test]
    fn test_post_process_html_tags_img() {
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.png");
        let png_data = vec![0x89, 0x50, 0x4E, 0x47];
        fs::write(&image_path, png_data).unwrap();

        let html = r#"<p><img src="test.png" alt="test" /></p>"#;
        let result = post_process_html_tags(html, temp_dir.path(), &[]);

        assert!(
            result.contains("data:image/png;base64,"),
            "Should convert img src to data URL"
        );
        assert!(
            !result.contains(r#"src="test.png""#),
            "Should not contain original path"
        );
        assert!(
            result.contains("data-original-src="),
            "Should preserve original path in data-original-src attribute"
        );
    }

    #[test]
    fn test_post_process_html_tags_anchor() {
        let html = r#"<a href="doc.md">Link</a>"#;
        let result = post_process_html_tags(html, Path::new("."), &[]);

        assert!(
            result.contains(r#"<span class="md-link""#),
            "Should convert to span"
        );
        assert!(
            result.contains("handleMarkdownLinkClick"),
            "Should add click handler"
        );
        assert!(!result.contains("<a "), "Should not contain anchor tag");
    }

    #[test]
    fn test_post_process_html_tags_http_urls() {
        let html =
            r#"<img src="https://example.com/image.png" /><a href="https://example.com">Link</a>"#;
        let result = post_process_html_tags(html, Path::new("."), &[]);

        assert!(
            result.contains(r#"src="https://example.com/image.png""#),
            "Should keep HTTP img"
        );
        assert!(
            result.contains(r#"<a href="https://example.com""#),
            "Should keep HTTP link"
        );
    }

    #[test]
    fn test_post_process_html_tags_non_md_local_file() {
        let html = r#"<a href="file.txt">Text File</a>"#;
        let result = post_process_html_tags(html, Path::new("."), &[]);

        assert!(
            result.contains(r#"<span class="md-link md-link-invalid""#),
            "Should convert to span with md-link and md-link-invalid class"
        );
        assert!(
            result.contains("handleMarkdownLinkClick"),
            "Should add click handler for local files"
        );
        assert!(!result.contains("<a "), "Should not contain anchor tag");
    }

    #[test]
    fn test_post_process_html_tags_md_vs_other_files() {
        let html = r#"<a href="doc.md">MD</a><a href="file.txt">TXT</a>"#;
        let result = post_process_html_tags(html, Path::new("."), &[]);

        // MD file should have only md-link class
        assert!(
            result.contains(r#"class="md-link""#),
            "Should have md-link for .md file"
        );

        // TXT file should have both md-link and md-link-invalid classes
        assert!(
            result.contains(r#"class="md-link md-link-invalid""#),
            "Should have md-link and md-link-invalid for .txt file"
        );

        // Both should have click handlers
        let click_handler_count = result.matches("handleMarkdownLinkClick").count();
        assert_eq!(
            click_handler_count, 2,
            "Should have click handlers for both links"
        );
    }

    #[test]
    fn test_post_process_html_with_headings_injects_ids() {
        let html = r#"<h1 data-source-line="1">Title</h1><h2 data-source-line="3">Section</h2>"#;
        let headings = vec![
            HeadingInfo {
                level: 1,
                text: "Title".to_string(),
                id: "title".to_string(),
            },
            HeadingInfo {
                level: 2,
                text: "Section".to_string(),
                id: "section".to_string(),
            },
        ];

        let result = post_process_html_with_headings(html, Path::new("."), &headings, &[]);

        assert!(
            result.contains(r#"id="title""#),
            "H1 should get id from headings: {result}"
        );
        assert!(
            result.contains(r#"id="section""#),
            "H2 should get id from headings: {result}"
        );
    }

    #[test]
    fn test_post_process_html_with_headings_more_html_headings_than_info() {
        // When HTML has more headings than HeadingInfo entries, extra headings are skipped
        let html = r#"<h1>A</h1><h2>B</h2><h3>C</h3>"#;
        let headings = vec![HeadingInfo {
            level: 1,
            text: "A".to_string(),
            id: "a".to_string(),
        }];

        let result = post_process_html_with_headings(html, Path::new("."), &headings, &[]);

        assert!(
            result.contains(r#"id="a""#),
            "First heading should get id: {result}"
        );
        // Remaining headings should still render without error
        assert!(
            result.contains("<h2>B</h2>") || result.contains("<h2 >B</h2>"),
            "Extra headings should render without id: {result}"
        );
    }

    #[test]
    fn test_post_process_html_with_headings_empty_headings() {
        let html = r#"<h1>Title</h1>"#;
        let headings: Vec<HeadingInfo> = vec![];

        let result = post_process_html_with_headings(html, Path::new("."), &headings, &[]);

        // Should not crash, heading renders without id
        assert!(
            result.contains("Title"),
            "Should still render heading text: {result}"
        );
    }

    #[test]
    fn test_data_original_src_not_set_for_http() {
        let html = indoc::indoc! {r#"
            <p><img src="https://example.com/image.png" alt="remote" /></p>
        "#};
        let result = post_process_html_tags(html, Path::new("."), &[]);

        assert!(
            !result.contains("data-original-src"),
            "HTTP image URLs should NOT receive a data-original-src attribute"
        );
        assert!(
            result.contains(r#"src="https://example.com/image.png""#),
            "HTTP image URL should be preserved as-is"
        );
    }

    #[test]
    fn test_data_original_src_not_set_for_data_urls() {
        let html = indoc::indoc! {r#"
            <p><img src="data:image/png;base64,iVBORw0KGgo=" alt="embedded" /></p>
        "#};
        let result = post_process_html_tags(html, Path::new("."), &[]);

        assert!(
            !result.contains("data-original-src"),
            "Data URL images should NOT receive a data-original-src attribute"
        );
        assert!(
            result.contains("data:image/png;base64,iVBORw0KGgo="),
            "Data URL should be preserved as-is in the src attribute"
        );
    }
}
