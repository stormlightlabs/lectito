use std::collections::{BTreeSet, HashMap};

use kuchiki::NodeRef;

use super::{RenderContext, normalize_markdown, render_children};
use crate::regexes::RegexPattern;
use crate::{dom, patterns};

struct FootnoteDef {
    label: String,
    body: String,
}

#[derive(Default)]
pub struct FootnoteContext {
    definitions: Vec<FootnoteDef>,
}

impl FootnoteContext {
    pub fn extract(root: &NodeRef) -> Self {
        let definition_nodes: Vec<NodeRef> = dom::select_nodes(root, "[id]")
            .into_iter()
            .filter(is_definition_node)
            .collect();
        if definition_nodes.is_empty() {
            return Self::default();
        }

        let mut target_labels = HashMap::new();
        let mut definitions = Vec::new();
        let mut used_labels = BTreeSet::new();

        for node in definition_nodes {
            let Some(id) = dom::attr(&node, "id") else {
                continue;
            };
            let label = unique_label(
                label_from_id(&id).unwrap_or_else(|| (definitions.len() + 1).to_string()),
                &mut used_labels,
            );
            let body = definition_body(&node);
            if body.is_empty() {
                continue;
            }

            target_labels.insert(id, label.clone());
            definitions.push(FootnoteDef { label, body });
            node.detach();
        }

        if definitions.is_empty() {
            return Self::default();
        }

        rewrite_references(root, &target_labels);

        Self { definitions }
    }

    pub fn render_defs(&self) -> String {
        if self.definitions.is_empty() {
            return String::new();
        }

        let mut output = String::new();
        output.push_str("\n\n");
        for definition in &self.definitions {
            output.push_str(&format!("[^{}]: {}\n\n", definition.label, definition.body));
        }
        output
    }
}

fn is_definition_node(node: &NodeRef) -> bool {
    let tag = dom::node_name(node);
    if matches!(tag.as_str(), "a" | "sup") {
        return false;
    }

    let id = dom::attr(node, "id").unwrap_or_default();
    let class_id = dom::class_id_string(node).to_ascii_lowercase();
    let id_lower = id.to_ascii_lowercase();

    if matches!(
        id_lower.as_str(),
        "footnotes" | "footnote" | "endnotes" | "endnote" | "references" | "reference-list"
    ) {
        return false;
    }

    is_definition_id(&id_lower)
        || id_lower.starts_with("fn:")
        || id_lower.starts_with("fn-")
        || id_lower.starts_with("fn_")
        || id_lower.starts_with("footnote")
        || id_lower.starts_with("cite_note")
        || id_lower.starts_with("ftnt")
        || id_lower.starts_with("_ftn")
        || id_lower.starts_with("sdfootnote")
        || id_lower.starts_with("easy-footnote")
        || class_id.contains("footnote")
        || class_id.contains("citation")
        || class_id.contains("sidenote")
}

fn is_definition_id(id: &str) -> bool {
    (id.starts_with("fn") || id.starts_with("note") || id.starts_with("ref"))
        && label_from_id(id).is_some()
        && !id.starts_with("fnref")
        && !id.starts_with("refref")
}

fn definition_body(node: &NodeRef) -> String {
    let clone = node.clone();
    remove_backlinks(&clone);

    let mut body = normalize_markdown(&render_children(&clone, RenderContext { in_pre: false, list_depth: 0 }));
    body = strip_leading_marker(&body);
    patterns::normalize_spaces(body.trim()).trim().to_string()
}

fn remove_backlinks(node: &NodeRef) {
    for wrapper in dom::select_nodes(node, ".mw-cite-backlink") {
        wrapper.detach();
    }

    for anchor in dom::select_nodes(node, "a") {
        if is_backlink_anchor(&anchor) {
            anchor.detach();
        }
    }
}

fn is_backlink_anchor(node: &NodeRef) -> bool {
    let href = dom::attr(node, "href").unwrap_or_default();
    let href = href.trim_start_matches('#').to_ascii_lowercase();
    let text = dom::inner_text(node).to_ascii_lowercase();
    let class_id = dom::class_id_string(node).to_ascii_lowercase();

    href.starts_with("fnref")
        || href.starts_with("cite_ref")
        || href.starts_with("ftnt_ref")
        || href.starts_with("_ftnref")
        || class_id.contains("backlink")
        || class_id.contains("mw-cite-backlink")
        || matches!(text.trim(), "" | "↩" | "↩︎" | "↑" | "^" | "back" | "return")
}

fn strip_leading_marker(value: &str) -> String {
    let trimmed = value.trim();
    let Some(rest) = trimmed.strip_prefix('[') else {
        return trimmed.to_string();
    };
    let Some((marker, tail)) = rest.split_once(']') else {
        return trimmed.to_string();
    };
    if marker.chars().all(|ch| ch.is_ascii_digit()) {
        tail.trim_start().to_string()
    } else {
        trimmed.to_string()
    }
}

fn rewrite_references(root: &NodeRef, target_labels: &HashMap<String, String>) {
    let refs: Vec<_> = dom::select_nodes(root, "a[href]")
        .into_iter()
        .filter_map(|anchor| {
            let href = dom::attr(&anchor, "href")?;
            let target = href
                .strip_prefix('#')
                .or_else(|| href.split_once('#').map(|(_, fragment)| fragment))?;
            let label = target_labels.get(target)?;
            Some((anchor, label.clone()))
        })
        .collect();

    for (anchor, label) in refs {
        let replacement = NodeRef::new_text(format!("[^{label}]"));
        let target = reference_wrapper(&anchor).unwrap_or(anchor);
        target.insert_before(replacement);
        target.detach();
    }
}

fn reference_wrapper(anchor: &NodeRef) -> Option<NodeRef> {
    let parent = anchor.parent()?;
    let tag = dom::node_name(&parent);
    if tag != "sup" {
        return None;
    }
    if parent
        .children()
        .filter(|child| {
            child.as_element().is_some() || child.as_text().is_some_and(|text| !text.borrow().trim().is_empty())
        })
        .count()
        == 1
    {
        Some(parent)
    } else {
        None
    }
}

fn label_from_id(id: &str) -> Option<String> {
    let normalized = id
        .trim_start_matches('#')
        .trim_start_matches("fn:")
        .trim_start_matches("fn-")
        .trim_start_matches("fn_")
        .trim_start_matches("ftnt")
        .trim_start_matches("_ftn");
    if !normalized.is_empty() && normalized.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(normalized.to_string());
    }

    let captures = RegexPattern::FootnoteTrailingNumber.to_regex().captures(id)?;
    captures
        .get(1)
        .or_else(|| captures.get(2))
        .map(|matched| matched.as_str().to_string())
}

fn unique_label(label: String, used_labels: &mut BTreeSet<String>) -> String {
    if used_labels.insert(label.clone()) {
        return label;
    }

    let mut suffix = 2;
    loop {
        let candidate = format!("{label}-{suffix}");
        if used_labels.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}
