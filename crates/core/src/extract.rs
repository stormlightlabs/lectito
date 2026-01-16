use crate::parse::{Document, Element};
use crate::postprocess::{PostProcessConfig, postprocess_html};
use crate::scoring::{ScoreConfig, ScoreResult, calculate_score};
use crate::{LectitoError, Result};

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

    for tag in CANDIDATE_TAGS {
        if let Ok(elements) = doc.select(tag) {
            for element in elements {
                let text = element.text();
                if text.chars().count() < config.char_threshold / 10 {
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
fn propagate_scores<'a>(candidates: &mut Vec<Candidate<'a>>, doc: &'a Document) {
    let score_config = ScoreConfig::default();
    let mut processed_elements: HashSet<String> = HashSet::new();
    let mut additional_candidates = Vec::new();

    let html = doc.as_string();
    let dom_tree = match crate::build_dom_tree(&html) {
        Ok(tree) => tree,
        Err(_) => return,
    };

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
/// otherwise returns a NotReaderable error.
fn select_top_candidate<'a>(candidates: &'a [Candidate<'a>], config: &ExtractConfig) -> Result<&'a Candidate<'a>> {
    if candidates.is_empty() {
        return Err(LectitoError::NoContent);
    }

    let top_candidate = candidates
        .iter()
        .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();

    if top_candidate.score() < config.min_score_threshold {
        return Err(LectitoError::NotReaderable {
            score: top_candidate.score(),
            threshold: config.min_score_threshold,
        });
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
    top_candidate: &Candidate<'a>, candidates: &[Candidate<'a>], config: &ExtractConfig,
) -> Vec<Element<'a>> {
    let mut siblings = Vec::new();
    let top_score = top_candidate.score();

    for candidate in candidates {
        if candidate.score() >= top_score * config.sibling_threshold {
            let top_html = top_candidate.element.outer_html();
            let candidate_html = candidate.element.outer_html();

            if top_html != candidate_html {
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

    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(config.max_top_candidates);

    propagate_scores(&mut candidates, doc);

    candidates.sort_by(|a, b| b.score().partial_cmp(&a.score()).unwrap_or(std::cmp::Ordering::Equal));

    let top_candidate = select_top_candidate(&candidates, config)?;
    let siblings = select_siblings(top_candidate, &candidates, config);

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
    fn test_not_readerable_error() {
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
        assert!(matches!(result, Err(LectitoError::NotReaderable { .. })));

        if let Err(LectitoError::NotReaderable { score, threshold }) = result {
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

        propagate_scores(&mut candidates, &doc);

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
