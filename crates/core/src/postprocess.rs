use regex::Regex;
use url::Url;

/// Configuration for HTML post-processing cleanup
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Whether to remove empty nodes
    pub remove_empty_nodes: bool,
    /// Maximum passes for removing empty nodes
    pub max_empty_node_passes: usize,
    /// Whether to remove nodes with high link density
    pub remove_high_link_density: bool,
    /// Maximum link density threshold (0.0 to 1.0)
    pub max_link_density: f64,
    /// Whether to clean up nested DIVs with single children
    pub clean_nested_divs: bool,
    /// Whether to remove conditional comments
    pub remove_conditional_comments: bool,
    /// Whether to strip all images
    pub strip_images: bool,
    /// Whether to keep class attributes (default: false)
    pub keep_classes: bool,
    /// Custom strip patterns (class/ID regex)
    pub strip_patterns: Option<String>,
    /// Base URL for converting relative URLs
    pub base_url: Option<Url>,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            remove_empty_nodes: true,
            max_empty_node_passes: 10,
            remove_high_link_density: true,
            max_link_density: 0.5,
            clean_nested_divs: true,
            remove_conditional_comments: true,
            strip_images: false,
            keep_classes: false,
            strip_patterns: None,
            base_url: None,
        }
    }
}

/// Post-process extracted HTML by cleaning up remaining unwanted content
pub fn postprocess_html(html: &str, config: &PostProcessConfig) -> String {
    let mut processed = html.to_string();

    if config.remove_conditional_comments {
        processed = remove_conditional_comments(&processed);
    }

    if config.strip_images {
        processed = strip_images(&processed);
    }

    if !config.keep_classes {
        processed = strip_classes(&processed);
    }

    processed = remove_doc_chrome_nodes(&processed);
    processed = remove_doc_chrome_text_blocks(&processed);

    if config.remove_empty_nodes {
        processed = remove_empty_nodes(&processed, config.max_empty_node_passes);
    }

    if config.remove_high_link_density {
        processed = remove_high_link_density_nodes(&processed, config.max_link_density);
    }

    if let Some(patterns) = &config.strip_patterns {
        processed = strip_patterns(&processed, patterns);
    }

    if config.clean_nested_divs {
        processed = clean_nested_divs(&processed);
    }

    if let Some(base_url) = &config.base_url {
        processed = fix_relative_urls(&processed, base_url);
    }

    processed
}

/// Remove Internet Explorer conditional comments
///
/// IE conditional comments have the format:
/// <!--[if condition]>...<![endif]-->
/// <!--[if !IE]>...<![endif]-->
fn remove_conditional_comments(html: &str) -> String {
    let re = Regex::new(r#"(?s)<!--\[if[^\]]*\]>.*?<!\[endif\]-->|<!--<!\[if[^\]]*\]>.*?<!\[endif\]-->"#).unwrap();
    re.replace_all(html, "").to_string()
}

/// Strip all image tags from HTML
fn strip_images(html: &str) -> String {
    let re = Regex::new(r#"<img[^>]*>"#).unwrap();
    re.replace_all(html, "").to_string()
}

/// Strip all class attributes from HTML
fn strip_classes(html: &str) -> String {
    let re = Regex::new(r#"\s+class=["'][^"']*["']"#).unwrap();
    re.replace_all(html, "").to_string()
}

/// Remove empty nodes from HTML
///
/// A node is considered empty if it has no text content or only whitespace.
/// This iteratively removes empty nodes until none remain.
fn remove_empty_nodes(html: &str, max_passes: usize) -> String {
    let mut result = html.to_string();
    let tags = [
        "div", "p", "span", "section", "article", "aside", "nav", "header", "footer",
    ];

    let mut passes = 0;
    loop {
        let mut modified = false;
        let prev_result = result.clone();

        for tag in tags {
            let empty_re = Regex::new(&format!(r#"<{}(?:\s[^>]*)?>\s*(?:<br\s*/?>\s*)*</{}>"#, tag, tag)).unwrap();
            let whitespace_re = Regex::new(&format!(r#"<{}(?:\s[^>]*)?>\s*</{}>"#, tag, tag)).unwrap();

            result = empty_re.replace_all(&result, "").to_string();
            result = whitespace_re.replace_all(&result, "").to_string();
        }

        if result != prev_result {
            modified = true;
        }

        if !modified {
            break;
        }
        passes += 1;
        if passes >= max_passes {
            break;
        }
    }

    result
}

/// Remove doc-site chrome blocks like sidebars, TOCs, and breadcrumbs.
fn remove_doc_chrome_nodes(html: &str) -> String {
    let pattern = Regex::new(
        r"(?i)(toc|table[-_ ]of[-_ ]contents|on[-_ ]this[-_ ]page|breadcrumbs?|breadcrumb|sidebar|sidenav|navigation|page[-_ ]nav|pagination|pager|edit[-_ ]on[-_ ]github|edit[-_ ]this[-_ ]page)",
    )
    .unwrap();

    let tags = ["nav", "aside", "div", "section", "ul", "ol"];
    let mut result = html.to_string();

    for tag in tags {
        let class_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+class=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = class_re
            .replace_all(&result, |caps: &regex::Captures| {
                let classes = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                if classes.split_whitespace().any(|c| pattern.is_match(c)) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();

        let id_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+id=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = id_re
            .replace_all(&result, |caps: &regex::Captures| {
                let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                if pattern.is_match(id) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Remove doc-site utility blocks by text content.
fn remove_doc_chrome_text_blocks(html: &str) -> String {
    let text_pattern = Regex::new(r"(?i)(edit on github|ask about this page|copy for llm)").unwrap();
    let tags = ["div", "p", "span", "a", "li"];
    let mut result = html.to_string();

    for tag in tags {
        let element_re = Regex::new(&format!(r#"<{}[^>]*>(.*?)</{}>"#, tag, tag)).unwrap();
        result = element_re
            .replace_all(&result, |caps: &regex::Captures| {
                let inner_html = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text = strip_tags(inner_html);
                if text_pattern.is_match(text.trim()) {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Remove nodes with high link density
///
/// Link density is the ratio of link text to total text.
/// Nodes above the threshold are removed as they're likely navigation/menus.
fn remove_high_link_density_nodes(html: &str, max_density: f64) -> String {
    let density_threshold = max_density;
    let mut result = html.to_string();

    let tags = ["div", "p", "section", "article", "aside", "nav", "li"];

    for tag in tags {
        let re = Regex::new(&format!(r#"<{}(?:\s[^>]*)?>(.*?)</{}\s*>"#, tag, tag)).unwrap();

        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let inner_html = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let text_content = strip_tags(inner_html);
                let text_length = text_content.chars().count();

                if text_length == 0 {
                    return caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string();
                }

                let link_text_length = extract_link_text_length(inner_html);
                let link_density = link_text_length as f64 / text_length as f64;

                if link_density > density_threshold {
                    String::new()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Clean up nested DIVs with single children
///
/// If a DIV contains only another DIV as its direct child,
/// unwrap the outer DIV to reduce nesting.
fn clean_nested_divs(html: &str) -> String {
    let mut result = html.to_string();
    let nested_div_re = Regex::new(r#"<div\s[^>]*>\s*<div\s[^>]*>(.*?)</div\s*>\s*</div\s*>"#).unwrap();

    let mut max_iterations = 10;
    let mut modified = true;

    while modified && max_iterations > 0 {
        let prev_result = result.clone();
        result = nested_div_re.replace_all(&result, r#"<div>$1</div>"#).to_string();
        modified = result != prev_result;
        max_iterations -= 1;
    }

    result
}

/// Remove elements matching strip patterns (class/ID regex)
///
/// Removes elements whose class or id attributes match the given regex pattern,
/// preserving the inner content.
fn strip_patterns(html: &str, patterns: &str) -> String {
    let re = match Regex::new(patterns) {
        Ok(regex) => regex,
        Err(_) => return html.to_string(),
    };

    let mut result = html.to_string();
    let tags = [
        "div", "p", "span", "section", "article", "aside", "nav", "header", "footer",
    ];

    for tag in tags {
        let element_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+class=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = element_re
            .replace_all(&result, |caps: &regex::Captures| {
                let classes = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let content = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                let should_remove = classes.split_whitespace().any(|c| re.is_match(c));

                if should_remove {
                    content.to_string()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();

        let id_re = Regex::new(&format!(
            r#"<{}((?:\s[^>]*?)?\s+id=["']([^"']*)["'][^>]*)>(.*?)</{}>"#,
            tag, tag
        ))
        .unwrap();

        result = id_re
            .replace_all(&result, |caps: &regex::Captures| {
                let id = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let content = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                if re.is_match(id) {
                    content.to_string()
                } else {
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                }
            })
            .to_string();
    }

    result
}

/// Fix remaining relative URLs to absolute URLs
fn fix_relative_urls(html: &str, base_url: &Url) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![
                lol_html::element!("a", |el| {
                    if let Some(href) = el.get_attribute("href")
                        && let Ok(absolute) = base_url.join(&href)
                    {
                        el.set_attribute("href", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
                lol_html::element!("img", |el| {
                    if let Some(src) = el.get_attribute("src")
                        && let Ok(absolute) = base_url.join(&src)
                    {
                        el.set_attribute("src", absolute.as_str()).ok();
                    }
                    Ok(())
                }),
            ],
            ..Default::default()
        },
        |c: &[u8]| {
            output.push_str(&String::from_utf8_lossy(c));
        },
    );

    match rewriter.write(html.as_bytes()) {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    match rewriter.end() {
        Ok(_) => {}
        Err(_) => return html.to_string(),
    }

    if output.is_empty() { html.to_string() } else { output }
}

/// Strip HTML tags from a string, keeping only text content
fn strip_tags(html: &str) -> String {
    let re = Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(html, "").to_string()
}

/// Extract the total length of link text from HTML
fn extract_link_text_length(html: &str) -> usize {
    let link_re = Regex::new(r"<a[^>]*>(.*?)</a>").unwrap();
    link_re
        .captures_iter(html)
        .map(|cap| cap.get(1).map(|m| m.as_str()).unwrap_or(""))
        .map(|text| strip_tags(text).chars().count())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_conditional_comments() {
        let html = r#"
            <html>
                <body>
                    <!--[if IE]>
                    <div>IE specific content</div>
                    <![endif]-->
                    <div>Normal content</div>
                    <!--[if !IE]>
                    <div>Non-IE content</div>
                    <![endif]-->
                </body>
            </html>
        "#;

        let result = remove_conditional_comments(html);
        assert!(!result.contains("[if IE]"));
        assert!(!result.contains("[endif]"));
        assert!(result.contains("Normal content"));
    }

    #[test]
    fn test_strip_images() {
        let html = r#"
            <html>
                <body>
                    <p>Text before</p>
                    <img src="test.jpg" alt="Test">
                    <p>Text after</p>
                </body>
            </html>
        "#;

        let result = strip_images(html);
        assert!(!result.contains("<img"));
        assert!(result.contains("Text before"));
        assert!(result.contains("Text after"));
    }

    #[test]
    fn test_remove_empty_nodes() {
        let html = r#"
            <html>
                <body>
                    <div></div>
                    <p>Content</p>
                    <span>   </span>
                    <section>Text</section>
                </body>
            </html>
        "#;

        let result = remove_empty_nodes(html, 10);
        assert!(result.contains("Content"));
        assert!(result.contains("Text"));
    }

    #[test]
    fn test_remove_high_link_density_nodes() {
        let html = r##"
            <html>
                <body>
                    <div class="nav">
                        <a href="#">Link 1</a>
                        <a href="#">Link 2</a>
                        <a href="#">Link 3</a>
                    </div>
                    <div class="content">
                        <p>This is substantial text content with links.
                        <a href="#">Small link</a>
                        More text here to ensure low link density.</p>
                    </div>
                </body>
            </html>
        "##;

        let result = remove_high_link_density_nodes(html, 0.5);
        assert!(result.contains("content"));
    }

    #[test]
    fn test_clean_nested_divs() {
        let html = r#"<div class="outer"><div class="inner">Content</div></div>"#;
        let result = clean_nested_divs(html);
        assert!(result.contains("Content"));
        assert!(result.contains("<div>"));
    }

    #[test]
    fn test_strip_patterns() {
        let html = r#"
            <html>
                <body>
                    <div class="advertisement">Ad content</div>
                    <div class="sidebar">Sidebar</div>
                    <div class="main">Main content</div>
                </body>
            </html>
        "#;

        let result = strip_patterns(html, r"(?i)(ad|advertisement)");
        assert!(!result.contains("advertisement"));
        assert!(result.contains("Sidebar"));
        assert!(result.contains("Main content"));
    }

    #[test]
    fn test_fix_relative_urls() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"
            <html>
                <body>
                    <a href="/about">About</a>
                    <img src="image.jpg" />
                </body>
            </html>
        "#;

        let result = fix_relative_urls(html, &base);
        assert!(result.contains("href=\"https://example.com/about\""));
        assert!(result.contains("src=\"https://example.com/blog/image.jpg\""));
    }

    #[test]
    fn test_postprocess_full_pipeline() {
        let html = r#"
            <html>
                <body>
                    <!--[if IE]>
                    <div>IE content</div>
                    <![endif]-->
                    <div></div>
                    <div class="main">Content</div>
                </body>
            </html>
        "#;

        let config = PostProcessConfig::default();
        let result = postprocess_html(html, &config);

        assert!(!result.contains("[if IE]"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_strip_tags() {
        let html = r#"<p>This is <strong>bold</strong> text</p>"#;
        let result = strip_tags(html);
        assert_eq!(result, "This is bold text");
    }

    #[test]
    fn test_extract_link_text_length() {
        let html = r##"<a href="#">Link text</a> and <a href="#">Another</a>"##;
        let length = extract_link_text_length(html);
        assert_eq!(length, 16);
    }

    #[test]
    fn test_postprocess_config_default() {
        let config = PostProcessConfig::default();
        assert!(config.remove_empty_nodes);
        assert!(config.remove_high_link_density);
        assert_eq!(config.max_link_density, 0.5);
        assert!(config.clean_nested_divs);
        assert!(config.remove_conditional_comments);
        assert!(!config.strip_images);
        assert!(!config.keep_classes);
        assert!(config.strip_patterns.is_none());
    }

    #[test]
    fn test_strip_classes() {
        let html = r#"<div class="container main">Content</div><p class="text">Text</p>"#;
        let result = strip_classes(html);
        assert!(!result.contains("class="));
        assert!(result.contains("<div>Content</div>"));
        assert!(result.contains("<p>Text</p>"));
    }

    #[test]
    fn test_keep_classes_true() {
        let html = r#"<div class="container">Content</div>"#;
        let config = PostProcessConfig { keep_classes: true, ..Default::default() };
        let result = postprocess_html(html, &config);
        assert!(result.contains("class="));
    }

    #[test]
    fn test_keep_classes_false() {
        let html = r#"<div class="container">Content</div>"#;
        let config = PostProcessConfig { keep_classes: false, ..Default::default() };
        let result = postprocess_html(html, &config);
        assert!(!result.contains("class="));
    }

    #[test]
    fn test_strip_images_with_attributes() {
        let html = r#"<img src="test.jpg" alt="Test" class="image" width="100" />"#;
        let result = strip_images(html);
        assert!(!result.contains("<img"));
        assert!(!result.contains("src="));
        assert!(!result.contains("alt="));
    }

    #[test]
    fn test_remove_empty_nodes_nested() {
        let html = r#"<div><p></p><span>Content</span></div>"#;
        let result = remove_empty_nodes(html, 10);
        assert!(!result.contains("<p></p>"));
        assert!(result.contains("Content"));
    }
}
