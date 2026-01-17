use crate::error::{LectitoError, Result};
use crate::siteconfig::directives::SiteConfig;
use sxd_document::parser;
use sxd_xpath::{Context, Factory, Value};

/// XPath evaluator for site config directives
pub struct XPathEvaluator {
    factory: Factory,
}

impl XPathEvaluator {
    /// Create a new XPath evaluator
    pub fn new() -> Self {
        Self { factory: Factory::new() }
    }

    /// Evaluate XPath expressions on HTML string and return the first non-empty result
    pub fn evaluate_strings_html(&self, html: &str, xpaths: &[String]) -> Result<Option<String>> {
        for xpath_str in xpaths {
            if let Ok(Some(result)) = self.evaluate_xpath_string_html(html, xpath_str)
                && !result.trim().is_empty()
            {
                return Ok(Some(result));
            }
        }

        Ok(None)
    }

    /// Evaluate XPath expression on HTML string and return string result
    pub fn evaluate_xpath_string_html(&self, html: &str, xpath: &str) -> Result<Option<String>> {
        let xpath_compiled = self
            .factory
            .build(xpath)
            .map_err(|e| LectitoError::SiteConfigError(format!("Invalid XPath '{}': {}", xpath, e)))?
            .ok_or_else(|| LectitoError::SiteConfigError(format!("Invalid XPath: {}", xpath)))?;

        let package = parser::parse(html)
            .map_err(|e| LectitoError::SiteConfigError(format!("Failed to parse HTML for XPath: {}", e)))?;

        let context = Context::new();
        match xpath_compiled.evaluate(&context, package.as_document().root())? {
            Value::String(s) => Ok(Some(s)),
            Value::Nodeset(nodeset) => {
                if let Some(node) = nodeset.iter().next() {
                    Ok(Some(node.string_value()))
                } else {
                    Ok(None)
                }
            }
            Value::Boolean(_) => Ok(None),
            Value::Number(_) => Ok(None),
        }
    }

    /// Evaluate XPath expression on HTML string and return all matching nodes
    pub fn evaluate_nodes_html(&self, html: &str, xpath: &str) -> Result<Vec<String>> {
        let xpath_compiled = self
            .factory
            .build(xpath)
            .map_err(|e| LectitoError::SiteConfigError(format!("Invalid XPath '{}': {}", xpath, e)))?
            .ok_or_else(|| LectitoError::SiteConfigError(format!("Invalid XPath: {}", xpath)))?;

        let package = parser::parse(html)
            .map_err(|e| LectitoError::SiteConfigError(format!("Failed to parse HTML for XPath: {}", e)))?;

        let context = Context::new();
        match xpath_compiled.evaluate(&context, package.as_document().root())? {
            Value::Nodeset(nodeset) => {
                let mut results = Vec::new();
                for node in nodeset.iter() {
                    results.push(node.string_value());
                }
                Ok(results)
            }
            _ => Ok(Vec::new()),
        }
    }
}

impl Default for XPathEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for SiteConfig to add XPath evaluation methods
pub trait SiteConfigXPath {
    /// Extract title using configured XPath expressions
    fn extract_title(&self, html: &str) -> Result<Option<String>>;

    /// Extract body using configured XPath expressions
    fn extract_body(&self, html: &str) -> Result<Option<String>>;

    /// Extract date using configured XPath expressions
    fn extract_date(&self, html: &str) -> Result<Option<String>>;

    /// Extract author using configured XPath expressions
    fn extract_author(&self, html: &str) -> Result<Option<String>>;

    /// Extract all strip nodes using configured XPath expressions
    fn extract_strip_nodes(&self, html: &str) -> Result<Vec<String>>;

    /// Extract nodes to strip by ID/class patterns
    fn extract_strip_id_or_class_nodes(&self, html: &str) -> Result<Vec<String>>;

    /// Extract image sources to strip by pattern matching
    fn extract_strip_image_src(&self, html: &str) -> Result<Vec<String>>;

    /// Extract attributes to strip by XPath
    fn extract_strip_attributes(&self, html: &str) -> Result<Vec<(String, String)>>;
}

impl SiteConfigXPath for SiteConfig {
    fn extract_title(&self, html: &str) -> Result<Option<String>> {
        let evaluator = XPathEvaluator::new();
        evaluator.evaluate_strings_html(html, &self.title)
    }

    fn extract_body(&self, html: &str) -> Result<Option<String>> {
        let evaluator = XPathEvaluator::new();
        evaluator.evaluate_strings_html(html, &self.body)
    }

    fn extract_date(&self, html: &str) -> Result<Option<String>> {
        let evaluator = XPathEvaluator::new();
        evaluator.evaluate_strings_html(html, &self.date)
    }

    fn extract_author(&self, html: &str) -> Result<Option<String>> {
        let evaluator = XPathEvaluator::new();
        evaluator.evaluate_strings_html(html, &self.author)
    }

    fn extract_strip_nodes(&self, html: &str) -> Result<Vec<String>> {
        let evaluator = XPathEvaluator::new();
        let mut all_nodes = Vec::new();

        for xpath in &self.strip {
            let nodes = evaluator.evaluate_nodes_html(html, xpath)?;
            all_nodes.extend(nodes);
        }

        Ok(all_nodes)
    }

    fn extract_strip_id_or_class_nodes(&self, html: &str) -> Result<Vec<String>> {
        let evaluator = XPathEvaluator::new();
        let mut all_nodes = Vec::new();

        for pattern in &self.strip_id_or_class {
            let id_xpath = format!("//*[@id='{}']", pattern);
            let nodes = evaluator.evaluate_nodes_html(html, &id_xpath)?;
            all_nodes.extend(nodes);

            let class_xpath = format!("//*[contains(@class, '{}')]", pattern);
            let nodes = evaluator.evaluate_nodes_html(html, &class_xpath)?;
            all_nodes.extend(nodes);
        }

        Ok(all_nodes)
    }

    fn extract_strip_image_src(&self, html: &str) -> Result<Vec<String>> {
        let evaluator = XPathEvaluator::new();
        let mut matching_images = Vec::new();

        let images = evaluator.evaluate_nodes_html(html, "//img/@src")?;

        for img_src in images {
            for pattern in &self.strip_image_src {
                if img_src.contains(pattern) {
                    matching_images.push(img_src.clone());
                    break;
                }
            }
        }

        Ok(matching_images)
    }

    fn extract_strip_attributes(&self, html: &str) -> Result<Vec<(String, String)>> {
        let evaluator = XPathEvaluator::new();
        let mut attributes = Vec::new();

        for xpath in &self.strip_attr {
            let nodes = evaluator.evaluate_nodes_html(html, xpath)?;
            for _node in nodes {
                if let Some((element_xpath, attr_name)) = xpath.rsplit_once("/@") {
                    attributes.push((element_xpath.to_string(), attr_name.to_string()));
                }
            }
        }

        Ok(attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xpath_evaluator_basic() {
        let html = r#"<html><body><h1 id="title">Test Title</h1><div id="content">Test Content</div></body></html>"#;
        let evaluator = XPathEvaluator::new();

        let result = evaluator.evaluate_xpath_string_html(html, "//h1").unwrap();
        assert_eq!(result, Some("Test Title".to_string()));

        let result = evaluator.evaluate_xpath_string_html(html, "//*[@id='title']").unwrap();
        assert_eq!(result, Some("Test Title".to_string()));
    }

    #[test]
    fn test_site_config_xpath_extraction() {
        let html = r#"
        <html>
            <head><title>Page Title</title></head>
            <body>
                <h1>Article Title</h1>
                <div class="author">John Doe</div>
                <time class="date">2023-01-01</time>
                <article id="content">Main content here</article>
            </body>
        </html>
        "#;

        let mut config = SiteConfig::new();
        config.title.push("//h1".to_string());
        config.author.push("//*[contains(@class, 'author')]/text()".to_string());
        config.date.push("//*[contains(@class, 'date')]/text()".to_string());
        config.body.push("//*[@id='content']".to_string());

        let title = config.extract_title(html).unwrap();
        assert_eq!(title, Some("Article Title".to_string()));

        let author = config.extract_author(html).unwrap();
        assert_eq!(author, Some("John Doe".to_string()));

        let date = config.extract_date(html).unwrap();
        assert_eq!(date, Some("2023-01-01".to_string()));

        let body = config.extract_body(html).unwrap();
        assert_eq!(body, Some("Main content here".to_string()));
    }

    #[test]
    fn test_multiple_xpath_fallback() {
        let html = r#"<html><body><h2>Fallback Title</h2></body></html>"#;

        let mut config = SiteConfig::new();
        config.title.push("//h1".to_string()); // Won't match
        config.title.push("//h2".to_string()); // Will match

        let title = config.extract_title(html).unwrap();
        assert_eq!(title, Some("Fallback Title".to_string()));
    }

    #[test]
    fn test_strip_id_or_class_extraction() {
        let html = r#"
        <html>
            <body>
                <div id="sidebar">Sidebar content</div>
                <div class="advertisement">Ad content</div>
                <div class="main">Main content</div>
            </body>
        </html>
        "#;

        let mut config = SiteConfig::new();
        config.strip_id_or_class.push("sidebar".to_string());
        config.strip_id_or_class.push("advertisement".to_string());

        let nodes = config.extract_strip_id_or_class_nodes(html).unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().any(|n| n.contains("Sidebar content")));
        assert!(nodes.iter().any(|n| n.contains("Ad content")));
    }
}
