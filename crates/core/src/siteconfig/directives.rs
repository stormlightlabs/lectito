use crate::error::{LectitoError, Result};
use std::collections::HashMap;

/// Represents a single FTR directive
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    /// XPath expressions for content extraction
    Title(String),
    Body(String),
    Date(String),
    Author(String),

    /// Strip directives for removing unwanted elements
    Strip(String),
    StripIdOrClass(String),
    StripImageSrc(String),
    StripAttr(String),

    /// Behavior options
    Tidy(bool),
    Prune(bool),
    AutodetectOnFailure(bool),

    /// Pagination
    SinglePageLink(String),
    NextPageLink(String),

    /// Text replacement
    FindString(String),
    ReplaceString(String),

    /// HTTP configuration
    HttpHeader(String, String),

    /// Testing
    TestUrl(String),

    /// Fingerprint matching (HTML fragment -> config mapping)
    Fingerprint(String, String),
}

/// Site configuration containing all directives for a domain
#[derive(Debug, Clone, Default)]
pub struct SiteConfig {
    /// Extraction directives (multiple allowed, evaluated in order)
    pub title: Vec<String>,
    pub body: Vec<String>,
    pub date: Vec<String>,
    pub author: Vec<String>,

    /// Strip directives
    pub strip: Vec<String>,
    pub strip_id_or_class: Vec<String>,
    pub strip_image_src: Vec<String>,
    pub strip_attr: Vec<String>,

    /// Behavior options
    pub tidy: Option<bool>,
    pub prune: Option<bool>,
    pub autodetect_on_failure: Option<bool>,

    /// Pagination
    pub single_page_link: Vec<String>,
    pub next_page_link: Vec<String>,

    /// Text replacement (paired)
    pub text_replacements: Vec<(String, String)>,

    /// HTTP headers
    pub http_headers: HashMap<String, String>,

    /// Test URLs
    pub test_urls: Vec<String>,

    /// Fingerprints for CMS/platform detection (HTML fragment -> hostname mapping)
    pub fingerprints: Vec<(String, String)>,
}

impl SiteConfig {
    /// Create a new empty site config
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a directive to this config
    pub fn add_directive(&mut self, directive: Directive) {
        match directive {
            Directive::Title(xpath) => self.title.push(xpath),
            Directive::Body(xpath) => self.body.push(xpath),
            Directive::Date(xpath) => self.date.push(xpath),
            Directive::Author(xpath) => self.author.push(xpath),

            Directive::Strip(xpath) => self.strip.push(xpath),
            Directive::StripIdOrClass(pattern) => self.strip_id_or_class.push(pattern),
            Directive::StripImageSrc(pattern) => self.strip_image_src.push(pattern),
            Directive::StripAttr(xpath) => self.strip_attr.push(xpath),
            Directive::Tidy(value) => self.tidy = Some(value),
            Directive::Prune(value) => self.prune = Some(value),
            Directive::AutodetectOnFailure(value) => self.autodetect_on_failure = Some(value),
            Directive::SinglePageLink(xpath) => self.single_page_link.push(xpath),
            Directive::NextPageLink(xpath) => self.next_page_link.push(xpath),
            Directive::FindString(find) => self.text_replacements.push((find, String::new())),
            Directive::ReplaceString(replace) => {
                if let Some(last) = self.text_replacements.last_mut() {
                    if last.1.is_empty() {
                        last.1 = replace;
                    } else {
                        self.text_replacements.push((String::new(), replace));
                    }
                } else {
                    self.text_replacements.push((String::new(), replace));
                }
            }

            Directive::HttpHeader(name, value) => {
                self.http_headers.insert(name, value);
            }

            Directive::TestUrl(url) => self.test_urls.push(url),

            Directive::Fingerprint(fragment, hostname) => {
                self.fingerprints.push((fragment, hostname));
            }
        }
    }

    /// Merge another config into this one
    /// Later directives do not override earlier ones, except for boolean options
    pub fn merge(&mut self, other: &SiteConfig) {
        self.title.extend(other.title.clone());
        self.body.extend(other.body.clone());
        self.date.extend(other.date.clone());
        self.author.extend(other.author.clone());

        self.strip.extend(other.strip.clone());
        self.strip_id_or_class.extend(other.strip_id_or_class.clone());
        self.strip_image_src.extend(other.strip_image_src.clone());
        self.strip_attr.extend(other.strip_attr.clone());

        if other.tidy.is_some() {
            self.tidy = other.tidy;
        }
        if other.prune.is_some() {
            self.prune = other.prune;
        }
        if other.autodetect_on_failure.is_some() {
            self.autodetect_on_failure = other.autodetect_on_failure;
        }

        self.single_page_link.extend(other.single_page_link.clone());
        self.next_page_link.extend(other.next_page_link.clone());

        self.text_replacements.extend(other.text_replacements.clone());

        for (name, value) in &other.http_headers {
            self.http_headers.insert(name.clone(), value.clone());
        }

        self.test_urls.extend(other.test_urls.clone());

        self.fingerprints.extend(other.fingerprints.clone());
    }

    /// Check if this config should stop auto-detection
    pub fn should_autodetect(&self) -> bool {
        self.autodetect_on_failure.unwrap_or(true)
    }

    /// Get effective prune setting (default: true)
    pub fn should_prune(&self) -> bool {
        self.prune.unwrap_or(true)
    }

    /// Get effective tidy setting (default: false)
    pub fn should_tidy(&self) -> bool {
        self.tidy.unwrap_or(false)
    }

    /// Check if this config has any meaningful extraction directives
    pub fn has_extraction_config(&self) -> bool {
        !self.body.is_empty() || !self.title.is_empty()
    }

    /// Check if this config is effectively empty
    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
            && self.title.is_empty()
            && self.author.is_empty()
            && self.date.is_empty()
            && self.strip.is_empty()
            && self.strip_id_or_class.is_empty()
    }
}

/// Parse a directive line from FTR config format
pub fn parse_directive(line: &str) -> Result<Directive> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Err(LectitoError::SiteConfigError("Empty or comment line".to_string()));
    }

    if let Some((key, value)) = line.split_once(':') {
        let key = key.trim();
        let value = value.trim();

        match key {
            "title" => Ok(Directive::Title(value.to_string())),
            "body" => Ok(Directive::Body(value.to_string())),
            "date" => Ok(Directive::Date(value.to_string())),
            "author" => Ok(Directive::Author(value.to_string())),

            "strip" => Ok(Directive::Strip(value.to_string())),
            "strip_id_or_class" => Ok(Directive::StripIdOrClass(value.to_string())),
            "strip_image_src" => Ok(Directive::StripImageSrc(value.to_string())),
            "strip_attr" => Ok(Directive::StripAttr(value.to_string())),

            "tidy" => {
                let bool_val = parse_boolean(value)?;
                Ok(Directive::Tidy(bool_val))
            }
            "prune" => {
                let bool_val = parse_boolean(value)?;
                Ok(Directive::Prune(bool_val))
            }
            "autodetect_on_failure" => {
                let bool_val = parse_boolean(value)?;
                Ok(Directive::AutodetectOnFailure(bool_val))
            }

            "single_page_link" => Ok(Directive::SinglePageLink(value.to_string())),
            "next_page_link" => Ok(Directive::NextPageLink(value.to_string())),

            "find_string" => Ok(Directive::FindString(value.to_string())),
            "replace_string" => Ok(Directive::ReplaceString(value.to_string())),

            "test_url" => Ok(Directive::TestUrl(value.to_string())),

            "fingerprint" => {
                let (fragment, hostname) = value
                    .split_once('|')
                    .ok_or_else(|| LectitoError::SiteConfigError(format!("Invalid fingerprint format: {}", value)))?;
                Ok(Directive::Fingerprint(
                    fragment.trim().to_string(),
                    hostname.trim().to_string(),
                ))
            }

            _ => {
                if let Some(header_name) = key.strip_prefix("http_header(") {
                    if let Some(header_name) = header_name.strip_suffix(')') {
                        Ok(Directive::HttpHeader(header_name.to_string(), value.to_string()))
                    } else {
                        Err(LectitoError::SiteConfigError(format!(
                            "Invalid http_header format: {}",
                            key
                        )))
                    }
                } else if let Some((_find, replace)) = key
                    .strip_prefix("replace_string(")
                    .and_then(|s| s.strip_suffix(')'))
                    .and_then(|s| s.split_once(')'))
                {
                    Ok(Directive::ReplaceString(replace.to_string()))
                } else {
                    Err(LectitoError::SiteConfigError(format!("Unknown directive: {}", key)))
                }
            }
        }
    } else {
        Err(LectitoError::SiteConfigError(format!(
            "Invalid directive format: {}",
            line
        )))
    }
}

/// Parse a boolean value from FTR config
fn parse_boolean(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "yes" | "true" | "1" => Ok(true),
        "no" | "false" | "0" => Ok(false),
        _ => Err(LectitoError::SiteConfigError(format!(
            "Invalid boolean value: {}",
            value
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directive_title() {
        let directive = parse_directive("title: //h1[@class='title']").unwrap();
        assert_eq!(directive, Directive::Title("//h1[@class='title']".to_string()));
    }

    #[test]
    fn test_parse_directive_strip() {
        let directive = parse_directive("strip: //div[@class='sidebar']").unwrap();
        assert_eq!(directive, Directive::Strip("//div[@class='sidebar']".to_string()));
    }

    #[test]
    fn test_parse_directive_boolean() {
        let directive = parse_directive("tidy: yes").unwrap();
        assert_eq!(directive, Directive::Tidy(true));

        let directive = parse_directive("prune: no").unwrap();
        assert_eq!(directive, Directive::Prune(false));
    }

    #[test]
    fn test_parse_directive_http_header() {
        let directive = parse_directive("http_header(User-Agent): Lectito/1.0").unwrap();
        assert_eq!(
            directive,
            Directive::HttpHeader("User-Agent".to_string(), "Lectito/1.0".to_string())
        );
    }

    #[test]
    fn test_parse_directive_invalid() {
        let result = parse_directive("invalid_directive");
        assert!(result.is_err());
    }

    #[test]
    fn test_site_config_add_directive() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::Title("//h1".to_string()));
        config.add_directive(Directive::Body("//article".to_string()));

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.body.len(), 1);
        assert_eq!(config.title[0], "//h1");
        assert_eq!(config.body[0], "//article");
    }

    #[test]
    fn test_site_config_merge() {
        let mut config1 = SiteConfig::new();
        config1.add_directive(Directive::Title("//h1".to_string()));
        config1.add_directive(Directive::Tidy(true));

        let mut config2 = SiteConfig::new();
        config2.add_directive(Directive::Body("//article".to_string()));
        config2.add_directive(Directive::Tidy(false));

        config1.merge(&config2);

        assert_eq!(config1.title.len(), 1);
        assert_eq!(config1.body.len(), 1);
        assert_eq!(config1.tidy, Some(false)); // config2 takes precedence
    }

    #[test]
    fn test_text_replacement_pairing() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::FindString("<p />".to_string()));
        config.add_directive(Directive::ReplaceString("<br /><br />".to_string()));

        assert_eq!(config.text_replacements.len(), 1);
        assert_eq!(
            config.text_replacements[0],
            ("<p />".to_string(), "<br /><br />".to_string())
        );
    }

    #[test]
    fn test_parse_fingerprint_directive() {
        let directive =
            parse_directive("fingerprint: <meta name=\"generator\" content=\"WordPress\" | fingerprint.wordpress.com")
                .unwrap();
        assert_eq!(
            directive,
            Directive::Fingerprint(
                "<meta name=\"generator\" content=\"WordPress\"".to_string(),
                "fingerprint.wordpress.com".to_string()
            )
        );
    }

    #[test]
    fn test_config_fingerprints() {
        let mut config = SiteConfig::new();
        config.add_directive(Directive::Fingerprint(
            "<meta name=\"generator\" content=\"WordPress\"".to_string(),
            "fingerprint.wordpress.com".to_string(),
        ));
        config.add_directive(Directive::Fingerprint(
            "<meta content='blogger' name='generator'".to_string(),
            "fingerprint.blogger.com".to_string(),
        ));

        assert_eq!(config.fingerprints.len(), 2);
        assert_eq!(
            config.fingerprints[0].0,
            "<meta name=\"generator\" content=\"WordPress\""
        );
        assert_eq!(config.fingerprints[0].1, "fingerprint.wordpress.com");
    }

    #[test]
    fn test_site_config_has_extraction_config() {
        let config = SiteConfig::new();
        assert!(!config.has_extraction_config());

        let mut config = SiteConfig::new();
        config.add_directive(Directive::Body("//article".to_string()));
        assert!(config.has_extraction_config());

        let mut config = SiteConfig::new();
        config.add_directive(Directive::Title("//h1".to_string()));
        assert!(config.has_extraction_config());

        let mut config = SiteConfig::new();
        config.add_directive(Directive::Strip("//aside".to_string()));
        assert!(!config.has_extraction_config());
    }

    #[test]
    fn test_site_config_is_empty() {
        let config = SiteConfig::new();
        assert!(config.is_empty());

        let mut config = SiteConfig::new();
        config.add_directive(Directive::Body("//article".to_string()));
        assert!(!config.is_empty());

        let mut config = SiteConfig::new();
        config.add_directive(Directive::Strip("//aside".to_string()));
        assert!(!config.is_empty());
    }
}
