use std::collections::HashSet;

use kuchiki::NodeRef;

use super::config::ExtractFlags;
use super::dom;
use super::patterns::TAGS_TO_SCORE;
use super::regexes::RegexPattern;

pub struct Candidate {
    pub node: NodeRef,
    pub score: f64,
}

pub fn score_candidates(document: &NodeRef, flags: ExtractFlags) -> Vec<Candidate> {
    let selector = TAGS_TO_SCORE.join(",");
    let mut nodes = dom::select_nodes(document, &selector);
    let mut seen: HashSet<_> = nodes.iter().map(dom::node_id).collect();
    for br in dom::select_nodes(document, "div > br") {
        if let Some(parent) = br.parent()
            && seen.insert(dom::node_id(&parent))
        {
            nodes.push(parent);
        }
    }
    let mut candidates = Vec::<Candidate>::new();

    for node in nodes {
        let text = dom::inner_text(&node);
        if text.chars().count() < 25 {
            continue;
        }

        let content_score = 1.0
            + RegexPattern::Comma.to_regex().find_iter(&text).count() as f64
            + ((text.chars().count() / 100).min(3) as f64);

        for (level, ancestor) in node
            .ancestors()
            .filter(|node| node.as_element().is_some())
            .take(5)
            .enumerate()
        {
            if dom::node_id(&ancestor) == dom::node_id(&node) {
                continue;
            }

            let divider = match level {
                0 | 1 => 1.0,
                2 => 2.0,
                _ => (level - 1) as f64 * 3.0,
            };
            let base = initialize_node_score(&ancestor, flags);
            let id = dom::node_id(&ancestor);
            if let Some(candidate) = candidates
                .iter_mut()
                .find(|candidate| dom::node_id(&candidate.node) == id)
            {
                candidate.score += content_score / divider;
            } else {
                candidates.push(Candidate { node: ancestor, score: base + content_score / divider });
            }
        }
    }

    candidates
}

pub fn class_weight(node: &NodeRef, flags: ExtractFlags) -> i32 {
    if !flags.weight_classes {
        return 0;
    }

    let mut weight = 0;
    if let Some(class) = dom::attr(node, "class") {
        if RegexPattern::Negative.to_regex().is_match(&class) {
            weight -= 25;
        }
        if RegexPattern::Positive.to_regex().is_match(&class) {
            weight += 25;
        }
    }
    if let Some(id) = dom::attr(node, "id") {
        if RegexPattern::Negative.to_regex().is_match(&id) {
            weight -= 25;
        }
        if RegexPattern::Positive.to_regex().is_match(&id) {
            weight += 25;
        }
    }
    weight
}

pub fn link_density(node: &NodeRef) -> f64 {
    let text_len = dom::inner_text(node).chars().count();
    if text_len == 0 {
        return 0.0;
    }

    let link_len: f64 = dom::select_nodes(node, "a")
        .into_iter()
        .map(|link| {
            let coefficient =
                if dom::attr(&link, "href").is_some_and(|href| href.starts_with('#')) { 0.3 } else { 1.0 };
            dom::inner_text(&link).chars().count() as f64 * coefficient
        })
        .sum();

    link_len / text_len as f64
}

fn initialize_node_score(node: &NodeRef, flags: ExtractFlags) -> f64 {
    let mut score = class_weight(node, flags) as f64;
    score += match dom::node_name(node).as_str() {
        "div" | "article" => 5.0,
        "pre" | "td" | "blockquote" => 3.0,
        "address" | "ol" | "ul" | "dl" | "dd" | "dt" | "li" | "form" => -3.0,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "th" => -5.0,
        _ => 0.0,
    };
    score
}
