//! HTML parsing and DOM manipulation.
//!
//! This module provides the [`Document`] and [`Element`] types for parsing
//! HTML and navigating the DOM tree using CSS selectors.
//!
//! # Example
//!
//! ```rust
//! use lectito_core::parse::Document;
//!
//! let html = r#"
//!     <html>
//!         <body>
//!             <h1>Title</h1>
//!             <p class="content">Paragraph</p>
//!         </body>
//!     </html>
//! "#;
//!
//! let doc = Document::parse(html).unwrap();
//! let title = doc.title();
//! let paragraphs = doc.select("p.content")?;
//! ```

use scraper::{Html, Selector};
use url::Url;

use crate::{LectitoError, PreprocessConfig, Result, preprocess};

/// Represents a parsed HTML document.
///
/// A Document wraps an HTML page and provides methods for querying elements
/// using CSS selectors, extracting metadata, and manipulating the DOM.
///
/// # Example
///
/// ```rust
/// use lectito_core::parse::Document;
///
/// let html = "<html><head><title>Test</title></head><body><p>Hello</p></body></html>";
/// let doc = Document::parse(html).unwrap();
/// assert_eq!(doc.title(), Some("Test".to_string()));
/// ```
pub struct Document {
    html: Html,
    base_url: Option<Url>,
}

impl Document {
    /// Parses HTML from a string without preprocessing.
    ///
    /// This creates a Document directly from the HTML string without any
    /// cleaning or modification. For most use cases, use `parse_with_preprocessing`
    /// instead to get better extraction results.
    ///
    /// # Arguments
    ///
    /// * `html` - The HTML content to parse
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::parse::Document;
    ///
    /// let html = "<html><body><h1>Title</h1></body></html>";
    /// let doc = Document::parse(html).unwrap();
    /// ```
    pub fn parse(html: &str) -> Result<Self> {
        let html = Html::parse_document(html);
        Ok(Self { html, base_url: None })
    }

    /// Parses HTML from a string with preprocessing.
    ///
    /// This applies HTML cleaning and normalization before parsing,
    /// which improves content extraction accuracy.
    ///
    /// # Arguments
    ///
    /// * `html` - The HTML content to parse
    /// * `base_url` - Optional base URL for resolving relative links
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::parse::Document;
    ///
    /// let html = "<html><body><article>Content</article></body></html>";
    /// let doc = Document::parse_with_preprocessing(html, None).unwrap();
    /// ```
    pub fn parse_with_preprocessing(html: &str, base_url: Option<Url>) -> Result<Self> {
        let config = PreprocessConfig { base_url: base_url.clone(), ..Default::default() };

        let cleaned = preprocess::preprocess_html(html, &config);
        let html = Html::parse_document(&cleaned);

        Ok(Self { html, base_url })
    }

    /// Gets the base URL used for preprocessing.
    ///
    /// Returns the base URL if one was provided during parsing.
    pub fn base_url(&self) -> Option<&Url> {
        self.base_url.as_ref()
    }

    /// Gets the raw HTML representation.
    ///
    /// Returns a reference to the underlying `scraper::Html` instance.
    pub fn html(&self) -> &Html {
        &self.html
    }

    /// Gets the entire HTML as a string.
    ///
    /// Returns the full HTML document as a string.
    pub fn as_string(&self) -> String {
        self.html.html()
    }

    /// Selects elements using a CSS selector.
    ///
    /// # Arguments
    ///
    /// * `selector` - A CSS selector string (e.g., "p.content", "#main", r"a\[href\]")
    ///
    /// # Errors
    ///
    /// Returns [`LectitoError::HtmlParseError`] if the selector is invalid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use lectito_core::parse::Document;
    ///
    /// let html = r#"<p class="content">First</p><p class="content">Second</p>"#;
    /// let doc = Document::parse(html).unwrap();
    /// let elements = doc.select("p.content").unwrap();
    /// assert_eq!(elements.len(), 2);
    /// ```
    pub fn select(&'_ self, selector: &str) -> Result<Vec<Element<'_>>> {
        let sel =
            Selector::parse(selector).map_err(|e| LectitoError::HtmlParseError(format!("Invalid selector: {}", e)))?;

        Ok(self.html.select(&sel).map(|el| Element { element: el }).collect())
    }

    /// Gets the title of the document.
    ///
    /// Returns the content of the `<title>` element if present.
    pub fn title(&self) -> Option<String> {
        let selector = Selector::parse("title").ok()?;
        self.html
            .select(&selector)
            .next()
            .map(|el| el.text().collect::<String>())
    }

    /// Gets all text content from the document.
    ///
    /// Returns the concatenation of all text nodes in the document,
    /// excluding script and style elements.
    pub fn text_content(&self) -> String {
        self.html.root_element().text().collect()
    }
}

/// A wrapper around scraper's ElementRef for easier DOM manipulation.
///
/// Element represents a single node in the HTML document tree and provides
/// methods for accessing its attributes, text content, and children.
///
/// # Example
///
/// ```rust
/// use lectito_core::parse::Document;
///
/// let html = r#"<a href="https://example.com">Link text</a>"#;
/// let doc = Document::parse(html).unwrap();
/// let link = &doc.select("a").unwrap()[0];
///
/// assert_eq!(link.text(), "Link text");
/// assert_eq!(link.attr("href"), Some("https://example.com"));
/// ```
#[derive(Clone, Debug)]
pub struct Element<'a> {
    element: scraper::ElementRef<'a>,
}

impl<'a> Element<'a> {
    /// Gets the inner HTML of this element.
    ///
    /// Returns the HTML content inside this element, excluding the element's own tags.
    pub fn inner_html(&self) -> String {
        self.element.inner_html()
    }

    /// Gets the outer HTML of this element.
    ///
    /// Returns the HTML content including this element's own tags.
    pub fn outer_html(&self) -> String {
        self.element.html()
    }

    /// Gets the text content of this element.
    ///
    /// Returns the concatenation of all text nodes within this element.
    pub fn text(&self) -> String {
        self.element.text().collect()
    }

    /// Gets the value of an attribute.
    ///
    /// # Arguments
    ///
    /// * `name` - The attribute name (e.g., "href", "class", "id")
    ///
    /// Returns `None` if the attribute is not present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.element.value().attr(name)
    }

    /// Gets the tag name of this element.
    ///
    /// Returns the lowercase tag name (e.g., "div", "a", "span").
    pub fn tag_name(&self) -> String {
        self.element.value().name().to_lowercase()
    }

    /// Selects child elements using a CSS selector.
    ///
    /// # Arguments
    ///
    /// * `selector` - A CSS selector string
    ///
    /// # Errors
    ///
    /// Returns [`LectitoError::HtmlParseError`] if the selector is invalid.
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
