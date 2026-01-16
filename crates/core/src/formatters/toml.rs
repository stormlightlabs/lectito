use crate::Result;
use crate::metadata::Metadata;

/// Convert metadata to TOML format (for --metadata-only flag)
///
/// Manual TOML serialization to avoid adding the toml crate dependency
pub fn metadata_to_toml(metadata: &Metadata) -> Result<String> {
    let mut toml = String::new();

    if let Some(title) = &metadata.title {
        toml.push_str(&format!("title = {}\n", toml_escape_string(title)));
    }

    if let Some(author) = &metadata.author {
        toml.push_str(&format!("author = {}\n", toml_escape_string(author)));
    }

    if let Some(date) = &metadata.date {
        toml.push_str(&format!("date = {}\n", toml_escape_string(date)));
    }

    if let Some(site) = &metadata.site_name {
        toml.push_str(&format!("site_name = {}\n", toml_escape_string(site)));
    }

    if let Some(excerpt) = &metadata.excerpt {
        toml.push_str(&format!("excerpt = {}\n", toml_escape_string(excerpt)));
    }

    if let Some(word_count) = metadata.word_count {
        toml.push_str(&format!("word_count = {}\n", word_count));
    }

    if let Some(reading_time) = metadata.reading_time_minutes {
        toml.push_str(&format!("reading_time_minutes = {:.1}\n", reading_time));
    }

    if let Some(language) = &metadata.language {
        toml.push_str(&format!("language = {}\n", toml_escape_string(language)));
    }

    Ok(toml)
}

/// Escape a string for TOML format
fn toml_escape_string(s: &str) -> String {
    let needs_escape = s.contains('"') || s.contains('\\') || s.contains('\n');
    if needs_escape {
        format!(
            "\"{}\"",
            s.replace('\\', "\\\\").replace('\"', "\\\"").replace('\n', "\\n")
        )
    } else {
        format!("\"{}\"", s)
    }
}

/// TOML formatter for metadata output
pub struct TomlFormatter;

impl TomlFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn format_metadata(&self, metadata: &Metadata) -> Result<String> {
        metadata_to_toml(metadata)
    }
}

impl Default for TomlFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_to_toml_basic() {
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            ..Default::default()
        };

        let toml = metadata_to_toml(&metadata).unwrap();
        assert!(toml.contains("title = \"Test Title\""));
        assert!(toml.contains("author = \"Test Author\""));
    }

    #[test]
    fn test_metadata_to_toml_with_all_fields() {
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2024-01-15".to_string()),
            site_name: Some("Test Site".to_string()),
            excerpt: Some("Test excerpt".to_string()),
            word_count: Some(500),
            reading_time_minutes: Some(2.5),
            language: Some("en".to_string()),
        };

        let toml = metadata_to_toml(&metadata).unwrap();
        assert!(toml.contains("title = \"Test Title\""));
        assert!(toml.contains("author = \"Test Author\""));
        assert!(toml.contains("date = \"2024-01-15\""));
        assert!(toml.contains("site_name = \"Test Site\""));
        assert!(toml.contains("excerpt = \"Test excerpt\""));
        assert!(toml.contains("word_count = 500"));
        assert!(toml.contains("reading_time_minutes = 2.5"));
        assert!(toml.contains("language = \"en\""));
    }

    #[test]
    fn test_toml_escape_with_quotes() {
        let escaped = toml_escape_string("My \"Title\" here");
        assert_eq!(escaped, r#""My \"Title\" here""#);
    }

    #[test]
    fn test_toml_escape_with_newlines() {
        let escaped = toml_escape_string("Line 1\nLine 2");
        assert_eq!(escaped, r#""Line 1\nLine 2""#);
    }

    #[test]
    fn test_toml_escape_with_backslashes() {
        let escaped = toml_escape_string(r#"Path\to\file"#);
        assert_eq!(escaped, r#""Path\\to\\file""#);
    }

    #[test]
    fn test_metadata_to_toml_empty() {
        let metadata = Metadata::default();
        let toml = metadata_to_toml(&metadata).unwrap();
        assert!(toml.is_empty());
    }

    #[test]
    fn test_toml_formatter() {
        let metadata = Metadata { title: Some("Test".to_string()), ..Default::default() };

        let formatter = TomlFormatter::new();
        let result = formatter.format_metadata(&metadata);

        assert!(result.is_ok());
        assert!(result.unwrap().contains("title = \"Test\""));
    }

    #[test]
    fn test_toml_formatter_default() {
        let formatter = TomlFormatter;
        let metadata = Metadata { title: Some("Default".to_string()), ..Default::default() };

        let result = formatter.format_metadata(&metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_metadata_to_toml_language_field() {
        let metadata = Metadata { language: Some("en".to_string()), ..Default::default() };

        let toml = metadata_to_toml(&metadata).unwrap();
        assert!(toml.contains("language = \"en\""));
    }
}
