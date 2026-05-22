use std::io;

use kuchiki::NodeRef;

use super::error::{Error, Result};
use super::patterns;

pub(crate) fn serialize_node(node: &NodeRef) -> Result<String> {
    let mut bytes = Vec::new();
    node.serialize(&mut bytes)
        .map_err(|_: io::Error| Error::Serialization)?;
    String::from_utf8(bytes).map_err(|_| Error::Serialization)
}

pub(crate) fn serialize_children(node: &NodeRef) -> Result<String> {
    let mut html = String::new();
    for child in node.children() {
        html.push_str(&serialize_node(&child)?);
    }
    Ok(html)
}

pub(crate) fn text_content(nodes: &[NodeRef]) -> String {
    let mut text = String::new();
    for node in nodes {
        text.push_str(&node.text_contents());
        text.push('\n');
    }
    patterns::normalize_spaces(text.trim())
}
