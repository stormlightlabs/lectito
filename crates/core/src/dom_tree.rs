use crate::Result;
use crate::parse::Document;

use std::collections::HashMap;

/// Safely truncate a string to at most `max_len` bytes at a character boundary
///
/// This function ensures we never slice in the middle of a multi-byte UTF-8 character.
/// If the max_len falls inside a character, we find the previous character boundary.
fn truncate_at_char_boundary(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }

    let safe_len = s.floor_char_boundary(max_len);
    &s[..safe_len]
}

/// A node in the DOM tree representing an element
#[derive(Debug, Clone)]
pub struct DomNode {
    /// The tag name of the element
    pub tag_name: String,
    /// The HTML content of this element
    pub html: String,
    /// Parent node ID (if any)
    pub parent_id: Option<usize>,
    /// Child node IDs
    pub child_ids: Vec<usize>,
}

/// A DOM tree structure that tracks parent-child relationships
#[derive(Debug, Clone)]
pub struct DomTree {
    /// All nodes in the tree
    nodes: Vec<DomNode>,
    /// Map from element HTML signature to node ID
    html_index: HashMap<String, usize>,
}

impl DomTree {
    /// Create a new empty DOM tree
    pub fn new() -> Self {
        Self { nodes: Vec::new(), html_index: HashMap::new() }
    }

    /// Add a node to the tree
    fn add_node(&mut self, node: DomNode) -> usize {
        let node_id = self.nodes.len();
        let signature = self.create_signature(&node);
        self.html_index.insert(signature, node_id);
        self.nodes.push(node);
        node_id
    }

    /// Create a unique signature for a node
    fn create_signature(&self, node: &DomNode) -> String {
        if node.html.len() > 200 {
            let safe_truncated = truncate_at_char_boundary(&node.html, 200);
            format!("{}-{}", node.tag_name, safe_truncated)
        } else {
            format!("{}-{}", node.tag_name, node.html)
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, id: usize) -> Option<&DomNode> {
        self.nodes.get(id)
    }

    /// Get a node by its HTML signature
    pub fn find_by_html(&self, html: &str, tag_name: &str) -> Option<&DomNode> {
        let signature = if html.len() > 200 {
            let safe_truncated = truncate_at_char_boundary(html, 200);
            format!("{}-{}", tag_name, safe_truncated)
        } else {
            format!("{}-{}", tag_name, html)
        };
        self.html_index.get(&signature).and_then(|id| self.nodes.get(*id))
    }

    /// Get the parent of a node
    pub fn get_parent(&self, node_id: usize) -> Option<&DomNode> {
        let node = self.nodes.get(node_id)?;
        let parent_id = node.parent_id?;
        self.nodes.get(parent_id)
    }

    /// Get the parent of a node by HTML
    pub fn get_parent_by_html(&self, html: &str, tag_name: &str) -> Option<&DomNode> {
        let node = self.find_by_html(html, tag_name)?;
        let parent_id = node.parent_id?;
        self.nodes.get(parent_id)
    }

    /// Get the total number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a DOM tree by analyzing containment relationships
///
/// This approach identifies parent-child relationships by checking if
/// one element's HTML contains another's HTML. It's a heuristic but
/// works well for the score propagation use case.
pub fn build_dom_tree(html: &str) -> Result<DomTree> {
    let doc = Document::parse(html)?;
    let mut tree = DomTree::new();

    let candidate_tags = &["div", "article", "section", "main", "p", "td", "pre", "blockquote"];

    let mut elements: Vec<(String, String)> = Vec::new();
    for tag in candidate_tags {
        if let Ok(results) = doc.select(tag) {
            for elem in results {
                elements.push((elem.tag_name(), elem.outer_html()));
            }
        }
    }

    for (tag_name, elem_html) in &elements {
        let node =
            DomNode { tag_name: tag_name.clone(), html: elem_html.clone(), parent_id: None, child_ids: Vec::new() };
        tree.add_node(node);
    }

    for i in 0..tree.len() {
        for j in 0..tree.len() {
            if i == j {
                continue;
            }

            let child = match tree.get_node(i) {
                Some(n) => n,
                None => continue,
            };

            let potential_parent = match tree.get_node(j) {
                Some(n) => n,
                None => continue,
            };

            if potential_parent.html.contains(&child.html) && potential_parent.html != child.html {
                let parent_len = potential_parent.html.len();
                let child_len = child.html.len();

                if parent_len > child_len && parent_len < child_len * 20 {
                    if let Some(node) = tree.nodes.get_mut(i) {
                        node.parent_id = Some(j);
                    }
                    if let Some(parent) = tree.nodes.get_mut(j) {
                        parent.child_ids.push(i);
                    }
                    break;
                }
            }
        }
    }

    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_dom_tree() {
        let html = r#"
            <div class="container">
                <article class="post">
                    <p>Test paragraph</p>
                </article>
            </div>
        "#;

        let tree = build_dom_tree(html).unwrap();
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_parent_child_relationships() {
        let html = r#"
            <div class="parent">
                <p>Child paragraph</p>
            </div>
        "#;

        let tree = build_dom_tree(html).unwrap();
        assert!(!tree.is_empty());
    }
}
