use crate::Document;
use regex::Regex;

/// Represents all extracted metadata from a document
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub word_count: Option<usize>,
    pub reading_time_minutes: Option<f64>,
}

impl Document {
    /// Extract title with priority fallback:
    /// 1. JSON-LD `headline`
    /// 2. Open Graph `og:title`
    /// 3. Twitter `twitter:title`
    /// 4. Meta `title` / `DC.title`
    /// 5. `<title>` element
    /// 6. First `<h1>` element
    pub fn extract_title(&self) -> Option<String> {
        if let Some(json_ld) = self.extract_json_ld()
            && let Some(headline) = json_ld.get("headline")
            && let Some(value) = headline.as_str()
        {
            return Some(value.to_string());
        }

        if let Some(title) = self.get_meta_content("og:title") {
            return Some(title);
        }

        if let Some(title) = self.get_meta_content("twitter:title") {
            return Some(title);
        }

        if let Some(title) = self.get_meta_content("title") {
            return Some(title);
        }
        if let Some(title) = self.get_meta_content("DC.title") {
            return Some(title);
        }

        if let Some(title) = self.title() {
            return Some(title);
        }

        if let Ok(elements) = self.select("h1")
            && let Some(first) = elements.first()
        {
            let text = first.text();
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }

        None
    }

    /// Extract author with priority fallback:
    /// 1. JSON-LD `author.name`
    /// 2. Meta `author` / `DC.creator`
    /// 3. `[rel="author"]` link text
    /// 4. `[itemprop="author"]` content
    /// 5. Class/ID containing "author", "byline"
    pub fn extract_author(&self) -> Option<String> {
        if let Some(json_ld) = self.extract_json_ld()
            && let Some(author) = json_ld.get("author")
            && let Some(name) = Self::extract_author_from_json_ld(author)
        {
            return Some(name);
        }

        if let Some(author) = self.get_meta_content("author") {
            return Some(author);
        }
        if let Some(author) = self.get_meta_content("DC.creator") {
            return Some(author);
        }

        if let Ok(elements) = self.select("[rel=\"author\"]")
            && let Some(first) = elements.first()
        {
            let text = first.text();
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }

        if let Ok(elements) = self.select("[itemprop=\"author\"]")
            && let Some(first) = elements.first()
        {
            let text = first.text();
            let text = text.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }

        let patterns = ["author", "byline", "by-author", "writer"];
        for pattern in &patterns {
            let selector = format!("[class*=\"{}\"]", pattern);
            if let Ok(elements) = self.select(&selector) {
                for el in elements.iter().take(3) {
                    let text = el.text();
                    let text = text.trim();
                    if !text.is_empty() && text.len() < 100 {
                        return Some(text.to_string());
                    }
                }
            }

            let selector = format!("[id*=\"{}\"]", pattern);
            if let Ok(elements) = self.select(&selector) {
                for el in elements.iter().take(3) {
                    let text = el.text();
                    let text = text.trim();
                    if !text.is_empty() && text.len() < 100 {
                        return Some(text.to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract date with priority fallback:
    /// 1. JSON-LD `datePublished`
    /// 2. Meta `article:published_time`
    /// 3. `<time datetime="">` element
    /// 4. Meta `date` / `DC.date`
    pub fn extract_date(&self) -> Option<String> {
        if let Some(json_ld) = self.extract_json_ld()
            && let Some(date) = json_ld.get("datePublished")
            && let Some(value) = date.as_str()
        {
            return Some(value.to_string());
        }

        if let Some(date) = self.get_meta_content("article:published_time") {
            return Some(date);
        }

        if let Ok(elements) = self.select("time[datetime]")
            && let Some(first) = elements.first()
            && let Some(datetime) = first.attr("datetime")
        {
            return Some(datetime.to_string());
        }

        if let Some(date) = self.get_meta_content("date") {
            return Some(date);
        }

        if let Some(date) = self.get_meta_content("DC.date") {
            return Some(date);
        }

        None
    }

    /// Extract excerpt with priority fallback:
    /// 1. JSON-LD `description`
    /// 2. Open Graph `og:description`
    /// 3. Meta `description`
    /// 4. First paragraph of content
    pub fn extract_excerpt(&self) -> Option<String> {
        if let Some(json_ld) = self.extract_json_ld()
            && let Some(desc) = json_ld.get("description")
            && let Some(value) = desc.as_str()
        {
            return Some(value.to_string());
        }

        if let Some(desc) = self.get_meta_content("og:description") {
            return Some(desc);
        }

        if let Some(desc) = self.get_meta_content("description") {
            return Some(desc);
        }

        if let Ok(elements) = self.select("p") {
            for el in elements.iter().take(5) {
                let text = el.text();
                let text = text.trim();
                if text.len() > 50 {
                    let excerpt = if text.len() > 300 { format!("{}...", &text[..300]) } else { text.to_string() };
                    return Some(excerpt);
                }
            }
        }

        None
    }

    /// Extract site name with priority fallback:
    /// 1. JSON-LD `publisher.name`
    /// 2. Open Graph `og:site_name`
    /// 3. Domain from URL
    pub fn extract_site_name(&self) -> Option<String> {
        if let Some(json_ld) = self.extract_json_ld()
            && let Some(publisher) = json_ld.get("publisher")
            && let Some(publisher_obj) = publisher.as_object()
            && let Some(name) = publisher_obj.get("name")
            && let Some(value) = name.as_str()
        {
            return Some(value.to_string());
        }

        if let Some(site) = self.get_meta_content("og:site_name") {
            return Some(site);
        }

        if let Some(base_url) = &self.base_url()
            && let Some(domain) = base_url.domain()
        {
            return Some(domain.to_string());
        }

        None
    }

    /// Calculate word count from text content
    pub fn calculate_word_count(&self) -> usize {
        let text = self.text_content();
        count_words(&text)
    }

    /// Calculate reading time in minutes (assuming 200 words per minute)
    pub fn calculate_reading_time(&self) -> f64 {
        let word_count = self.calculate_word_count();
        word_count as f64 / 200.0
    }

    /// Extract all metadata at once
    pub fn extract_metadata(&self) -> Metadata {
        Metadata {
            title: self.extract_title(),
            author: self.extract_author(),
            date: self.extract_date(),
            excerpt: self.extract_excerpt(),
            site_name: self.extract_site_name(),
            word_count: Some(self.calculate_word_count()),
            reading_time_minutes: Some(self.calculate_reading_time()),
        }
    }

    /// Get meta tag content by name or property attribute
    fn get_meta_content(&self, attr: &str) -> Option<String> {
        let selector = format!("meta[name=\"{}\"]", attr);
        if let Ok(elements) = self.select(&selector)
            && let Some(el) = elements.first()
            && let Some(content) = el.attr("content")
        {
            return Some(content.to_string());
        }

        let selector = format!("meta[property=\"{}\"]", attr);
        if let Ok(elements) = self.select(&selector)
            && let Some(el) = elements.first()
            && let Some(content) = el.attr("content")
        {
            return Some(content.to_string());
        }

        None
    }

    /// Extract and parse JSON-LD from script tags
    fn extract_json_ld(&self) -> Option<serde_json::Value> {
        if let Ok(elements) = self.select("script[type=\"application/ld+json\"]") {
            for el in elements.iter() {
                let text = el.text();
                let json_str = text.trim();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Some(value);
                }
            }
        }
        None
    }

    /// Extract author name from JSON-LD author field
    /// Handles both string and object formats
    fn extract_author_from_json_ld(author: &serde_json::Value) -> Option<String> {
        if let Some(name) = author.as_str() {
            return Some(name.to_string());
        }

        if let Some(obj) = author.as_object()
            && let Some(name) = obj.get("name")
            && let Some(name_str) = name.as_str()
        {
            return Some(name_str.to_string());
        }

        if let Some(arr) = author.as_array()
            && let Some(first) = arr.first()
        {
            return Self::extract_author_from_json_ld(first);
        }

        None
    }
}

/// Count words in text, handling various whitespace and punctuation patterns
fn count_words(text: &str) -> usize {
    let word_regex = Regex::new(r"\b[\w'-]+\b").unwrap();
    word_regex.find_iter(text).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML_WITH_META: &str = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Test Page Title</title>
            <meta name="author" content="John Doe">
            <meta name="description" content="This is a test description of the page.">
            <meta property="og:title" content="OG Title">
            <meta property="og:description" content="OG Description">
            <meta property="og:site_name" content="Example Site">
            <meta property="article:published_time" content="2024-01-15T10:30:00Z">
            <script type="application/ld+json">
            {
                "@context": "https://schema.org",
                "@type": "Article",
                "headline": "JSON-LD Headline",
                "author": {
                    "@type": "Person",
                    "name": "Jane Smith"
                },
                "datePublished": "2024-01-15T10:30:00Z",
                "description": "JSON-LD Description",
                "publisher": {
                    "@type": "Organization",
                    "name": "JSON-LD Publisher"
                }
            }
            </script>
        </head>
        <body>
            <h1>Main Heading</h1>
            <p>This is the first paragraph of the content. It contains multiple words that should be counted for the word count calculation.</p>
            <p>This is a second paragraph with additional content.</p>
            <time datetime="2024-01-15T10:30:00Z">January 15, 2024</time>
        </body>
        </html>
    "#;

    const HTML_WITHOUT_META: &str = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Simple Page</title>
        </head>
        <body>
            <h1>Heading</h1>
            <p>This is a paragraph with some text content.</p>
        </body>
        </html>
    "#;

    #[test]
    fn test_extract_title_from_json_ld() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let title = doc.extract_title();
        assert_eq!(title, Some("JSON-LD Headline".to_string()));
    }

    #[test]
    fn test_extract_title_fallback() {
        let doc = Document::parse(HTML_WITHOUT_META).unwrap();
        let title = doc.extract_title();
        assert_eq!(title, Some("Simple Page".to_string()));
    }

    #[test]
    fn test_extract_author_from_json_ld() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let author = doc.extract_author();
        assert_eq!(author, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_extract_author_from_meta() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta name="author" content="John Doe">
            </head>
            <body></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let author = doc.extract_author();
        assert_eq!(author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_extract_date_from_json_ld() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let date = doc.extract_date();
        assert_eq!(date, Some("2024-01-15T10:30:00Z".to_string()));
    }

    #[test]
    fn test_extract_date_from_time_element() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <time datetime="2024-03-20T14:00:00Z">March 20, 2024</time>
            </body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let date = doc.extract_date();
        assert_eq!(date, Some("2024-03-20T14:00:00Z".to_string()));
    }

    #[test]
    fn test_extract_excerpt_from_json_ld() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let excerpt = doc.extract_excerpt();
        assert_eq!(excerpt, Some("JSON-LD Description".to_string()));
    }

    #[test]
    fn test_extract_excerpt_fallback_to_paragraph() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <p>This is a substantial paragraph that should be used as an excerpt because it contains enough text to be meaningful.</p>
            </body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let excerpt = doc.extract_excerpt();
        assert!(excerpt.is_some());
        assert!(excerpt.unwrap().contains("substantial paragraph"));
    }

    #[test]
    fn test_extract_site_name_from_json_ld() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let site = doc.extract_site_name();
        assert_eq!(site, Some("JSON-LD Publisher".to_string()));
    }

    #[test]
    fn test_extract_site_name_from_og() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta property="og:site_name" content="OG Site">
            </head>
            <body></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let site = doc.extract_site_name();
        assert_eq!(site, Some("OG Site".to_string()));
    }

    #[test]
    fn test_calculate_word_count() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let count = doc.calculate_word_count();
        assert!(count > 20);
    }

    #[test]
    fn test_calculate_reading_time() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let reading_time = doc.calculate_reading_time();
        assert!(reading_time > 0.0);
    }

    #[test]
    fn test_extract_all_metadata() {
        let doc = Document::parse(HTML_WITH_META).unwrap();
        let metadata = doc.extract_metadata();

        assert!(metadata.title.is_some());
        assert!(metadata.author.is_some());
        assert!(metadata.date.is_some());
        assert!(metadata.excerpt.is_some());
        assert!(metadata.site_name.is_some());
        assert!(metadata.word_count.is_some());
        assert!(metadata.reading_time_minutes.is_some());

        assert_eq!(metadata.title, Some("JSON-LD Headline".to_string()));
        assert_eq!(metadata.author, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("one"), 1);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("a b c d e"), 5);
        assert_eq!(count_words("word's with-apostrophe"), 2);
    }

    #[test]
    fn test_extract_author_array_from_json_ld() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "Article",
                    "author": [
                        {
                            "@type": "Person",
                            "name": "First Author"
                        },
                        {
                            "@type": "Person",
                            "name": "Second Author"
                        }
                    ]
                }
                </script>
            </head>
            <body></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let author = doc.extract_author();
        assert_eq!(author, Some("First Author".to_string()));
    }
}
