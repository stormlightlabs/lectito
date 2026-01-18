use regex::Regex;
use url::Url;

/// Configuration for HTML preprocessing
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    /// Whether to remove script tags
    pub remove_scripts: bool,
    /// Whether to remove style tags
    pub remove_styles: bool,
    /// Whether to remove noscript tags
    pub remove_noscript: bool,
    /// Whether to remove iframe tags
    pub remove_iframes: bool,
    /// Whether to remove svg tags
    pub remove_svg: bool,
    /// Whether to remove canvas tags
    pub remove_canvas: bool,
    /// Whether to remove unlikely candidates
    pub remove_unlikely: bool,
    /// Whether to keep positive candidates even if they match unlikely patterns
    pub keep_positive: bool,
    /// Whether to remove hidden elements
    pub remove_hidden: bool,
    /// Whether to convert relative URLs to absolute
    pub convert_urls: bool,
    /// Base URL for converting relative URLs
    pub base_url: Option<Url>,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            remove_scripts: true,
            remove_styles: true,
            remove_noscript: true,
            remove_iframes: true,
            remove_svg: true,
            remove_canvas: true,
            remove_unlikely: true,
            keep_positive: true,
            remove_hidden: true,
            convert_urls: true,
            base_url: None,
        }
    }
}

/// Preprocess HTML by removing unwanted elements and normalizing the document
pub fn preprocess_html(html: &str, config: &PreprocessConfig) -> String {
    let mut processed = html.to_string();

    if config.remove_scripts
        || config.remove_styles
        || config.remove_noscript
        || config.remove_iframes
        || config.remove_svg
        || config.remove_canvas
    {
        processed = remove_unwanted_tags(&processed, config);
    }

    processed = remove_comments(&processed);

    if config.remove_unlikely {
        processed = remove_unlikely_candidates(&processed, config.keep_positive);
    }

    if config.remove_hidden {
        processed = remove_hidden_elements(&processed);
    }

    if config.convert_urls
        && let Some(base_url) = &config.base_url
    {
        processed = convert_relative_urls(&processed, base_url);
    }

    normalize_whitespace(processed)
}

/// Remove script, style, noscript, iframe, svg, and canvas tags from HTML
fn remove_unwanted_tags(html: &str, config: &PreprocessConfig) -> String {
    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![
                if config.remove_scripts {
                    Some(lol_html::element!("script", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
                if config.remove_styles {
                    Some(lol_html::element!("style", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
                if config.remove_noscript {
                    Some(lol_html::element!("noscript", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
                if config.remove_iframes {
                    Some(lol_html::element!("iframe", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
                if config.remove_svg {
                    Some(lol_html::element!("svg", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
                if config.remove_canvas {
                    Some(lol_html::element!("canvas", |el| {
                        el.remove();
                        Ok(())
                    }))
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect(),
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

/// Remove HTML comments from the document
fn remove_comments(html: &str) -> String {
    let re = Regex::new(r"<!--.*?-->").unwrap();
    re.replace_all(html, "").to_string()
}

/// Remove elements that match unlikely candidate patterns
fn remove_unlikely_candidates(html: &str, keep_positive: bool) -> String {
    let unlikely_pattern = Regex::new(
        r"(?i)(banner|breadcrumbs?|combx|comment|community|disqus|extra|foot|header|menu|related|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup)",
    ).unwrap();

    let positive_pattern =
        Regex::new(r"(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story|tweet)").unwrap();

    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("*", |el| {
                if let Some(id) = el.get_attribute("id")
                    && unlikely_pattern.is_match(&id)
                    && (!keep_positive || !positive_pattern.is_match(&id))
                {
                    el.remove_and_keep_content();
                    return Ok(());
                }

                if let Some(class) = el.get_attribute("class") {
                    let classes: Vec<&str> = class.split_whitespace().collect();
                    for class_name in classes {
                        if unlikely_pattern.is_match(class_name)
                            && (!keep_positive || !positive_pattern.is_match(class_name))
                        {
                            el.remove_and_keep_content();
                            return Ok(());
                        }
                    }
                }

                Ok(())
            })],
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

/// Convert relative URLs to absolute URLs
pub fn convert_relative_urls(html: &str, base_url: &Url) -> String {
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
                lol_html::element!("link", |el| {
                    if let Some(href) = el.get_attribute("href")
                        && let Ok(absolute) = base_url.join(&href)
                    {
                        el.set_attribute("href", absolute.as_str()).ok();
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

/// Remove elements with display:none or visibility:hidden styles
fn remove_hidden_elements(html: &str) -> String {
    let hidden_pattern = Regex::new(r"(?i)(display\s*:\s*none|visibility\s*:\s*hidden)").unwrap();

    let mut output = String::new();
    let mut rewriter = lol_html::HtmlRewriter::new(
        lol_html::Settings {
            element_content_handlers: vec![lol_html::element!("*", |el| {
                if let Some(style) = el.get_attribute("style")
                    && hidden_pattern.is_match(&style)
                {
                    el.remove();
                    return Ok(());
                }
                Ok(())
            })],
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

/// Normalize whitespace in HTML
fn normalize_whitespace(html: String) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(&html, " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_unwanted_tags() {
        let html = r#"
            <html>
                <head><script>alert('test');</script><style>body{color:red;}</style></head>
                <body>
                    <noscript>Enable JavaScript</noscript>
                    <iframe src="https://example.com"></iframe>
                    <svg><rect width="100" height="100"/></svg>
                    <canvas id="chart"></canvas>
                    <p>Content</p>
                </body>
            </html>
        "#;

        let config = PreprocessConfig {
            remove_scripts: true,
            remove_styles: true,
            remove_noscript: true,
            remove_iframes: true,
            remove_svg: true,
            remove_canvas: true,
            ..Default::default()
        };

        let result = remove_unwanted_tags(html, &config);
        assert!(!result.contains("<script"));
        assert!(!result.contains("<style"));
        assert!(!result.contains("<noscript"));
        assert!(!result.contains("<iframe"));
        assert!(!result.contains("<svg"));
        assert!(!result.contains("<canvas"));
        assert!(result.contains("<p>Content</p>"));

        assert!(!result.contains("alert"), "Script content should be removed");
        assert!(!result.contains("color:red"), "Style content should be removed");
        assert!(
            !result.contains("Enable JavaScript"),
            "Noscript content should be removed"
        );
        assert!(!result.contains("example.com"), "Iframe src should be removed");
        assert!(!result.contains("rect"), "SVG content should be removed");
        assert!(!result.contains("chart"), "Canvas id should be removed");
    }

    #[test]
    fn test_remove_comments() {
        let html = r#"
            <html>
                <body>
                    <!-- This is a comment -->
                    <p>Visible content</p>
                    <!-- Another comment -->
                </body>
            </html>
        "#;

        let result = remove_comments(html);
        assert!(!result.contains("<!--"));
        assert!(result.contains("Visible content"));
    }

    #[test]
    fn test_remove_unlikely_candidates() {
        let html = r#"
            <html>
                <body>
                    <div id="sidebar">Sidebar content</div>
                    <div id="main-content">Main content</div>
                    <div class="banner-ad">Ad</div>
                    <div class="article">Article content</div>
                </body>
            </html>
        "#;

        let result = remove_unlikely_candidates(html, true);
        assert!(!result.contains("sidebar"));
        assert!(!result.contains("banner-ad"));
        assert!(result.contains("main-content"));
        assert!(result.contains("article"));
    }

    #[test]
    fn test_convert_relative_urls() {
        let base = Url::parse("https://example.com/blog/").unwrap();
        let html = r#"
            <html>
                <body>
                    <a href="/about">About</a>
                    <a href="post.html">Post</a>
                    <img src="image.jpg" />
                </body>
            </html>
        "#;

        let result = convert_relative_urls(html, &base);
        assert!(result.contains("href=\"https://example.com/about\""));
        assert!(result.contains("href=\"https://example.com/blog/post.html\""));
        assert!(result.contains("src=\"https://example.com/blog/image.jpg\""));
    }

    #[test]
    fn test_remove_hidden_elements() {
        let html = r#"
            <html>
                <body>
                    <div style="display:none">Hidden content</div>
                    <div style="visibility:hidden">Invisible content</div>
                    <div>Visible content</div>
                </body>
            </html>
        "#;

        let result = remove_hidden_elements(html);
        assert!(!result.contains("Hidden content"));
        assert!(!result.contains("Invisible content"));
        assert!(result.contains("Visible content"));
    }

    #[test]
    fn test_normalize_whitespace() {
        let html = "<html><body>    Multiple   spaces\t\t\n\nhere</body></html>";
        let result = normalize_whitespace(html.to_string());
        let spaces_before = html.matches(|c: char| c.is_whitespace()).count();
        let spaces_after = result.matches(' ').count();
        assert!(spaces_after < spaces_before);
    }

    #[test]
    fn test_preprocess_full_pipeline() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script>console.log('test');</script>
                <style>.hidden{display:none;}</style>
                <!-- Comment -->
            </head>
            <body>
                <div id="sidebar" class="menu">
                    <p>Sidebar</p>
                </div>
                <div id="main" class="article">
                    <a href="/post">Link</a>
                    <p style="display:none">Hidden</p>
                    <p>Content</p>
                </div>
            </body>
            </html>
        "#;

        let base = Url::parse("https://example.com").unwrap();
        let config = PreprocessConfig { base_url: Some(base), ..Default::default() };

        let result = preprocess_html(html, &config);

        assert!(!result.contains("<script"));
        assert!(!result.contains("<style"));
        assert!(!result.contains("<!--"));
        assert!(!result.contains("sidebar"));
        assert!(!result.contains("Hidden"));
        assert!(result.contains("main"));
        assert!(result.contains("href=\"https://example.com/post\""));
        assert!(result.contains("Content"));
    }
}
