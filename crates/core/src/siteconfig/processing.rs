use crate::error::{LectitoError, Result};
use crate::siteconfig::directives::SiteConfig;
use regex::Regex;

/// Text replacer for FTR find_string/replace_string directives
pub struct TextReplacer {
    replacements: Vec<(String, String)>,
}

impl TextReplacer {
    /// Create a new text replacer from a site config
    pub fn from_config(config: &SiteConfig) -> Self {
        Self { replacements: config.text_replacements.clone() }
    }

    /// Apply all text replacements to HTML content
    pub fn apply(&self, html: &str) -> String {
        let mut result = html.to_string();

        for (find, replace) in &self.replacements {
            if !find.is_empty() {
                result = result.replace(find, replace);
            }
        }

        result
    }
}

/// Strip processor for removing unwanted elements using FTR strip directives
pub struct StripProcessor {
    config: SiteConfig,
    id_regex: Regex,
    class_contains_regex: Regex,
    attribute_regex: Regex,
}

impl StripProcessor {
    /// Create a new strip processor from a site config
    pub fn from_config(config: &SiteConfig) -> Self {
        Self {
            config: config.clone(),
            id_regex: Regex::new(r#"//(\w+|\*)\[@id='([^']+)'\]"#).unwrap(),
            class_contains_regex: Regex::new(r"//(\w+|\*)\[contains\(@class, '([^']+)'\)\]").unwrap(),
            attribute_regex: Regex::new(r#"//(\w+|\*)\[@([^=]+)='([^']+)'\]"#).unwrap(),
        }
    }

    /// Apply all strip directives to HTML content using regex-based stripping
    pub fn apply(&self, html: &str) -> Result<String> {
        let mut result = html.to_string();

        let style_re = Regex::new(r"(?s)<style[^>]*>.*?</style>").unwrap();
        result = style_re.replace_all(&result, "").to_string();

        let edit_re = Regex::new(r#"(?s)<span[^>]*class="[^"]*mw-editsection[^"]*"[^>]*>.*?</span>"#).unwrap();
        result = edit_re.replace_all(&result, "").to_string();

        let edit_link_re = Regex::new(r#"(?s)<a[^>]*href="[^"]*action=edit[^"]*"[^>]*>.*?</a>"#).unwrap();
        result = edit_link_re.replace_all(&result, "").to_string();

        let ref_re = Regex::new(r#"(?s)<sup[^>]*class="[^"]*reference[^"]*"[^>]*>.*?</sup>"#).unwrap();
        result = ref_re.replace_all(&result, "").to_string();

        let cite_re = Regex::new(r#"(?s)<span[^>]*class="[^"]*cite-bracket[^"]*"[^>]*>.*?</span>"#).unwrap();
        result = cite_re.replace_all(&result, "").to_string();

        for xpath in &self.config.strip {
            result = self.strip_by_xpath(&result, xpath)?;
        }

        for pattern in &self.config.strip_id_or_class {
            result = self.strip_by_id(&result, pattern)?;
            result = self.strip_by_class(&result, pattern)?;
        }

        for pattern in &self.config.strip_image_src {
            result = self.strip_images_by_src(&result, pattern)?;
        }

        for xpath in &self.config.strip_attr {
            result = self.strip_attributes_by_xpath(&result, xpath)?;
        }

        Ok(result)
    }

    /// Strip elements matching XPath expression
    fn strip_by_xpath(&self, html: &str, xpath: &str) -> Result<String> {
        if let Some(css_selector) = self.xpath_to_css_selector(xpath) {
            self.strip_by_css_selector(html, &css_selector)
        } else if let Some(tag) = extract_tag_from_xpath(xpath) {
            self.strip_element_by_tag(html, &tag)
        } else {
            Ok(html.to_string())
        }
    }

    /// Strip elements matching CSS selector using regex
    fn strip_by_css_selector(&self, html: &str, selector: &str) -> Result<String> {
        if selector.starts_with('#') {
            let id = selector.trim_start_matches('#');
            self.strip_element_by_attribute(html, "id", id)
        } else if selector.contains('#') && !selector.contains("[class*=") {
            if let Some((_tag, id)) = selector.split_once('#') {
                self.strip_element_by_attribute(html, "id", id)
            } else {
                Ok(html.to_string())
            }
        } else if selector.contains('[') && selector.contains('=') {
            let re = Regex::new(r#"\[([^=]+)="([^"]+)"\]"#).unwrap();
            if let Some(captures) = re.captures(selector) {
                let attr = captures.get(1).unwrap().as_str();
                let value = captures.get(2).unwrap().as_str();
                self.strip_element_by_attribute(html, attr, value)
            } else {
                Ok(html.to_string())
            }
        } else {
            self.strip_element_by_tag(html, selector)
        }
    }

    /// Strip elements by ID attribute
    fn strip_element_by_attribute(&self, html: &str, attr: &str, value: &str) -> Result<String> {
        let pattern = if attr == "id" {
            format!(r#"(?s)<[^>]*id="{}"[^>]*>.*?</[^>]*>"#, regex::escape(value))
        } else if attr == "class" {
            format!(
                r#"(?s)<[^>]*class="[^"]*{}[^"]*"[^>]*>.*?</[^>]*>"#,
                regex::escape(value)
            )
        } else {
            format!(
                r#"(?s)<[^>]*{}="{}"[^>]*>.*?</[^>]*>"#,
                regex::escape(attr),
                regex::escape(value)
            )
        };

        let re = Regex::new(&pattern).map_err(|e| LectitoError::SiteConfigError(format!("Regex error: {}", e)))?;
        Ok(re.replace_all(html, "").to_string())
    }

    /// Strip elements by tag name
    fn strip_element_by_tag(&self, html: &str, tag: &str) -> Result<String> {
        let pattern = format!(r#"(?s)<{}[^>]*>.*?</{}>"#, regex::escape(tag), regex::escape(tag));
        let re = Regex::new(&pattern).map_err(|e| LectitoError::SiteConfigError(format!("Regex error: {}", e)))?;
        Ok(re.replace_all(html, "").to_string())
    }

    /// Strip elements by ID
    fn strip_by_id(&self, html: &str, id: &str) -> Result<String> {
        self.strip_element_by_attribute(html, "id", id)
    }

    /// Strip elements by class (contains)
    fn strip_by_class(&self, html: &str, class: &str) -> Result<String> {
        self.strip_element_by_attribute(html, "class", class)
    }

    /// Strip images by src pattern
    fn strip_images_by_src(&self, html: &str, pattern: &str) -> Result<String> {
        let img_pattern = format!(r#"(?s)<img[^>]*src="[^"]*{}[^"]*"[^>]*>"#, regex::escape(pattern));
        let re = Regex::new(&img_pattern).map_err(|e| LectitoError::SiteConfigError(format!("Regex error: {}", e)))?;
        Ok(re.replace_all(html, "").to_string())
    }

    /// Strip attributes by XPath
    fn strip_attributes_by_xpath(&self, html: &str, xpath: &str) -> Result<String> {
        if let Some((element_selector, attr_name)) = xpath.rsplit_once("/@") {
            if let Some(css_selector) = self.xpath_to_css_selector(element_selector) {
                self.strip_attribute_by_selector(html, &css_selector, attr_name)
            } else {
                Ok(html.to_string())
            }
        } else {
            Ok(html.to_string())
        }
    }

    /// Strip specific attribute from elements matching selector
    fn strip_attribute_by_selector(&self, html: &str, selector: &str, attr_name: &str) -> Result<String> {
        let pattern = format!(
            r#"<({}[^>]*)(\s{}="[^"]*"|{}='[^']*')"#,
            regex::escape(selector),
            regex::escape(attr_name),
            regex::escape(attr_name)
        );
        let re = Regex::new(&pattern).map_err(|e| LectitoError::SiteConfigError(format!("Regex error: {}", e)))?;
        Ok(re.replace_all(html, r#"<$1"#).to_string())
    }

    /// Convert simple XPath expressions to CSS selectors
    fn xpath_to_css_selector(&self, xpath: &str) -> Option<String> {
        let trimmed = xpath.trim();

        if !trimmed.contains('[') && !trimmed.contains('@') && !trimmed.contains('/') {
            return Some(trimmed.to_string());
        }

        if let Some(captures) = self.extract_id_selector(trimmed) {
            return Some(captures);
        }

        if let Some(captures) = self.extract_class_contains_selector(trimmed) {
            return Some(captures);
        }
        if let Some(captures) = self.extract_attribute_selector(trimmed) {
            return Some(captures);
        }

        None
    }

    /// Extract ID selector from XPath like //div[@id='content']
    fn extract_id_selector(&self, xpath: &str) -> Option<String> {
        if let Some(captures) = self.id_regex.captures(xpath) {
            let tag = captures.get(1).unwrap().as_str();
            let id = captures.get(2).unwrap().as_str();

            if tag == "*" { Some(format!("#{}", id)) } else { Some(format!("{}#{}", tag, id)) }
        } else {
            None
        }
    }

    /// Extract class contains selector from XPath like //*[contains(@class, 'sidebar')]
    fn extract_class_contains_selector(&self, xpath: &str) -> Option<String> {
        if let Some(captures) = self.class_contains_regex.captures(xpath) {
            let tag = captures.get(1).unwrap().as_str();
            let class = captures.get(2).unwrap().as_str();

            if tag == "*" {
                Some(format!("[class*='{}']", class))
            } else {
                Some(format!("{}[class*='{}']", tag, class))
            }
        } else {
            None
        }
    }

    /// Extract attribute selector from XPath like //img[@src='foo']
    fn extract_attribute_selector(&self, xpath: &str) -> Option<String> {
        if let Some(captures) = self.attribute_regex.captures(xpath) {
            let tag = captures.get(1).unwrap().as_str();
            let attr = captures.get(2).unwrap().as_str();
            let value = captures.get(3).unwrap().as_str();

            if tag == "*" {
                Some(format!("[{}='{}']", attr, value))
            } else {
                Some(format!("{}[{}='{}']", tag, attr, value))
            }
        } else {
            None
        }
    }
}

fn extract_tag_from_xpath(xpath: &str) -> Option<String> {
    let trimmed = xpath.trim();
    let path = trimmed.strip_prefix("//")?;
    let tag = path.split(['[', '/']).next()?.trim();
    if tag.is_empty() || tag == "*" { None } else { Some(tag.to_string()) }
}

/// Extension trait for SiteConfig to add text replacement and stripping methods
pub trait SiteConfigProcessing {
    /// Apply text replacements to HTML content
    fn apply_text_replacements(&self, html: &str) -> String;

    /// Apply strip directives to HTML content
    fn apply_strip_directives(&self, html: &str) -> Result<String>;

    /// Apply both text replacements and strip directives
    fn apply_all_processing(&self, html: &str) -> Result<String>;
}

impl SiteConfigProcessing for SiteConfig {
    fn apply_text_replacements(&self, html: &str) -> String {
        let replacer = TextReplacer::from_config(self);
        replacer.apply(html)
    }

    fn apply_strip_directives(&self, html: &str) -> Result<String> {
        let processor = StripProcessor::from_config(self);
        processor.apply(html)
    }

    fn apply_all_processing(&self, html: &str) -> Result<String> {
        let mut result = html.to_string();

        result = self.apply_text_replacements(&result);
        result = self.apply_strip_directives(&result)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::siteconfig::directives::{Directive, SiteConfig};

    #[test]
    fn test_text_replacement() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::FindString("<p />".to_string()));
        config.add_directive(Directive::ReplaceString("<br /><br />".to_string()));

        let html = r#"<div><p />Some content</div>"#;
        let result = config.apply_text_replacements(html);

        assert!(result.contains("<br /><br />"));
        assert!(!result.contains("<p />"));
    }

    #[test]
    fn test_strip_id_selector() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::Strip("//div[@id='sidebar']".to_string()));

        let html = r#"<div id="sidebar">Sidebar content</div><div id="main">Main content</div>"#;
        let result = config.apply_strip_directives(html).unwrap();

        assert!(!result.contains("sidebar"));
        assert!(result.contains("Main content"));
    }

    #[test]
    fn test_strip_class_contains() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::StripIdOrClass("advertisement".to_string()));

        let html = r#"<div class="advertisement">Ad content</div><div class="main">Main content</div>"#;
        let result = config.apply_strip_directives(html).unwrap();

        assert!(!result.contains("Ad content"));
        assert!(result.contains("Main content"));
    }

    #[test]
    fn test_strip_image_src() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::StripImageSrc("/ads/".to_string()));

        let html = r#"<img src="/ads/banner.jpg" /><img src="/images/logo.png" />"#;
        let result = config.apply_strip_directives(html).unwrap();

        assert!(!result.contains("/ads/banner.jpg"));
        assert!(result.contains("/images/logo.png"));
    }

    #[test]
    fn test_combined_processing() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::FindString("<p />".to_string()));
        config.add_directive(Directive::ReplaceString("<br /><br />".to_string()));
        config.add_directive(Directive::StripIdOrClass("sidebar".to_string()));

        let html = r#"<div id="sidebar"><p />Sidebar</div><div><p />Main</div>"#;
        let result = config.apply_all_processing(html).unwrap();

        assert!(!result.contains("sidebar"));
        assert!(!result.contains("<p />"));
        assert!(result.contains("<br /><br />"));
        assert!(result.contains("Main"));
    }
}
