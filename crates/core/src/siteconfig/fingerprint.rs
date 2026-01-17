use crate::error::{LectitoError, Result};
use crate::siteconfig::directives::SiteConfig;
use std::path::{Path, PathBuf};

/// Fingerprint matcher for detecting CMS/platform from HTML fragments
#[derive(Debug)]
pub struct FingerprintMatcher {
    custom_dir: Option<PathBuf>,
    standard_dir: Option<PathBuf>,
}

impl FingerprintMatcher {
    /// Create a new fingerprint matcher
    pub fn new() -> Self {
        Self { custom_dir: None, standard_dir: None }
    }

    /// Create fingerprint matcher with custom and standard config directories
    pub fn with_dirs(custom_dir: Option<PathBuf>, standard_dir: Option<PathBuf>) -> Self {
        Self { custom_dir, standard_dir }
    }

    /// Match HTML content against all known fingerprints
    ///
    /// Returns the hostname of the first matching fingerprint config
    pub fn match_html(&self, html: &str) -> Option<String> {
        let fingerprints = self.collect_all_fingerprints();

        for (fragment, hostname) in &fingerprints {
            if html.contains(fragment) {
                return Some(hostname.clone());
            }
        }

        None
    }

    /// Match HTML content against fingerprints in the head section only
    ///
    /// Some fingerprints are designed to only match meta tags in the head
    pub fn match_head(&self, html: &str) -> Option<String> {
        let fingerprints = self.collect_all_fingerprints();

        let head_content = self.extract_head_content(html);
        for (fragment, hostname) in &fingerprints {
            if head_content.contains(fragment) {
                return Some(hostname.clone());
            }
        }

        None
    }

    /// Load config for a hostname matched by fingerprint
    pub fn load_config_for_fingerprint(&self, hostname: &str) -> Result<SiteConfig> {
        let config_file = self.find_fingerprint_config(hostname)?;

        let config = crate::siteconfig::parser::ConfigParser::parse_file(&config_file)?;
        Ok(config)
    }

    /// Collect all fingerprints from both custom and standard config directories
    fn collect_all_fingerprints(&self) -> Vec<(String, String)> {
        let mut fingerprints = Vec::new();

        if let Some(custom_dir) = &self.custom_dir {
            fingerprints.extend(self.load_fingerprints_from_dir(custom_dir));
        }

        if let Some(standard_dir) = &self.standard_dir {
            fingerprints.extend(self.load_fingerprints_from_dir(standard_dir));
        }

        fingerprints
    }

    /// Load fingerprints from a config directory
    fn load_fingerprints_from_dir(&self, dir: &Path) -> Vec<(String, String)> {
        let mut fingerprints = Vec::new();

        if !dir.exists() {
            return fingerprints;
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "txt")
                    && let Ok(config) = crate::siteconfig::parser::ConfigParser::parse_file(&path)
                {
                    fingerprints.extend(config.fingerprints);
                }
            }
        }

        fingerprints
    }

    /// Extract head section content from HTML
    fn extract_head_content(&self, html: &str) -> String {
        if let Some(start) = html.find("<head")
            && let Some(end) = html[start..].find("</head>")
        {
            return html[start..start + end + 7].to_string();
        }
        String::new()
    }

    /// Find fingerprint config file by hostname
    fn find_fingerprint_config(&self, hostname: &str) -> Result<PathBuf> {
        let config_name = format!("{}.txt", hostname);

        if let Some(custom_dir) = &self.custom_dir {
            let custom_path = custom_dir.join(&config_name);
            if custom_path.exists() {
                return Ok(custom_path);
            }
        }

        if let Some(standard_dir) = &self.standard_dir {
            let standard_path = standard_dir.join(&config_name);
            if standard_path.exists() {
                return Ok(standard_path);
            }
        }

        Err(LectitoError::SiteConfigError(format!(
            "Fingerprint config not found: {}",
            config_name
        )))
    }
}

impl Default for FingerprintMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_head_content() {
        let matcher = FingerprintMatcher::new();

        let html = r#"<html><head><meta name="generator" content="WordPress"></head><body>Content</body></html>"#;
        let head = matcher.extract_head_content(html);

        assert!(head.contains("<head"));
        assert!(head.contains("</head>"));
        assert!(head.contains("WordPress"));
        assert!(!head.contains("body"));
    }

    #[test]
    fn test_extract_head_content_no_head() {
        let matcher = FingerprintMatcher::new();

        let html = r#"<html><body>Content</body></html>"#;
        let head = matcher.extract_head_content(html);

        assert!(head.is_empty());
    }

    #[test]
    fn test_match_html_wordpress() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fingerprint.wordpress.com.txt");

        let config_content = r#"
# WordPress fingerprint config
fingerprint: <meta name="generator" content="WordPress | fingerprint.wordpress.com
body: //div[@class='entry-content']
"#;

        fs::write(&config_path, config_content).unwrap();

        let matcher = FingerprintMatcher::with_dirs(Some(temp_dir.path().to_path_buf()), None);

        let html = r#"<html><head><meta name="generator" content="WordPress 6.0"></head><body>Content</body></html>"#;

        let matched = matcher.match_html(html);
        assert_eq!(matched, Some("fingerprint.wordpress.com".to_string()));
    }

    #[test]
    fn test_match_html_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fingerprint.wordpress.com.txt");

        let config_content = r#"
fingerprint: <meta name="generator" content="WordPress" | fingerprint.wordpress.com
body: //div[@class='entry-content']
"#;

        fs::write(&config_path, config_content).unwrap();

        let matcher = FingerprintMatcher::with_dirs(Some(temp_dir.path().to_path_buf()), None);

        let html = r#"<html><head><meta name="generator" content="CustomCMS"></head><body>Content</body></html>"#;

        let matched = matcher.match_html(html);
        assert!(matched.is_none());
    }

    #[test]
    fn test_match_head_only() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fingerprint.blogger.com.txt");

        let config_content = r#"
fingerprint: <meta content='blogger' name='generator' | fingerprint.blogger.com
body: //div[@class='post-body']
"#;

        fs::write(&config_path, config_content).unwrap();

        let matcher = FingerprintMatcher::with_dirs(Some(temp_dir.path().to_path_buf()), None);

        let html = r#"<html><head><meta content='blogger' name='generator'></head><body><meta name="generator" content="WordPress"></body></html>"#;

        let matched = matcher.match_head(html);
        assert_eq!(matched, Some("fingerprint.blogger.com".to_string()));
    }

    #[test]
    fn test_collect_fingerprints_multiple_configs() {
        let temp_dir = TempDir::new().unwrap();

        let wp_config = temp_dir.path().join("fingerprint.wordpress.com.txt");
        fs::write(
            &wp_config,
            "fingerprint: <meta name=\"generator\" content=\"WordPress\" | fingerprint.wordpress.com\n",
        )
        .unwrap();

        let blogger_config = temp_dir.path().join("fingerprint.blogger.com.txt");
        fs::write(
            &blogger_config,
            "fingerprint: <meta content='blogger' name='generator' | fingerprint.blogger.com\n",
        )
        .unwrap();

        let matcher = FingerprintMatcher::with_dirs(Some(temp_dir.path().to_path_buf()), None);

        let fingerprints = matcher.collect_all_fingerprints();
        assert_eq!(fingerprints.len(), 2);
    }

    #[test]
    fn test_load_config_for_fingerprint() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("fingerprint.wordpress.com.txt");

        let config_content = r#"
fingerprint: <meta name="generator" content="WordPress" | fingerprint.wordpress.com
body: //div[@class='entry-content']
strip: //div[@class='sidebar']
"#;

        fs::write(&config_path, config_content).unwrap();

        let matcher = FingerprintMatcher::with_dirs(Some(temp_dir.path().to_path_buf()), None);

        let config = matcher
            .load_config_for_fingerprint("fingerprint.wordpress.com")
            .unwrap();

        assert_eq!(config.body.len(), 1);
        assert_eq!(config.strip.len(), 1);
        assert_eq!(config.body[0], "//div[@class='entry-content']");
    }
}
