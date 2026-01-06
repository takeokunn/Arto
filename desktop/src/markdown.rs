use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use lol_html::{element, HtmlRewriter, Settings};
use pulldown_cmark::{html, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_yaml::Value as YamlValue;
use std::path::{Path, PathBuf};

/// Information about a heading extracted from markdown
#[derive(Debug, Clone, PartialEq)]
pub struct HeadingInfo {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text content
    pub text: String,
    /// Generated anchor ID for linking
    pub id: String,
}

/// Generate a URL-safe slug from heading text
fn generate_slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' || c == '.' {
                '-'
            } else {
                // Skip other characters (including non-ASCII)
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Extract headings from markdown content
pub fn extract_headings(markdown: &str) -> Vec<HeadingInfo> {
    let options = Options::all();

    // Skip frontmatter if present
    let content = if let Some(after_start) = markdown.strip_prefix("---") {
        if let Some(end_pos) = after_start.find("\n---") {
            after_start[end_pos + 4..].trim_start()
        } else {
            markdown
        }
    } else {
        markdown
    };

    // Process GitHub alerts (they contain their own parsing)
    let processed = process_github_alerts(content);
    let parser = Parser::new_ext(&processed, options);

    let mut headings = Vec::new();
    let mut current_level: Option<u8> = None;
    let mut current_text = String::new();
    let mut slug_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_level = Some(match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                });
                current_text.clear();
            }
            Event::Text(text) if current_level.is_some() => {
                current_text.push_str(&text);
            }
            Event::Code(code) if current_level.is_some() => {
                current_text.push_str(&code);
            }
            Event::SoftBreak | Event::HardBreak if current_level.is_some() => {
                current_text.push(' ');
            }
            Event::End(TagEnd::Heading(_)) if current_level.is_some() => {
                let level = current_level.take().unwrap();
                let base_slug = generate_slug(&current_text);

                // Handle duplicate slugs by appending a number
                let id = if let Some(count) = slug_counts.get(&base_slug) {
                    let new_count = count + 1;
                    slug_counts.insert(base_slug.clone(), new_count);
                    format!("{}-{}", base_slug, new_count)
                } else {
                    slug_counts.insert(base_slug.clone(), 0);
                    base_slug
                };

                headings.push(HeadingInfo {
                    level,
                    text: current_text.trim().to_string(),
                    id,
                });
            }
            _ => {}
        }
    }

    headings
}

/// Render Markdown to HTML
pub fn render_to_html(markdown: impl AsRef<str>, base_path: impl AsRef<Path>) -> Result<String> {
    let markdown = markdown.as_ref();
    let base_path = base_path.as_ref();

    // Enable GitHub Flavored Markdown options
    let options = Options::all();

    // Get base directory for resolving relative paths
    let base_dir = base_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Extract frontmatter if present
    let (frontmatter_html, content) = extract_and_render_frontmatter(markdown);

    // Process GitHub alerts
    let processed_markdown = process_github_alerts(&content);

    // Parse Markdown and process blocks
    let parser = Parser::new_ext(&processed_markdown, options);
    let parser = process_code_blocks(parser, "mermaid");
    let parser = process_code_blocks(parser, "math");
    let parser = process_math_expressions(parser);

    // Convert to HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Post-process HTML to handle all img and anchor tags (both from Markdown syntax and HTML tags)
    let html_output = post_process_html_tags(&html_output, base_dir.as_path());

    // Prepend frontmatter table if present
    let final_output = if frontmatter_html.is_empty() {
        html_output
    } else {
        format!("{}\n{}", frontmatter_html, html_output)
    };

    Ok(final_output)
}

/// Extract frontmatter from markdown and render it as an HTML table
fn extract_and_render_frontmatter(markdown: &str) -> (String, String) {
    // Check if markdown starts with frontmatter delimiter
    if !markdown.starts_with("---") {
        return (String::new(), markdown.to_string());
    }

    // Find the closing delimiter
    let rest = &markdown[3..];
    let Some(end_pos) = rest.find("\n---") else {
        return (String::new(), markdown.to_string());
    };

    let frontmatter_str = rest[..end_pos].trim();
    let content = rest[end_pos + 4..].trim_start();

    // Parse YAML
    let Ok(yaml) = serde_yaml::from_str::<YamlValue>(frontmatter_str) else {
        return (String::new(), markdown.to_string());
    };

    // Render frontmatter as table
    let html = render_frontmatter_table(&yaml);

    (html, content.to_string())
}

/// Render YAML frontmatter as an HTML table
fn render_frontmatter_table(yaml: &YamlValue) -> String {
    let YamlValue::Mapping(mapping) = yaml else {
        return String::new();
    };

    if mapping.is_empty() {
        return String::new();
    }

    let mut rows = String::new();
    for (key, value) in mapping {
        let key_str = yaml_to_string(key);
        let value_str = render_yaml_value(value);
        rows.push_str(&format!(
            "<tr><th>{}</th><td>{}</td></tr>\n",
            html_escape::encode_text(&key_str),
            value_str
        ));
    }

    format!(
        r#"<details class="frontmatter">
<summary class="frontmatter-summary">Frontmatter</summary>
<table class="frontmatter-table">
<tbody>
{}
</tbody>
</table>
</details>"#,
        rows
    )
}

/// Convert a YAML value to a string representation
fn yaml_to_string(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "null".to_string(),
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => s.clone(),
        YamlValue::Sequence(seq) => seq
            .iter()
            .map(yaml_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        YamlValue::Mapping(_) => "[object]".to_string(),
        YamlValue::Tagged(tagged) => yaml_to_string(&tagged.value),
    }
}

/// Render a YAML value as HTML (with special handling for arrays and objects)
fn render_yaml_value(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "<span class=\"yaml-null\">null</span>".to_string(),
        YamlValue::Bool(b) => format!("<span class=\"yaml-bool\">{}</span>", b),
        YamlValue::Number(n) => format!("<span class=\"yaml-number\">{}</span>", n),
        YamlValue::String(s) => html_escape::encode_text(s).to_string(),
        YamlValue::Sequence(seq) => {
            if seq.is_empty() {
                return "<span class=\"yaml-empty\">[]</span>".to_string();
            }
            let items: Vec<String> = seq
                .iter()
                .map(|v| format!("<li>{}</li>", render_yaml_value(v)))
                .collect();
            format!("<ul class=\"yaml-list\">{}</ul>", items.join(""))
        }
        YamlValue::Mapping(mapping) => {
            if mapping.is_empty() {
                return "<span class=\"yaml-empty\">{{}}</span>".to_string();
            }
            let rows: Vec<String> = mapping
                .iter()
                .map(|(k, v)| {
                    format!(
                        "<tr><th>{}</th><td>{}</td></tr>",
                        html_escape::encode_text(&yaml_to_string(k)),
                        render_yaml_value(v)
                    )
                })
                .collect();
            format!(
                "<table class=\"yaml-nested-table\"><tbody>{}</tbody></table>",
                rows.join("")
            )
        }
        YamlValue::Tagged(tagged) => render_yaml_value(&tagged.value),
    }
}

/// Get SVG icon placeholder for alert type (actual SVG injected by JavaScript)
fn get_alert_icon_placeholder(alert_type: &str) -> String {
    format!(
        r#"<span class="alert-icon" data-alert-type="{}"></span>"#,
        alert_type
    )
}

/// Check if a line starts a GitHub alert and return alert info
fn parse_alert_start(line: &str) -> Option<(&'static str, &'static str, &str)> {
    const ALERT_TYPES: [(&str, &str); 5] = [
        ("NOTE", "note"),
        ("TIP", "tip"),
        ("IMPORTANT", "important"),
        ("WARNING", "warning"),
        ("CAUTION", "caution"),
    ];

    for &(alert_name, alert_class) in &ALERT_TYPES {
        if let Some(rest) = line.strip_prefix(&format!("> [!{}]", alert_name)) {
            return Some((alert_name, alert_class, rest));
        }
    }
    None
}

/// Process a single alert block and return HTML lines and next index
fn process_alert_block(
    lines: &[&str],
    start_index: usize,
    alert_name: &str,
    alert_class: &str,
    first_line_content: &str,
) -> (Vec<String>, usize) {
    let mut html_lines = Vec::new();

    // Alert opening tag
    html_lines.push(format!(
        r#"<div class="markdown-alert markdown-alert-{}" dir="auto">"#,
        alert_class
    ));

    // Alert title with icon
    let icon_placeholder = get_alert_icon_placeholder(alert_class);
    html_lines.push(format!(
        r#"<p class="markdown-alert-title" dir="auto">{}{}</p>"#,
        icon_placeholder, alert_name
    ));

    // Collect alert content as markdown
    let mut content_lines = Vec::new();
    if !first_line_content.trim().is_empty() {
        content_lines.push(first_line_content.trim().to_string());
    }

    // Collect following quoted lines
    let mut i = start_index + 1;
    while i < lines.len() && lines[i].starts_with('>') {
        if let Some(content) = lines[i].strip_prefix('>') {
            // Preserve the structure by keeping leading space after '>'
            content_lines.push(content.trim_start().to_string());
        }
        i += 1;
    }

    // Render the collected content as markdown
    if !content_lines.is_empty() {
        let content_markdown = content_lines.join("\n");
        let options = Options::all();
        let parser = Parser::new_ext(&content_markdown, options);
        let mut content_html = String::new();
        html::push_html(&mut content_html, parser);
        html_lines.push(content_html);
    }

    html_lines.push("</div>".to_string());

    (html_lines, i)
}

/// Process GitHub alert format
fn process_github_alerts(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if let Some((alert_name, alert_class, rest)) = parse_alert_start(line) {
            let (alert_html, next_index) =
                process_alert_block(&lines, i, alert_name, alert_class, rest);
            result.extend(alert_html);
            i = next_index;
        } else {
            result.push(line.to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Process Code blocks
fn process_code_blocks<'a>(
    parser: impl Iterator<Item = Event<'a>>,
    target_lang: &'a str,
) -> impl Iterator<Item = Event<'a>> {
    let mut in_block = false;
    let mut content = String::new();

    parser.flat_map(move |event| match event {
        Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
            if lang.as_ref() == target_lang =>
        {
            in_block = true;
            content.clear();
            vec![]
        }
        Event::End(TagEnd::CodeBlock) if in_block => {
            in_block = false;
            // Store original content in data attribute for JavaScript processing
            let html = format!(
                r#"<pre class="preprocessed-{}" data-original-content="{}">{}</pre>"#,
                target_lang,
                html_escape::encode_double_quoted_attribute(&content),
                &content,
            );
            vec![Event::Html(html.into())]
        }
        Event::Text(text) if in_block => {
            content.push_str(&text);
            vec![]
        }
        _ => vec![event],
    })
}

/// Process math expressions (inline and display)
fn process_math_expressions<'a>(
    parser: impl Iterator<Item = Event<'a>>,
) -> impl Iterator<Item = Event<'a>> {
    parser.map(|event| match event {
        Event::InlineMath(content) => {
            // Convert inline math to custom HTML structure
            let html = format!(
                r#"<span class="preprocessed-math-inline" data-original-content="{}">{}</span>"#,
                html_escape::encode_text(&content),
                &content,
            );
            Event::Html(html.into())
        }
        Event::DisplayMath(content) => {
            // Convert display math to custom HTML structure
            let html = format!(
                r#"<div class="preprocessed-math-display" data-original-content="{}">{}</div>"#,
                html_escape::encode_text(&content),
                &content,
            );
            Event::Html(html.into())
        }
        other => other,
    })
}

/// Infer MIME type from file extension
fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        _ => "image/png", // Default
    }
}

/// Render Markdown to HTML with TOC information
///
/// Returns a tuple of (rendered HTML with heading IDs, extracted headings)
pub fn render_to_html_with_toc(
    markdown: impl AsRef<str>,
    base_path: impl AsRef<Path>,
) -> Result<(String, Vec<HeadingInfo>)> {
    let markdown = markdown.as_ref();
    let base_path = base_path.as_ref();

    // Extract headings first
    let headings = extract_headings(markdown);

    // Enable GitHub Flavored Markdown options
    let options = Options::all();

    // Get base directory for resolving relative paths
    let base_dir = base_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    // Extract frontmatter if present
    let (frontmatter_html, content) = extract_and_render_frontmatter(markdown);

    // Process GitHub alerts
    let processed_markdown = process_github_alerts(&content);

    // Parse Markdown and process blocks
    let parser = Parser::new_ext(&processed_markdown, options);
    let parser = process_code_blocks(parser, "mermaid");
    let parser = process_code_blocks(parser, "math");
    let parser = process_math_expressions(parser);

    // Convert to HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Post-process HTML with heading IDs
    let html_output = post_process_html_with_headings(&html_output, base_dir.as_path(), &headings);

    // Prepend frontmatter table if present
    let final_output = if frontmatter_html.is_empty() {
        html_output
    } else {
        format!("{}\n{}", frontmatter_html, html_output)
    };

    Ok((final_output, headings))
}

/// Post-process HTML to handle img, anchor tags, and add heading IDs using lol_html
fn post_process_html_with_headings(
    html_str: &str,
    base_dir: &Path,
    headings: &[HeadingInfo],
) -> String {
    let base_dir = base_dir.to_path_buf();
    let mut output = Vec::new();
    let heading_index = std::cell::RefCell::new(0usize);
    let headings = headings.to_vec();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
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

/// Post-process HTML to handle img and anchor tags using lol_html
fn post_process_html_tags(html_str: &str, base_dir: &Path) -> String {
    let base_dir = base_dir.to_path_buf();
    let mut output = Vec::new();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
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
    use indoc::indoc;
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
    fn test_get_alert_icon_placeholder() {
        let result = get_alert_icon_placeholder("note");
        assert_eq!(
            result,
            r#"<span class="alert-icon" data-alert-type="note"></span>"#
        );

        let result = get_alert_icon_placeholder("warning");
        assert_eq!(
            result,
            r#"<span class="alert-icon" data-alert-type="warning"></span>"#
        );
    }

    #[test]
    fn test_process_github_alerts_note() {
        let input = indoc! {"
            > [!NOTE]
            > This is a note
        "};
        let result = process_github_alerts(input);

        assert!(result.contains(r#"<div class="markdown-alert markdown-alert-note""#));
        assert!(result.contains(r#"<p class="markdown-alert-title""#));
        assert!(result.contains("NOTE"));
        assert!(result.contains("This is a note"));
        assert!(result.contains("</div>"));
    }

    #[test]
    fn test_process_github_alerts_warning() {
        let input = indoc! {"
            > [!WARNING]
            > Be careful!
        "};
        let result = process_github_alerts(input);

        assert!(result.contains(r#"markdown-alert-warning"#));
        assert!(result.contains("WARNING"));
        assert!(result.contains("Be careful!"));
    }

    #[test]
    fn test_process_github_alerts_with_multiline() {
        let input = indoc! {"
            > [!IMPORTANT]
            > First line
            > Second line
            > Third line
        "};
        let result = process_github_alerts(input);

        assert!(result.contains(r#"markdown-alert-important"#));
        assert!(result.contains("First line"));
        assert!(result.contains("Second line"));
        assert!(result.contains("Third line"));
    }

    #[test]
    fn test_process_github_alerts_all_types() {
        let alert_types = vec![
            ("NOTE", "note"),
            ("TIP", "tip"),
            ("IMPORTANT", "important"),
            ("WARNING", "warning"),
            ("CAUTION", "caution"),
        ];

        for (alert_name, alert_class) in alert_types {
            let input = format!("> [!{}]\n> Test content", alert_name);
            let result = process_github_alerts(&input);

            assert!(
                result.contains(&format!(r#"markdown-alert-{}"#, alert_class)),
                "Should contain alert class for {}",
                alert_name
            );
            assert!(
                result.contains(alert_name),
                "Should contain alert name {}",
                alert_name
            );
        }
    }

    #[test]
    fn test_process_github_alerts_no_match() {
        let input = "Regular paragraph\n> Regular quote";
        let result = process_github_alerts(input);

        assert_eq!(result, input);
        assert!(!result.contains("markdown-alert"));
    }

    #[test]
    fn test_process_mermaid_blocks() {
        let markdown = indoc! {"
            ```mermaid
            graph TD
                A-->B
            ```
        "};

        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);
        let events: Vec<Event> = process_code_blocks(parser, "mermaid").collect();

        // Verify that Mermaid block is converted to a single HTML event
        let html_events: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Html(html) = e {
                    Some(html.as_ref())
                } else {
                    None
                }
            })
            .collect();

        assert!(!html_events.is_empty(), "Should contain HTML event");
        let html = html_events[0];
        assert!(html.contains(r#"<pre class="preprocessed-mermaid""#));
        assert!(html.contains(r#"data-original-content="#));
        // Content is HTML-escaped in data attribute, so we just check the structure
        assert!(html.contains("</pre>"));
    }

    #[test]
    fn test_post_process_html_tags_img() {
        let temp_dir = TempDir::new().unwrap();
        let image_path = temp_dir.path().join("test.png");
        let png_data = vec![0x89, 0x50, 0x4E, 0x47];
        fs::write(&image_path, png_data).unwrap();

        let html = r#"<p><img src="test.png" alt="test" /></p>"#;
        let result = post_process_html_tags(html, temp_dir.path());

        assert!(
            result.contains("data:image/png;base64,"),
            "Should convert img src to data URL"
        );
        assert!(
            !result.contains(r#"src="test.png""#),
            "Should not contain original path"
        );
    }

    #[test]
    fn test_post_process_html_tags_anchor() {
        let html = r#"<a href="doc.md">Link</a>"#;
        let result = post_process_html_tags(html, Path::new("."));

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
        let result = post_process_html_tags(html, Path::new("."));

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
        let result = post_process_html_tags(html, Path::new("."));

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
        let result = post_process_html_tags(html, Path::new("."));

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
    fn test_render_to_html_basic() {
        let markdown = "# Hello\n\nThis is a test.";
        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        assert!(result.contains("<h1>"));
        assert!(result.contains("Hello"));
        assert!(result.contains("<p>"));
        assert!(result.contains("This is a test."));
    }

    #[test]
    fn test_code_block_language_classes() {
        let markdown = indoc! {"
            # Code Blocks Test

            ```rust
            fn main() {
                println!(\"Hello\");
            }
            ```

            ```python
            def hello():
                print(\"world\")
            ```

            ```
            no language specified
            ```
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        // Print the output to inspect
        println!("\n=== HTML OUTPUT ===\n{}\n===================\n", result);

        // Check if language classes are present
        let has_rust = result.contains("language-rust") || result.contains("class=\"rust\"");
        let has_python = result.contains("language-python") || result.contains("class=\"python\"");

        println!("Has rust class: {}", has_rust);
        println!("Has python class: {}", has_python);
    }

    #[test]
    fn test_render_to_html_with_alert() {
        let markdown = indoc! {"
            # Title

            > [!NOTE]
            > This is important
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        assert!(result.contains("markdown-alert-note"));
        assert!(result.contains("This is important"));
    }

    #[test]
    fn test_render_to_html_with_mermaid() {
        let markdown = indoc! {"
            ```mermaid
            graph LR
                A-->B
            ```
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        assert!(result.contains(r#"<pre class="preprocessed-mermaid""#));
        assert!(result.contains("graph LR"));
    }

    #[test]
    fn test_process_math_expressions_inline() {
        let markdown = "This is inline math: $x = y + z$";
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let events: Vec<Event> = process_math_expressions(parser).collect();

        // Verify that inline math is converted to custom HTML
        let html_events: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Html(html) = e {
                    Some(html.as_ref())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            html_events
                .iter()
                .any(|h| h.contains(r#"<span class="preprocessed-math-inline""#)),
            "Should contain inline-math span"
        );
        assert!(
            html_events
                .iter()
                .any(|h| h.contains("data-original-content")),
            "Should contain data attribute"
        );
        assert!(
            html_events.iter().any(|h| h.contains("x = y + z")),
            "Should contain the math content"
        );
    }

    #[test]
    fn test_process_math_expressions_display() {
        let markdown = indoc! {"
            Display math:

            $$
            x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}
            $$
        "};
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let events: Vec<Event> = process_math_expressions(parser).collect();

        // Verify that display math is converted to custom HTML
        let html_events: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Html(html) = e {
                    Some(html.as_ref())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            html_events
                .iter()
                .any(|h| h.contains(r#"<div class="preprocessed-math-display""#)),
            "Should contain display-math div"
        );
        assert!(
            html_events
                .iter()
                .any(|h| h.contains("data-original-content")),
            "Should contain data attribute"
        );
        assert!(
            html_events.iter().any(|h| h.contains("frac")),
            "Should contain the math content"
        );
    }

    #[test]
    fn test_process_math_expressions_mixed() {
        let markdown = "Inline $a + b$ and display $$c = d$$";
        let options = Options::all();
        let parser = Parser::new_ext(markdown, options);

        let events: Vec<Event> = process_math_expressions(parser).collect();

        let html_events: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let Event::Html(html) = e {
                    Some(html.as_ref())
                } else {
                    None
                }
            })
            .collect();

        // Should have both inline and display math
        assert!(
            html_events
                .iter()
                .any(|h| h.contains(r#"class="preprocessed-math-inline""#)),
            "Should contain inline math"
        );
        assert!(
            html_events
                .iter()
                .any(|h| h.contains(r#"class="preprocessed-math-display""#)),
            "Should contain display math"
        );
    }

    #[test]
    fn test_render_to_html_with_math() {
        let markdown = indoc! {"
            # Math Test

            Inline math: $E = mc^2$

            Display math:
            $$
            \\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}
            $$
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        assert!(
            result.contains(r#"class="preprocessed-math-inline""#),
            "Should render inline math"
        );
        assert!(
            result.contains(r#"class="preprocessed-math-display""#),
            "Should render display math"
        );
        assert!(
            result.contains("data-original-content"),
            "Should include data attributes"
        );
    }

    #[test]
    fn test_render_to_html_integrated() {
        // Integration test: combining multiple features
        let temp_dir = TempDir::new().unwrap();

        // Create test image
        let image_path = temp_dir.path().join("image.png");
        let png_data = vec![0x89, 0x50, 0x4E, 0x47];
        fs::write(&image_path, png_data).unwrap();

        let markdown = indoc! {"
            # Test Document

            > [!WARNING]
            > Be careful

            ![Test Image](image.png)

            [Link to other doc](other.md)

            ```mermaid
            graph TD
                A-->B
            ```
        "};

        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        // Verify that all features are correctly integrated
        assert!(result.contains("<h1>"), "Should render heading");
        assert!(
            result.contains("markdown-alert-warning"),
            "Should render alert"
        );
        assert!(
            result.contains("data:image/png"),
            "Should convert image to data URL"
        );
        assert!(
            result.contains(r#"class="md-link""#),
            "Should convert md link"
        );
        assert!(
            result.contains(r#"<pre class="preprocessed-mermaid""#),
            "Should render mermaid"
        );
    }

    #[test]
    fn test_extract_and_render_frontmatter_basic() {
        let markdown = indoc! {"
            ---
            title: Test Document
            author: John Doe
            ---

            # Hello World
        "};

        let (html, content) = extract_and_render_frontmatter(markdown);

        assert!(html.contains(r#"<details class="frontmatter">"#));
        assert!(html.contains(r#"<table class="frontmatter-table""#));
        assert!(html.contains("<th>title</th>"));
        assert!(html.contains("<td>Test Document</td>"));
        assert!(html.contains("<th>author</th>"));
        assert!(html.contains("<td>John Doe</td>"));
        assert!(content.starts_with("# Hello World"));
    }

    #[test]
    fn test_extract_and_render_frontmatter_with_types() {
        let markdown = indoc! {r#"
            ---
            enabled: true
            count: 42
            empty:
            ---

            Content
        "#};

        let (html, _content) = extract_and_render_frontmatter(markdown);

        assert!(html.contains(r#"<span class="yaml-bool">true</span>"#));
        assert!(html.contains(r#"<span class="yaml-number">42</span>"#));
        assert!(html.contains(r#"<span class="yaml-null">null</span>"#));
    }

    #[test]
    fn test_extract_and_render_frontmatter_with_list() {
        let markdown = indoc! {"
            ---
            tags:
              - rust
              - markdown
            ---

            Content
        "};

        let (html, _content) = extract_and_render_frontmatter(markdown);

        assert!(html.contains(r#"<ul class="yaml-list">"#));
        assert!(html.contains("<li>rust</li>"));
        assert!(html.contains("<li>markdown</li>"));
    }

    #[test]
    fn test_extract_and_render_frontmatter_no_frontmatter() {
        let markdown = "# Just a heading\n\nSome content";

        let (html, content) = extract_and_render_frontmatter(markdown);

        assert!(html.is_empty());
        assert_eq!(content, markdown);
    }

    #[test]
    fn test_render_to_html_with_frontmatter() {
        let markdown = indoc! {"
            ---
            title: My Document
            draft: false
            ---

            # Content Here
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let result = render_to_html(markdown, &md_path).unwrap();

        // Frontmatter should appear before the main content
        assert!(result.contains(r#"<details class="frontmatter""#));
        assert!(result.contains("<th>title</th>"));
        assert!(result.contains("<td>My Document</td>"));
        assert!(result.contains(r#"<span class="yaml-bool">false</span>"#));
        assert!(result.contains("<h1>Content Here</h1>"));

        // Frontmatter table should come before the heading
        let frontmatter_pos = result.find("frontmatter-table").unwrap();
        let heading_pos = result.find("<h1>").unwrap();
        assert!(
            frontmatter_pos < heading_pos,
            "Frontmatter should appear before content"
        );
    }

    #[test]
    fn test_generate_slug() {
        assert_eq!(generate_slug("Hello World"), "hello-world");
        assert_eq!(generate_slug("My Heading"), "my-heading");
        assert_eq!(
            generate_slug("Heading with  Multiple   Spaces"),
            "heading-with-multiple-spaces"
        );
        assert_eq!(
            generate_slug("Special: Characters! Here?"),
            "special-characters-here"
        );
        assert_eq!(generate_slug("日本語"), ""); // Non-ASCII characters are stripped
        assert_eq!(generate_slug("Code `example`"), "code-example");
        assert_eq!(generate_slug("under_score"), "under-score");
    }

    #[test]
    fn test_extract_headings_basic() {
        let markdown = indoc! {"
            # Title

            Some content

            ## Section 1

            More content

            ### Subsection 1.1

            Even more content

            ## Section 2
        "};

        let headings = extract_headings(markdown);

        assert_eq!(headings.len(), 4);
        assert_eq!(
            headings[0],
            HeadingInfo {
                level: 1,
                text: "Title".to_string(),
                id: "title".to_string()
            }
        );
        assert_eq!(
            headings[1],
            HeadingInfo {
                level: 2,
                text: "Section 1".to_string(),
                id: "section-1".to_string()
            }
        );
        assert_eq!(
            headings[2],
            HeadingInfo {
                level: 3,
                text: "Subsection 1.1".to_string(),
                id: "subsection-1-1".to_string()
            }
        );
        assert_eq!(
            headings[3],
            HeadingInfo {
                level: 2,
                text: "Section 2".to_string(),
                id: "section-2".to_string()
            }
        );
    }

    #[test]
    fn test_extract_headings_with_duplicate_slugs() {
        let markdown = indoc! {"
            # Introduction

            ## Overview

            Content

            ## Overview

            More content

            ## Overview
        "};

        let headings = extract_headings(markdown);

        assert_eq!(headings.len(), 4);
        assert_eq!(headings[0].id, "introduction");
        assert_eq!(headings[1].id, "overview");
        assert_eq!(headings[2].id, "overview-1");
        assert_eq!(headings[3].id, "overview-2");
    }

    #[test]
    fn test_extract_headings_with_frontmatter() {
        let markdown = indoc! {"
            ---
            title: Test
            ---

            # Heading After Frontmatter

            Content
        "};

        let headings = extract_headings(markdown);

        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "Heading After Frontmatter");
    }

    #[test]
    fn test_render_to_html_with_toc() {
        let markdown = indoc! {"
            # Title

            Some content

            ## Section 1

            More content
        "};

        let temp_dir = TempDir::new().unwrap();
        let md_path = temp_dir.path().join("test.md");

        let (html, headings) = render_to_html_with_toc(markdown, &md_path).unwrap();

        // Check headings were extracted
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].text, "Section 1");

        // Check IDs were added to HTML
        assert!(
            html.contains(r#"<h1 id="title">"#),
            "H1 should have id attribute"
        );
        assert!(
            html.contains(r#"<h2 id="section-1">"#),
            "H2 should have id attribute"
        );
    }
}
