use crate::parse::Element;
use regex::Regex;

/// Configuration for content scoring algorithm
#[derive(Debug, Clone)]
pub struct ScoreConfig {
    /// Minimum score threshold to consider content as readable
    pub min_score_threshold: f64,
    /// Maximum number of top candidates to track
    pub max_top_candidates: usize,
    /// Weight for positive class/ID patterns
    pub positive_weight: f64,
    /// Weight for negative class/ID patterns
    pub negative_weight: f64,
    /// Maximum content density score from character count
    pub max_char_density_score: f64,
    /// Maximum content density score from comma count
    pub max_comma_density_score: f64,
    /// Characters per point for content density scoring
    pub chars_per_point: usize,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 20.0,
            max_top_candidates: 5,
            positive_weight: 25.0,
            negative_weight: -25.0,
            max_char_density_score: 3.0,
            max_comma_density_score: 3.0,
            chars_per_point: 100,
        }
    }
}

/// Result of scoring an element
#[derive(Debug, Clone)]
pub struct ScoreResult {
    /// The element's tag name
    pub tag_name: String,
    /// The element's class attribute (if present)
    pub class: Option<String>,
    /// The element's id attribute (if present)
    pub id: Option<String>,
    /// Base score from tag type
    pub base_score: f64,
    /// Weight adjustment from class/ID patterns
    pub class_weight: f64,
    /// Content density score
    pub content_density: f64,
    /// Link density (0.0 to 1.0)
    pub link_density: f64,
    /// Final calculated score
    pub final_score: f64,
}

/// Calculate the base score for an element based on its tag name
///
/// Scores are assigned based on how likely a tag is to contain main content:
/// - ARTICLE: +10 (primary content container)
/// - SECTION: +8 (content section)
/// - DIV: +5 (generic container)
/// - TD, BLOCKQUOTE: +3 (content elements)
/// - PRE: 0 (code blocks are rarely main content, kept neutral)
/// - FORM: -3 (unlikely to contain main content)
/// - ADDRESS, OL, UL, DL, DD, DT, LI: -3 (list/metadata elements)
/// - H1-H6, TH, HEADER, FOOTER, NAV: -5 (header/navigation elements)
pub fn base_tag_score(element: &Element<'_>) -> f64 {
    match element.tag_name().as_str() {
        "article" => 10.0,
        "section" => 8.0,
        "div" => 5.0,
        "td" | "blockquote" => 3.0,
        "pre" => 0.0,
        "form" => -3.0,
        "address" | "ol" | "ul" | "dl" | "dd" | "dt" | "li" => -3.0,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "th" | "header" | "footer" | "nav" => -5.0,
        _ => 0.0,
    }
}

/// Positive patterns that suggest an element contains main content
const POSITIVE_PATTERNS: &str = r"(?i)(article|body|content|entry|hentry|h-entry|main|page|post|text|blog|story|tweet)";

/// Negative patterns that suggest an element does NOT contain main content
const NEGATIVE_PATTERNS: &str = r"(?i)(banner|breadcrumbs?|combx|comment|community|disqus|extra|foot|header|menu|related|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup|highlight|code|example)";

/// Calculate the class/ID weight adjustment for an element
///
/// Returns +positive_weight if the element's class or ID matches positive patterns,
/// or negative_weight if it matches negative patterns (but not positive).
pub fn class_id_weight(element: &Element<'_>, config: &ScoreConfig) -> f64 {
    let positive_regex = Regex::new(POSITIVE_PATTERNS).unwrap();
    let negative_regex = Regex::new(NEGATIVE_PATTERNS).unwrap();

    if let Some(id) = element.attr("id") {
        if positive_regex.is_match(id) {
            return config.positive_weight;
        }
        if negative_regex.is_match(id) {
            return config.negative_weight;
        }
    }

    if let Some(class) = element.attr("class") {
        for class_name in class.split_whitespace() {
            if positive_regex.is_match(class_name) {
                return config.positive_weight;
            }
            if negative_regex.is_match(class_name) {
                return config.negative_weight;
            }
        }
    }

    0.0
}

/// Calculate content density score based on text length and comma count
///
/// This gives higher scores to elements with:
/// - More text content (up to max_char_density_score)
/// - More commas (indicates prose, up to max_comma_density_score)
pub fn content_density_score(element: &Element<'_>, config: &ScoreConfig) -> f64 {
    let text = element.text();
    let char_score = ((text.chars().count() / config.chars_per_point) as f64).min(config.max_char_density_score);
    let comma_count = text.matches(',').count();
    let comma_score = (comma_count as f64).min(config.max_comma_density_score);

    char_score + comma_score
}

/// Calculate the link density of an element
///
/// Link density is the ratio of link text characters to total text characters.
/// Returns a value from 0.0 (no links) to 1.0 (all text is in links).
pub fn link_density(element: &Element<'_>) -> f64 {
    let text = element.text();
    let text_length = text.chars().count();

    if text_length == 0 {
        return 0.0;
    }

    let link_text_length = element
        .select("a")
        .unwrap_or_default()
        .iter()
        .map(|link| link.text().chars().count())
        .sum::<usize>();

    link_text_length as f64 / text_length as f64
}

/// Calculate the final score for an element
///
/// The final score combines:
/// - Base tag score
/// - Class/ID weight adjustment
/// - Content density
/// - Link density penalty (multiplies by 1 - link_density)
/// - Code detection penalty (for <pre> tags that look like code)
///
/// Link density penalty is reduced for elements with:
/// - Positive class/ID patterns (content indicators)
/// - High text content (prose vs navigation)
pub fn calculate_score(element: &Element<'_>, config: &ScoreConfig) -> ScoreResult {
    let tag_name = element.tag_name();
    let class = element.attr("class").map(|s| s.to_string());
    let id = element.attr("id").map(|s| s.to_string());

    let base_score = base_tag_score(element);
    let class_weight = class_id_weight(element, config);
    let content_density = content_density_score(element, config);
    let ld = link_density(element);
    let raw_score = base_score + class_weight + content_density;

    let text = element.text();
    let is_code = if tag_name == "pre" && text.len() > 50 {
        let comma_ratio = text.matches(',').count() as f64 / text.len() as f64;
        let space_ratio = text.matches(' ').count() as f64 / text.len() as f64;
        let special_ratio = text
            .chars()
            .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
            .count() as f64
            / text.len() as f64;

        special_ratio > 0.15 && comma_ratio < 0.01 && space_ratio < 0.15
    } else {
        false
    };

    let has_positive_pattern = class_weight > 0.0;
    let text_length = text.chars().count();
    let is_content_rich = text_length > 500;

    let link_penalty = if has_positive_pattern || is_content_rich { 1.0 - (ld * 0.5) } else { 1.0 - ld };

    let code_penalty = if is_code { -10.0 } else { 0.0 };

    let final_score = (raw_score + code_penalty) * link_penalty;

    ScoreResult { tag_name, class, id, base_score, class_weight, content_density, link_density: ld, final_score }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::Document;

    #[test]
    fn test_base_tag_score_article() {
        let html = r#"<article>Content</article>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("article").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&element), 10.0);
    }

    #[test]
    fn test_base_tag_score_section() {
        let html = r#"<section>Content</section>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("section").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&element), 8.0);
    }

    #[test]
    fn test_base_tag_score_div() {
        let html = r#"<div>Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&element), 5.0);
    }

    #[test]
    fn test_base_tag_score_positive_content_elements() {
        let html = r#"<table><tr><td>Cell</td></tr></table><pre>Code</pre><blockquote>Quote</blockquote>"#;
        let doc = Document::parse(html).unwrap();

        let pre_elem = doc.select("pre").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&pre_elem), 0.0);

        let td_elem = doc.select("td").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&td_elem), 3.0);

        let bq_elem = doc.select("blockquote").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&bq_elem), 3.0);
    }

    #[test]
    fn test_base_tag_score_negative_elements() {
        let html = r#"<form>Form</form><nav>Nav</nav><header>Header</header>"#;
        let doc = Document::parse(html).unwrap();

        let form_elem = doc.select("form").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&form_elem), -3.0);

        let nav_elem = doc.select("nav").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&nav_elem), -5.0);

        let header_elem = doc.select("header").unwrap().into_iter().next().unwrap();
        assert_eq!(base_tag_score(&header_elem), -5.0);
    }

    #[test]
    fn test_class_weight_positive() {
        let html = r#"<div class="article-content">Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(class_id_weight(&element, &config), 25.0);
    }

    #[test]
    fn test_class_weight_negative() {
        let html = r#"<div class="sidebar">Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(class_id_weight(&element, &config), -25.0);
    }

    #[test]
    fn test_class_weight_positive_id() {
        let html = r#"<div id="main-content">Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(class_id_weight(&element, &config), 25.0);
    }

    #[test]
    fn test_class_weight_no_match() {
        let html = r#"<div class="container" id="wrapper">Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(class_id_weight(&element, &config), 0.0);
    }

    #[test]
    fn test_class_weight_positive_overrides_negative() {
        let html = r#"<div id="main-article">Content</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(class_id_weight(&element, &config), 25.0);
    }

    #[test]
    fn test_content_density_short_text() {
        let html = r#"<div>Short text here.</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(content_density_score(&element, &config), 0.0);
    }

    #[test]
    fn test_content_density_long_text() {
        let html = r#"<div>This is a very long piece of text that contains more than one hundred characters in total to ensure it scores at least one point for character density.</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(content_density_score(&element, &config), 1.0);
    }

    #[test]
    fn test_content_density_with_commas() {
        let html = r#"<div>Text with commas, more commas, even more commas, and additional commas here.</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(content_density_score(&element, &config), 3.0);
    }

    #[test]
    fn test_content_density_max_char_score() {
        let long_text = "a".repeat(500);
        let html = format!(r#"<div>{}</div>"#, long_text);
        let doc = Document::parse(&html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        assert_eq!(content_density_score(&element, &config), 3.0);
    }

    #[test]
    fn test_link_density_no_links() {
        let html = r#"<div>Text content without any links.</div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        assert_eq!(link_density(&element), 0.0);
    }

    #[test]
    fn test_link_density_all_links() {
        let html = r##"<div><a href="#">Link text</a></div>"##;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        assert_eq!(link_density(&element), 1.0);
    }

    #[test]
    fn test_link_density_mixed() {
        let html = r##"<div>Some text <a href="#">link</a> more text</div>"##;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let density = link_density(&element);
        assert!(density > 0.0 && density < 1.0);
    }

    #[test]
    fn test_link_density_nested_links() {
        let html = r##"
            <div>
                <a href="#">First link</a>
                Regular text
                <a href="#">Second link</a>
            </div>
        "##;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let density = link_density(&element);
        assert!(density > 0.0 && density < 1.0);
    }

    #[test]
    fn test_calculate_score_combined() {
        let html = r##"<article class="main-content" id="post">
            This is a long piece of text that should score well, with multiple commas, to indicate prose content, and some links.
            <a href="#">Small link</a>
            More text here to increase character count, more commas, more content, this should definitely score high.
        </article>"##;

        let doc = Document::parse(html).unwrap();
        let element = doc.select("article").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();
        let result = calculate_score(&element, &config);

        assert_eq!(result.tag_name, "article");
        assert_eq!(result.class, Some("main-content".to_string()));
        assert_eq!(result.id, Some("post".to_string()));
        assert_eq!(result.base_score, 10.0);
        assert_eq!(result.class_weight, 25.0);
        assert!(result.content_density > 0.0);
        assert!(result.link_density > 0.0 && result.link_density < 0.3);
        assert!(result.final_score > 25.0);
    }

    #[test]
    fn test_calculate_score_nav_penalized() {
        let html = r##"<nav class="menu">
            <a href="#">Link 1</a>
            <a href="#">Link 2</a>
            <a href="#">Link 3</a>
        </nav>"##;

        let doc = Document::parse(html).unwrap();
        let element = doc.select("nav").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();

        let result = calculate_score(&element, &config);
        assert_eq!(result.tag_name, "nav");
        assert_eq!(result.base_score, -5.0);
        assert_eq!(result.class_weight, -25.0);
        assert!(result.link_density > 0.2);
        assert!(result.final_score < 0.0);
    }

    #[test]
    fn test_calculate_score_empty_div() {
        let html = r#"<div class="sidebar"></div>"#;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();

        let result = calculate_score(&element, &config);
        assert_eq!(result.base_score, 5.0);
        assert_eq!(result.class_weight, -25.0);
        assert_eq!(result.content_density, 0.0);
        assert_eq!(result.final_score, -20.0);
    }

    #[test]
    fn test_calculate_score_link_density_penalty() {
        let html = r##"
            <div>
                <a href="#">Link 1</a>
                <a href="#">Link 2</a>
                <a href="#">Link 3</a>
                <a href="#">Link 4</a>
                <a href="#">Link 5</a>
            </div>
        "##;
        let doc = Document::parse(html).unwrap();
        let element = doc.select("div").unwrap().into_iter().next().unwrap();
        let config = ScoreConfig::default();

        let result = calculate_score(&element, &config);
        assert!(result.link_density > 0.0);

        let raw_score = result.base_score + result.class_weight + result.content_density;
        assert!(result.final_score < raw_score);
    }
}
