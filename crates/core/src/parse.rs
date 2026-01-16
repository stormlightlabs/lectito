use scraper::{Html, Selector};

use crate::{LectitoError, Result};

/// Represents a parsed HTML document
pub struct Document {
    html: Html,
}

impl Document {
    /// Parse HTML from a string
    pub fn parse(html: &str) -> Result<Self> {
        let html = Html::parse_document(html);
        Ok(Self { html })
    }

    /// Get the raw HTML representation
    pub fn html(&self) -> &Html {
        &self.html
    }

    /// Get the entire HTML as a string
    pub fn as_string(&self) -> String {
        self.html.html()
    }

    /// Select elements using a CSS selector
    pub fn select(&'_ self, selector: &str) -> Result<Vec<Element<'_>>> {
        let sel =
            Selector::parse(selector).map_err(|e| LectitoError::HtmlParseError(format!("Invalid selector: {}", e)))?;

        Ok(self.html.select(&sel).map(|el| Element { element: el }).collect())
    }

    /// Get the title of the document
    pub fn title(&self) -> Option<String> {
        let selector = Selector::parse("title").ok()?;
        self.html
            .select(&selector)
            .next()
            .map(|el| el.text().collect::<String>())
    }

    /// Get all text content from the document
    pub fn text_content(&self) -> String {
        self.html.root_element().text().collect()
    }
}

/// A wrapper around scraper's ElementRef for easier DOM manipulation
pub struct Element<'a> {
    element: scraper::ElementRef<'a>,
}

impl<'a> Element<'a> {
    /// Get the inner HTML of this element
    pub fn inner_html(&self) -> String {
        self.element.inner_html()
    }

    /// Get the outer HTML of this element
    pub fn outer_html(&self) -> String {
        self.element.html()
    }

    /// Get the text content of this element
    pub fn text(&self) -> String {
        self.element.text().collect()
    }

    /// Get the value of an attribute
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.element.value().attr(name)
    }

    /// Get the tag name of this element
    pub fn tag_name(&self) -> String {
        self.element.value().name().to_lowercase()
    }

    /// Select child elements using a CSS selector
    pub fn select(&'_ self, selector: &str) -> Result<Vec<Element<'_>>> {
        let sel =
            Selector::parse(selector).map_err(|e| LectitoError::HtmlParseError(format!("Invalid selector: {}", e)))?;

        Ok(self.element.select(&sel).map(|el| Element { element: el }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Test Page</title>
        </head>
        <body>
            <h1>Heading</h1>
            <p class="content">Paragraph 1</p>
            <p class="content">Paragraph 2</p>
            <a href="https://example.com">Link</a>
        </body>
        </html>
    "#;

    #[test]
    fn test_parse_document() {
        let doc = Document::parse(SAMPLE_HTML).unwrap();
        assert_eq!(doc.title(), Some("Test Page".to_string()));
    }

    #[test]
    fn test_select_elements() {
        let doc = Document::parse(SAMPLE_HTML).unwrap();
        let elements = doc.select("p.content").unwrap();

        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].text(), "Paragraph 1");
        assert_eq!(elements[1].text(), "Paragraph 2");
    }

    #[test]
    fn test_element_attributes() {
        let doc = Document::parse(SAMPLE_HTML).unwrap();
        let elements = doc.select("a").unwrap();

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].attr("href"), Some("https://example.com"));
        assert_eq!(elements[0].text(), "Link");
    }

    #[test]
    fn test_invalid_selector() {
        let doc = Document::parse(SAMPLE_HTML).unwrap();
        let result = doc.select("[[invalid");

        assert!(matches!(result, Err(LectitoError::HtmlParseError(_))));
    }

    #[test]
    fn test_text_content() {
        let doc = Document::parse(SAMPLE_HTML).unwrap();
        let text = doc.text_content();

        assert!(text.contains("Heading"));
        assert!(text.contains("Paragraph 1"));
        assert!(text.contains("Paragraph 2"));
    }
}
