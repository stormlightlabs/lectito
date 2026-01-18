use crate::dom_tree::DomTree;
use crate::parse::{Document, Element};
use crate::postprocess::{PostProcessConfig, postprocess_html};
use crate::scoring::{ScoreConfig, ScoreResult, calculate_score};
use crate::siteconfig::{SiteConfig, SiteConfigProcessing, SiteConfigXPath};
use crate::{LectitoError, Result, preprocess};

use std::collections::HashSet;

/// Configuration for content extraction
#[derive(Debug, Clone)]
pub struct ExtractConfig {
    /// Minimum score threshold for top candidate
    pub min_score_threshold: f64,
    /// Maximum number of top candidates to track
    pub max_top_candidates: usize,
    /// Minimum character threshold for content
    pub char_threshold: usize,
    /// Maximum elements to consider
    pub max_elements: usize,
    /// Sibling score threshold (multiplier of top score)
    pub sibling_threshold: f64,
    /// Post-processing configuration
    pub postprocess: PostProcessConfig,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            min_score_threshold: 10.0,
            max_top_candidates: 5,
            char_threshold: 500,
            max_elements: 1000,
            sibling_threshold: 0.2,
            postprocess: PostProcessConfig::default(),
        }
    }
}

/// A candidate element with its score
#[derive(Debug, Clone)]
pub struct Candidate<'a> {
    /// The element itself
    pub element: Element<'a>,
    /// The calculated score result
    pub score_result: ScoreResult,
}

/// The result of content extraction
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// The main content element
    pub content: String,
    /// The top candidate score
    pub top_score: f64,
    /// Number of elements extracted
    pub element_count: usize,
}

impl<'a> Candidate<'a> {
    /// Create a new candidate from an element
    fn new(element: Element<'a>, score_result: ScoreResult) -> Self {
        Self { element, score_result }
    }

    /// Get the final score of this candidate
    fn score(&self) -> f64 {
        self.score_result.final_score
    }
}

/// Tags that are considered potential content containers
const CANDIDATE_TAGS: &[&str] = &["div", "article", "section", "main", "p", "td", "pre", "blockquote"];

/// Identify all candidate elements from the document
fn identify_candidates<'a>(
    doc: &'a Document, config: &ExtractConfig, score_config: &ScoreConfig,
) -> Vec<Candidate<'a>> {
    let mut candidates = Vec::new();
    let max_elements = if config.max_elements == 0 { usize::MAX } else { config.max_elements };
    let mut scanned = 0usize;

    for tag in CANDIDATE_TAGS {
        if let Ok(elements) = doc.select(tag) {
            for element in elements {
                if scanned >= max_elements {
                    return candidates;
                }
                scanned += 1;
                let tag_name = element.tag_name();
                let text = element.text();
                if !matches!(tag_name.as_str(), "article" | "section" | "main")
                    && text.chars().count() < config.char_threshold / 10
                {
                    continue;
                }

                let score_result = calculate_score(&element, score_config);
                candidates.push(Candidate::new(element, score_result));
            }
        }
    }

    candidates
}

/// Propagate scores from candidates to their ancestors
///
/// This implements proper score propagation by traversing up the DOM tree:
/// - Parent elements get candidate_score / 2
/// - Grandparent elements get candidate_score / 3
///
/// This helps ensure that parent containers that contain high-scoring
/// content are also considered as candidates.
fn propagate_scores<'a>(candidates: &mut Vec<Candidate<'a>>, doc: &'a Document, dom_tree: &DomTree) {
    let score_config = ScoreConfig::default();
    let mut processed_elements: HashSet<String> = HashSet::new();
    let mut additional_candidates = Vec::new();

    for candidate in candidates.iter() {
        let cand_html = candidate.element.outer_html();
        let key = if cand_html.len() > 200 {
            format!("{}-{}", candidate.element.tag_name(), &cand_html[..200])
        } else {
            format!("{}-{}", candidate.element.tag_name(), cand_html)
        };
        processed_elements.insert(key);
    }

    for candidate in candidates.iter() {
        let candidate_score = candidate.score();
        let candidate_html = candidate.element.outer_html();
        let candidate_tag = candidate.element.tag_name();

        if let Some(parent_node) = dom_tree.get_parent_by_html(&candidate_html, &candidate_tag) {
            let parent_html = &parent_node.html;
            let parent_tag = &parent_node.tag_name;
            let parent_key = if parent_html.len() > 200 {
                format!("{}-{}", parent_tag, &parent_html[..200])
            } else {
                format!("{}-{}", parent_tag, parent_html)
            };

            if !processed_elements.contains(&parent_key)
                && let Ok(parent_elements) = doc.select(parent_tag)
            {
                for parent_elem in parent_elements {
                    if parent_elem.outer_html() == *parent_html {
                        let parent_score_result = calculate_score(&parent_elem, &score_config);
                        let boosted_score = parent_score_result.final_score + candidate_score / 2.0;

                        let mut boosted_result = parent_score_result.clone();
                        boosted_result.final_score = boosted_score;

                        additional_candidates.push(Candidate::new(parent_elem, boosted_result));
                        processed_elements.insert(parent_key);
                        break;
                    }
                }
            }

            if let Some(parent_id) = parent_node.parent_id
                && let Some(grandparent_node) = dom_tree.get_parent(parent_id)
            {
                let grandparent_html = &grandparent_node.html;
                let grandparent_tag = &grandparent_node.tag_name;
                let grandparent_key = if grandparent_html.len() > 200 {
                    format!("{}-{}", grandparent_tag, &grandparent_html[..200])
                } else {
                    format!("{}-{}", grandparent_tag, grandparent_html)
                };

                if !processed_elements.contains(&grandparent_key)
                    && let Ok(grandparent_elements) = doc.select(grandparent_tag)
                {
                    for grandparent_elem in grandparent_elements {
                        if grandparent_elem.outer_html() == *grandparent_html {
                            let grandparent_score_result = calculate_score(&grandparent_elem, &score_config);
                            let boosted_score = grandparent_score_result.final_score + candidate_score / 3.0;

                            let mut boosted_result = grandparent_score_result.clone();
                            boosted_result.final_score = boosted_score;

                            additional_candidates.push(Candidate::new(grandparent_elem, boosted_result));
                            processed_elements.insert(grandparent_key);
                            break;
                        }
                    }
                }
            }
        }
    }

    candidates.extend(additional_candidates);
}

/// Select the top candidate from the list
///
/// Returns the highest scoring candidate if it meets the minimum threshold,
/// otherwise returns a NotReadable error.
fn select_top_candidate<'a>(candidates: &'a [Candidate<'a>], config: &ExtractConfig) -> Result<&'a Candidate<'a>> {
    if candidates.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let top_candidate = candidates
        .iter()
        .max_by(|a, b| compare_candidates(a, b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();

    if top_candidate.score() < config.min_score_threshold {
        return Err(LectitoError::NotReadable { score: top_candidate.score(), threshold: config.min_score_threshold });
    }

    Ok(top_candidate)
}

/// Select siblings that should be included with the top candidate
///
/// Siblings are included if:
/// - They share the same parent
/// - Their score is >= top_score * sibling_threshold
/// - For P tags: link_density < 0.25 and text_length > 80 chars
fn select_siblings<'a>(
    doc: &'a Document, top_candidate: &Candidate<'a>, candidates: &[Candidate<'a>], config: &ExtractConfig,
    dom_tree: Option<&DomTree>,
) -> Vec<Element<'a>> {
    let mut siblings = Vec::new();
    let top_score = top_candidate.score();
    let dom_tree = match dom_tree {
        Some(tree) => tree,
        None => return siblings,
    };
    let top_parent_id = parent_id_for(&top_candidate.element, dom_tree);
    if top_parent_id.is_none() {
        return siblings;
    }

    for candidate in candidates {
        if candidate.score() >= top_score * config.sibling_threshold {
            let top_html = top_candidate.element.outer_html();
            let candidate_html = candidate.element.outer_html();

            if top_html != candidate_html {
                if parent_id_for(&candidate.element, dom_tree) != top_parent_id {
                    continue;
                }
                if candidate.element.tag_name() == "p" {
                    let text = candidate.element.text();
                    let text_len = text.chars().count();

                    if text_len > 80 {
                        let link_density = crate::scoring::link_density(&candidate.element);
                        if link_density < 0.25 {
                            siblings.push(candidate.element.clone());
                        }
                    }
                } else {
                    siblings.push(candidate.element.clone());
                }
            }
        }
    }

    if let Ok(headers) = doc.select("header") {
        for header in headers {
            if parent_id_for(&header, dom_tree) != top_parent_id {
                continue;
            }
            if header.outer_html() == top_candidate.element.outer_html() {
                continue;
            }
            let text_len = header.text().trim().chars().count();
            if text_len < 10 {
                continue;
            }
            if !siblings.iter().any(|s| s.outer_html() == header.outer_html()) {
                siblings.push(header);
            }
        }
    }

    siblings
}

/// Extract the main content from a document
///
/// This is the main entry point for content extraction. It:
/// 1. Identifies candidate elements
/// 2. Propagates scores to ancestors
/// 3. Selects the top candidate
/// 4. Includes relevant siblings
/// 5. Post-processes the extracted content
/// 6. Returns the cleaned content
pub fn extract_content(doc: &Document, config: &ExtractConfig) -> Result<ExtractedContent> {
    let score_config = ScoreConfig::default();

    let mut candidates = identify_candidates(doc, config, &score_config);

    let dom_tree = crate::build_dom_tree(&doc.as_string()).ok();
    if let Some(tree) = dom_tree.as_ref() {
        propagate_scores(&mut candidates, doc, tree);
    }

    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(config.max_top_candidates);

    let top_candidate = select_top_candidate(&candidates, config)?;
    let siblings = select_siblings(doc, top_candidate, &candidates, config, dom_tree.as_ref());

    let mut content = String::new();
    content.push_str(&top_candidate.element.outer_html());

    for sibling in &siblings {
        content.push('\n');
        content.push_str(&sibling.outer_html());
    }

    let content = postprocess_html(&content, &config.postprocess);

    let element_count = 1 + siblings.len();

    Ok(ExtractedContent { content, top_score: top_candidate.score(), element_count })
}

fn parent_id_for(element: &Element<'_>, dom_tree: &DomTree) -> Option<usize> {
    let html = element.outer_html();
    let tag = element.tag_name();
    dom_tree.find_by_html(&html, &tag).and_then(|node| node.parent_id)
}

fn compare_candidates<'a>(a: &Candidate<'a>, b: &Candidate<'a>) -> Option<std::cmp::Ordering> {
    let score_order = a.score().partial_cmp(&b.score())?;
    if score_order != std::cmp::Ordering::Equal {
        return Some(score_order);
    }

    let a_tag = a.element.tag_name();
    let b_tag = b.element.tag_name();
    let tag_order = candidate_priority(&a_tag).cmp(&candidate_priority(&b_tag));
    if tag_order != std::cmp::Ordering::Equal {
        return Some(tag_order);
    }

    let a_len = a.element.text().chars().count();
    let b_len = b.element.text().chars().count();
    Some(a_len.cmp(&b_len))
}

fn candidate_priority(tag_name: &str) -> u8 {
    match tag_name {
        "article" | "main" | "section" => 3,
        "div" => 2,
        _ => 1,
    }
}

/// Extract content using site configuration with fallback to heuristics
pub fn extract_content_with_config(
    doc: &Document, config: &ExtractConfig, site_config: Option<&SiteConfig>,
) -> Result<ExtractedContent> {
    if let Some(site_cfg) = site_config {
        if !site_cfg.title.is_empty() || !site_cfg.body.is_empty() {
            match extract_with_site_config(doc, site_cfg, config) {
                Ok(content) => return Ok(content),
                Err(_) => {
                    if !site_cfg.should_autodetect() {
                        return Err(LectitoError::NoContent);
                    }
                }
            }
        }

        if !site_cfg.should_autodetect() {
            return Err(LectitoError::NoContent);
        }
    }

    extract_content(doc, config)
}

/// Extract content using explicit site configuration XPath expressions
fn extract_with_site_config(
    doc: &Document, site_config: &SiteConfig, _config: &ExtractConfig,
) -> Result<ExtractedContent> {
    let html = doc.html().html();

    let body_content = 'extracted: {
        for xpath in &site_config.body {
            if xpath.contains("[@id='")
                && let Some(start) = xpath.find("[@id='")
                && let Some(end) = xpath[start + 6..].find("']")
            {
                let id = &xpath[start + 6..start + 6 + end];
                let selector_str = format!("#{}", id);
                match scraper::Selector::parse(&selector_str) {
                    Ok(_selector) => {
                        if let Ok(elements) = doc.select(&selector_str)
                            && !elements.is_empty()
                        {
                            break 'extracted elements.iter().map(|el| el.outer_html()).collect::<Vec<_>>().join("\n");
                        }
                    }
                    Err(_) => continue,
                }
            }

            if xpath.contains("[@class='")
                && let Some(start) = xpath.find("[@class='")
                && let Some(end) = xpath[start + 8..].find("']")
            {
                let class = &xpath[start + 8..start + 8 + end];
                if let Some(tag_end) = xpath[2..].find('[') {
                    let tag = &xpath[2..2 + tag_end];
                    let selector_str = format!("{}.{}", tag, class);
                    match scraper::Selector::parse(&selector_str) {
                        Ok(_selector) => {
                            if let Ok(elements) = doc.select(&selector_str)
                                && !elements.is_empty()
                            {
                                break 'extracted elements
                                    .iter()
                                    .map(|el| el.outer_html())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        if let Some(body) = site_config.extract_body(&html)? {
            body
        } else {
            return Err(LectitoError::NoContent);
        }
    };

    let body_content = site_config.apply_strip_directives(&body_content)?;

    let body_content = if let Some(base_url) = doc.base_url() {
        preprocess::convert_relative_urls(&body_content, base_url)
    } else {
        body_content
    };

    let _title = site_config.extract_title(&html)?.or_else(|| doc.title());

    let element_count = 1;
    let top_score = 100.0;

    Ok(ExtractedContent { content: body_content, element_count, top_score })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_config_default() {
        let config = ExtractConfig::default();
        assert_eq!(config.min_score_threshold, 10.0);
        assert_eq!(config.max_top_candidates, 5);
        assert_eq!(config.char_threshold, 500);
        assert_eq!(config.max_elements, 1000);
        assert_eq!(config.sibling_threshold, 0.2);
    }

    #[test]
    fn test_identify_candidates_simple_article() {
        let html = r#"
            <html>
                <body>
                    <div class="sidebar">Sidebar</div>
                    <article class="main-content">
                        <h1>Article Title</h1>
                        <p>This is a long paragraph with lots of content to ensure it meets the character threshold.
                        It continues with more text, more content, and even more text to increase the character count.
                        This should definitely qualify as a candidate with reasonable content density.</p>
                        <p>Another paragraph with substantial content. It has multiple sentences,
                        commas for density, and enough text to be considered meaningful content.
                        The scoring algorithm should recognize this as legitimate article content.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();
        let score_config = ScoreConfig::default();

        let candidates = identify_candidates(&doc, &config, &score_config);
        assert!(!candidates.is_empty());
        let has_article = candidates.iter().any(|c| c.element.tag_name() == "article");
        assert!(has_article);
    }

    #[test]
    fn test_select_top_candidate_threshold() {
        let html = r#"
            <html>
                <body>
                    <div class="sidebar" id="sidebar">
                        <p>Short sidebar text</p>
                    </div>
                    <article class="main-content" id="main">
                        <h1>Main Article Title</h1>
                        <p>This is a very long paragraph with extensive content. It contains multiple sentences,
                        commas, periods, and various punctuation marks. The purpose is to create a substantial
                        amount of text that will score well in the content density calculation. More text here,
                        more content, more sentences, more everything. This should definitely be the top candidate
                        with a score that exceeds the minimum threshold of 20 points.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);
        assert!(result.is_ok());

        let extracted = result.unwrap();
        assert!(extracted.top_score >= config.min_score_threshold);
    }

    #[test]
    fn test_not_readable_error() {
        let html = r##"
            <html>
                <body>
                    <nav class="menu">
                        <a href="#">Link 1</a>
                        <a href="#">Link 2</a>
                        <a href="#">Link 3</a>
                        <a href="#">Link 4</a>
                        <a href="#">Link 5</a>
                        <a href="#">Link 6</a>
                    </nav>
                    <div class="sidebar">
                        This is a sidebar with some links and navigation.
                        <a href="#">Nav Link</a>
                        <a href="#">Another Link</a>
                        More sidebar content here.
                    </div>
                </body>
            </html>
        "##;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);
        assert!(matches!(result, Err(LectitoError::NotReadable { .. })));

        if let Err(LectitoError::NotReadable { score, threshold }) = result {
            assert!(score < threshold);
        }
    }

    #[test]
    fn test_extract_content_with_siblings() {
        let html = r#"
            <html>
                <body>
                    <article class="content">
                        <h1>Main Article</h1>
                        <p class="lead">This is the lead paragraph with substantial content.
                        It has enough text to be considered, with commas, and meaningful content.</p>
                        <p>This is a supporting paragraph with content, text, and commas,
                        making it a good sibling candidate for extraction.</p>
                    </article>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);

        assert!(result.is_ok());
        let extracted = result.unwrap();

        assert!(!extracted.content.is_empty());
        assert!(extracted.top_score > 0.0);
    }

    #[test]
    fn test_candidate_score_propagation() {
        let html = r#"
            <html>
                <body>
                    <div class="container">
                        <article class="post">
                            <p>A long paragraph with content, text, and more content.
                            This should score reasonably well and propagate to parent containers.
                            More text to increase character count and content density.</p>
                        </article>
                    </div>
                </body>
            </html>
        "#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();
        let score_config = ScoreConfig::default();

        let mut candidates = identify_candidates(&doc, &config, &score_config);
        let initial_count = candidates.len();

        let dom_tree = crate::build_dom_tree(&doc.as_string()).unwrap();
        propagate_scores(&mut candidates, &doc, &dom_tree);

        assert!(candidates.len() >= initial_count);
    }

    #[test]
    fn test_empty_document_error() {
        let html = r#"<html><body></body></html>"#;

        let doc = Document::parse(html).unwrap();
        let config = ExtractConfig::default();

        let result = extract_content(&doc, &config);

        assert!(matches!(result, Err(LectitoError::NoContent)));
    }
}
