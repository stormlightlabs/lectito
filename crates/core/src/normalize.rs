use kuchiki::NodeRef;
use kuchiki::traits::TendrilSink;

use super::{dom, patterns};

pub(crate) fn normalize_article(nodes: &[NodeRef], title: Option<&str>) {
    for node in nodes {
        remove_duplicate_title_heading(node, title);
        normalize_code_blocks(node);
        unwrap_heading_permalinks(node);
        remove_section_permalink_glyphs(node);
        remove_wbr(node);
        collapse_redundant_breaks(node);
        remove_empty_wrappers(node);
    }
}

fn remove_section_permalink_glyphs(root: &NodeRef) {
    for anchor in dom::select_nodes(root, "h1 a, h2 a, h3 a, h4 a, h5 a, h6 a") {
        if !dom::attr(&anchor, "href").is_some_and(|href| href.starts_with('#') || href.contains('#')) {
            continue;
        }
        let text = dom::inner_text(&anchor);
        let class = dom::attr(&anchor, "class").unwrap_or_default();
        if text.trim() == "§" || class.split_whitespace().any(|token| token == "doc-anchor") {
            anchor.detach();
        }
    }
}

fn remove_duplicate_title_heading(root: &NodeRef, title: Option<&str>) {
    let Some(title) = title else {
        return;
    };
    let Some(heading) = dom::select_nodes(root, "h1").into_iter().next() else {
        return;
    };
    if normalized_text(&dom::inner_text(&heading)) == normalized_text(title) {
        heading.detach();
    }
}

fn normalize_code_blocks(root: &NodeRef) {
    for pre in dom::select_nodes(root, "pre") {
        if !dom::select_nodes(&pre, "code").is_empty() {
            continue;
        }
        let fragment = kuchiki::parse_html().one("<html><body><pre><code></code></pre></body></html>");
        let Some(new_pre) = dom::select_nodes(&fragment, "pre").into_iter().next() else {
            continue;
        };
        let Some(code) = dom::select_nodes(&new_pre, "code").into_iter().next() else {
            continue;
        };
        while let Some(child) = pre.first_child() {
            code.append(child);
        }
        pre.insert_before(new_pre);
        pre.detach();
    }
}

fn unwrap_heading_permalinks(root: &NodeRef) {
    for heading in dom::select_nodes(root, "h1, h2, h3, h4, h5, h6") {
        let element_children: Vec<_> = heading
            .children()
            .filter(|child| child.as_element().is_some())
            .collect();
        if element_children.len() != 1 {
            continue;
        }
        let anchor = &element_children[0];
        if dom::node_name(anchor) != "a" || !dom::attr(anchor, "href").is_some_and(|href| href.starts_with('#')) {
            continue;
        }
        dom::replace_with_children(anchor);
    }
}

fn remove_wbr(root: &NodeRef) {
    for node in dom::select_nodes(root, "wbr") {
        node.detach();
    }
}

fn collapse_redundant_breaks(root: &NodeRef) {
    for parent in dom::select_nodes(root, "*") {
        let mut run = Vec::new();
        for child in parent.children() {
            if dom::node_name(&child) == "br" {
                run.push(child);
                continue;
            }
            if run.len() > 2 {
                for extra in run.drain(2..) {
                    extra.detach();
                }
            }
            run.clear();
        }
        if run.len() > 2 {
            for extra in run.drain(2..) {
                extra.detach();
            }
        }
    }
}

fn remove_empty_wrappers(root: &NodeRef) {
    for node in dom::select_nodes(root, "div, section, header, span") {
        if dom::node_id(&node) == dom::node_id(root) {
            continue;
        }
        let has_media = !dom::select_nodes(&node, "img, iframe, video, audio, object, embed, source").is_empty();
        if !has_media && dom::inner_text(&node).is_empty() {
            node.detach();
        }
    }
}

fn normalized_text(value: &str) -> String {
    patterns::normalize_spaces(value.trim()).to_lowercase()
}
