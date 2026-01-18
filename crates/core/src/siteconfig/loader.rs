use crate::error::{LectitoError, Result};
use crate::siteconfig::directives::SiteConfig;
use crate::siteconfig::fingerprint::FingerprintMatcher;
use crate::siteconfig::parser::ConfigParser;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration loader for FTR site configs
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Custom config directory path
    custom_dir: Option<PathBuf>,
    /// Standard config directory path
    standard_dir: Option<PathBuf>,
    /// Config file cache
    cache: HashMap<String, SiteConfig>,
}

impl ConfigLoader {
    /// Create a new config loader
    pub fn new() -> Self {
        Self { custom_dir: None, standard_dir: None, cache: HashMap::new() }
    }

    /// Load configuration for a URL
    pub fn load_for_url(&mut self, url: &str) -> Result<SiteConfig> {
        let domain = self.extract_domain(url)?;
        self.load_for_domain(&domain)
    }

    /// Load configuration for HTML content with fingerprint matching
    pub fn load_for_html(&mut self, html: &str) -> Result<SiteConfig> {
        let matcher = FingerprintMatcher::with_dirs(self.custom_dir.clone(), self.standard_dir.clone());

        if let Some(hostname) = matcher.match_html(html) {
            return self.load_for_fingerprint(&hostname);
        }

        Ok(SiteConfig::new())
    }

    /// Load configuration for a fingerprint hostname
    pub fn load_for_fingerprint(&mut self, hostname: &str) -> Result<SiteConfig> {
        if let Some(config) = self.cache.get(hostname) {
            return Ok(config.clone());
        }

        let mut merged_config = SiteConfig::new();
        let mut found_configs = false;

        let config_files = self.find_fingerprint_config_files(hostname)?;

        for file_path in config_files.iter().rev() {
            match ConfigParser::parse_file(file_path) {
                Ok(config) => {
                    merged_config.merge(&config);
                    found_configs = true;

                    if let Some(false) = merged_config.autodetect_on_failure {
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse config file {}: {}", file_path.display(), e);
                }
            }
        }

        self.cache.insert(hostname.to_string(), merged_config.clone());

        if found_configs { Ok(merged_config) } else { Ok(SiteConfig::new()) }
    }

    /// Load configuration for a domain
    pub fn load_for_domain(&mut self, domain: &str) -> Result<SiteConfig> {
        if let Some(config) = self.cache.get(domain) {
            return Ok(config.clone());
        }

        let mut merged_config = SiteConfig::new();
        let mut found_configs = false;

        let config_files = self.find_config_files(domain)?;

        for file_path in config_files.iter().rev() {
            match ConfigParser::parse_file(file_path) {
                Ok(config) => {
                    merged_config.merge(&config);
                    found_configs = true;

                    if let Some(false) = merged_config.autodetect_on_failure {
                        break;
                    }
                }
                Err(e) => eprintln!("Warning: Failed to parse config file {}: {}", file_path.display(), e),
            }
        }

        self.cache.insert(domain.to_string(), merged_config.clone());

        if found_configs { Ok(merged_config) } else { Ok(SiteConfig::new()) }
    }

    /// Load global configuration
    pub fn load_global(&mut self) -> Result<SiteConfig> {
        let mut global_config = SiteConfig::new();

        if let Some(custom_dir) = &self.custom_dir {
            let global_path = custom_dir.join("global.txt");
            if global_path.exists()
                && let Ok(config) = ConfigParser::parse_file(&global_path)
            {
                global_config.merge(&config);
            }
        }

        if let Some(standard_dir) = &self.standard_dir {
            let global_path = standard_dir.join("global.txt");
            if global_path.exists()
                && let Ok(config) = ConfigParser::parse_file(&global_path)
            {
                global_config.merge(&config);
            }
        }

        Ok(global_config)
    }

    /// Find all config files for a domain in priority order
    fn find_config_files(&self, domain: &str) -> Result<Vec<PathBuf>> {
        let mut config_files = Vec::new();

        let config_names = self.generate_config_names(domain);

        if let Some(custom_dir) = &self.custom_dir {
            for name in &config_names {
                let file_path = custom_dir.join(name);
                if file_path.exists() {
                    config_files.push(file_path);
                }
            }
        }

        if let Some(standard_dir) = &self.standard_dir {
            for name in &config_names {
                let file_path = standard_dir.join(name);
                if file_path.exists() && !config_files.contains(&file_path) {
                    config_files.push(file_path);
                }
            }
        }

        Ok(config_files)
    }

    /// Find all config files for a fingerprint hostname in priority order
    fn find_fingerprint_config_files(&self, hostname: &str) -> Result<Vec<PathBuf>> {
        let mut config_files = Vec::new();
        let config_name = format!("{}.txt", hostname);

        if let Some(custom_dir) = &self.custom_dir {
            let file_path = custom_dir.join(&config_name);
            if file_path.exists() {
                config_files.push(file_path);
            }
        }

        if let Some(standard_dir) = &self.standard_dir {
            let file_path = standard_dir.join(&config_name);
            if file_path.exists() && !config_files.contains(&file_path) {
                config_files.push(file_path);
            }
        }

        Ok(config_files)
    }

    /// Generate possible config file names for a domain
    fn generate_config_names(&self, domain: &str) -> Vec<String> {
        let mut names = Vec::new();

        names.push(format!("{}.txt", domain));

        if let Some(without_www) = domain.strip_prefix("www.") {
            names.push(format!("{}.txt", without_www));
        }

        if !domain.starts_with('.') {
            names.push(format!(".{}.txt", domain));
        }

        if let Some(without_www) = domain.strip_prefix("www.")
            && !without_www.starts_with('.')
        {
            names.push(format!(".{}.txt", without_www));
        }

        let parts: Vec<&str> = domain.split('.').collect();
        for i in 1..parts.len().saturating_sub(1) {
            let parent = parts[i..].join(".");
            if parent.contains('.') {
                names.push(format!("{}.txt", parent));
                names.push(format!(".{}.txt", parent));
            }
        }

        names
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> Result<String> {
        let url = url::Url::parse(url).map_err(|e| LectitoError::InvalidUrl(e.to_string()))?;

        let domain = url
            .host_str()
            .ok_or_else(|| LectitoError::InvalidUrl("No domain found in URL".to_string()))?;

        Ok(domain.to_string())
    }

    /// Clear the config cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Preload configs for a list of domains
    pub fn preload_configs(&mut self, domains: &[&str]) -> Result<()> {
        for domain in domains {
            self.load_for_domain(domain)?;
        }
        Ok(())
    }
}

/// Builder for ConfigLoader
#[derive(Debug)]
pub struct ConfigLoaderBuilder {
    custom_dir: Option<PathBuf>,
    standard_dir: Option<PathBuf>,
}

impl ConfigLoaderBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { custom_dir: None, standard_dir: None }
    }

    /// Set custom config directory
    pub fn custom_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.custom_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set standard config directory
    pub fn standard_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.standard_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Build the ConfigLoader
    pub fn build(self) -> ConfigLoader {
        ConfigLoader { custom_dir: self.custom_dir, standard_dir: self.standard_dir, cache: HashMap::new() }
    }
}

impl Default for ConfigLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        let mut builder = ConfigLoaderBuilder::new();

        if let Some(custom_dir) = Self::default_custom_dir() {
            builder = builder.custom_dir(custom_dir);
        }

        if let Some(standard_dir) = Self::default_standard_dir() {
            builder = builder.standard_dir(standard_dir);
        }

        builder.build()
    }
}

impl ConfigLoader {
    /// Get default custom config directory (~/.config/lectito/sites)
    fn default_custom_dir() -> Option<PathBuf> {
        if let Some(home_dir) = dirs::home_dir() {
            let config_dir = home_dir.join(".config").join("lectito").join("sites");
            let _ = fs::create_dir_all(&config_dir);
            Some(config_dir)
        } else {
            None
        }
    }

    /// Get default standard config directory (bundled with binary)
    fn default_standard_dir() -> Option<PathBuf> {
        // TODO: set during build/install
        let std_dir = PathBuf::from("site_configs");
        if std_dir.exists() { Some(std_dir) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_config_names() {
        let loader = ConfigLoader::new();
        let names = loader.generate_config_names("example.com");
        assert!(names.contains(&"example.com.txt".to_string()));
        assert!(names.contains(&".example.com.txt".to_string()));

        let names = loader.generate_config_names("www.example.com");
        assert!(names.contains(&"www.example.com.txt".to_string()));
        assert!(names.contains(&"example.com.txt".to_string()));
        assert!(names.contains(&".example.com.txt".to_string()));

        let names = loader.generate_config_names("news.example.com");
        assert!(names.contains(&"news.example.com.txt".to_string()));
        assert!(names.contains(&".news.example.com.txt".to_string()));
    }

    #[test]
    fn test_generate_config_names_parent_domains() {
        let loader = ConfigLoader::new();
        let names = loader.generate_config_names("en.wikipedia.org");

        assert!(names.contains(&"en.wikipedia.org.txt".to_string()));
        assert!(names.contains(&".en.wikipedia.org.txt".to_string()));

        assert!(names.contains(&"wikipedia.org.txt".to_string()));
        assert!(names.contains(&".wikipedia.org.txt".to_string()));

        assert!(!names.iter().any(|n| n == "org.txt" || n == ".org.txt"));
    }

    #[test]
    fn test_generate_config_names_deep_subdomain() {
        let loader = ConfigLoader::new();
        let names = loader.generate_config_names("news.bbc.co.uk");

        assert!(names.contains(&"news.bbc.co.uk.txt".to_string()));
        assert!(names.contains(&".news.bbc.co.uk.txt".to_string()));

        assert!(names.contains(&"bbc.co.uk.txt".to_string()));
        assert!(names.contains(&".bbc.co.uk.txt".to_string()));

        assert!(!names.iter().any(|n| n == "uk.txt" || n == ".uk.txt"));
    }

    #[test]
    fn test_extract_domain() {
        let loader = ConfigLoader::new();

        assert_eq!(
            loader.extract_domain("https://example.com/article").unwrap(),
            "example.com"
        );
        assert_eq!(
            loader.extract_domain("https://www.example.com/path").unwrap(),
            "www.example.com"
        );
        assert_eq!(
            loader.extract_domain("http://news.example.org/story").unwrap(),
            "news.example.org"
        );
    }

    #[test]
    fn test_config_loader_builder() {
        let temp_dir = TempDir::new().unwrap();
        let custom_path = temp_dir.path().join("custom");
        let standard_path = temp_dir.path().join("standard");

        fs::create_dir_all(&custom_path).unwrap();
        fs::create_dir_all(&standard_path).unwrap();

        let loader = ConfigLoaderBuilder::new()
            .custom_dir(&custom_path)
            .standard_dir(&standard_path)
            .build();

        assert_eq!(loader.custom_dir, Some(custom_path));
        assert_eq!(loader.standard_dir, Some(standard_path));
    }

    #[test]
    fn test_load_for_domain() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("example.com.txt");

        fs::write(&config_path, "title: //h1\nbody: //article\n").unwrap();

        let mut loader = ConfigLoaderBuilder::new().custom_dir(temp_dir.path()).build();

        let config = loader.load_for_domain("example.com").unwrap();

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.body.len(), 1);
        assert_eq!(config.title[0], "//h1");
        assert_eq!(config.body[0], "//article");
    }

    #[test]
    fn test_load_global() {
        let temp_dir = TempDir::new().unwrap();
        let global_path = temp_dir.path().join("global.txt");

        fs::write(&global_path, "strip_id_or_class: sidebar\nprune: no\n").unwrap();

        let mut loader = ConfigLoaderBuilder::new().custom_dir(temp_dir.path()).build();

        let config = loader.load_global().unwrap();

        assert_eq!(config.strip_id_or_class.len(), 1);
        assert_eq!(config.prune, Some(false));
    }

    #[test]
    fn test_config_caching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("example.com.txt");

        fs::write(&config_path, "title: //h1\n").unwrap();

        let mut loader = ConfigLoaderBuilder::new().custom_dir(temp_dir.path()).build();

        let config1 = loader.load_for_domain("example.com").unwrap();
        assert_eq!(loader.cache.len(), 1);

        let config2 = loader.load_for_domain("example.com").unwrap();
        assert_eq!(config1.title, config2.title);
        assert_eq!(loader.cache.len(), 1);
    }

    #[test]
    fn test_config_merge_priority() {
        let temp_dir = TempDir::new().unwrap();

        let custom_path = temp_dir.path().join("custom");
        fs::create_dir_all(&custom_path).unwrap();
        fs::write(custom_path.join("example.com.txt"), "title: //h1\ntidy: yes\n").unwrap();

        let standard_path = temp_dir.path().join("standard");
        fs::create_dir_all(&standard_path).unwrap();
        fs::write(standard_path.join("example.com.txt"), "body: //article\ntidy: no\n").unwrap();

        let mut loader = ConfigLoaderBuilder::new()
            .custom_dir(&custom_path)
            .standard_dir(&standard_path)
            .build();

        let config = loader.load_for_domain("example.com").unwrap();

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.body.len(), 1);

        assert_eq!(config.tidy, Some(true));
    }
}
