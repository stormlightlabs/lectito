use crate::{Document, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct SiteConfig {
    pub title: String,
    pub body: String,
    pub http_headers: HashMap<String, String>,
}

impl SiteConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_extraction_config(&self) -> bool {
        !self.title.is_empty() || !self.body.is_empty()
    }

    pub fn should_autodetect(&self) -> bool {
        false
    }

    pub fn merge(&mut self, other: &SiteConfig) {
        if self.title.is_empty() {
            self.title = other.title.clone();
        }
        if self.body.is_empty() {
            self.body = other.body.clone();
        }
        self.http_headers.extend(other.http_headers.clone());
    }

    pub fn extract_title(&self, _html: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub fn extract_author(&self, _html: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub fn extract_date(&self, _html: &str) -> Result<Option<String>> {
        Ok(None)
    }

    pub fn extract_body(&self, _html: &str) -> Result<Option<String>> {
        Ok(None)
    }
}

pub trait SiteConfigProcessing {
    fn apply_text_replacements(&self, html: &str) -> String;
    fn apply_strip_directives(&self, html: &str) -> Result<String>;
}

impl SiteConfigProcessing for SiteConfig {
    fn apply_text_replacements(&self, html: &str) -> String {
        html.to_string()
    }

    fn apply_strip_directives(&self, html: &str) -> Result<String> {
        Ok(html.to_string())
    }
}

pub trait SiteConfigXPath {
    fn extract_body_html(&self, doc: &Document) -> Result<Option<String>>;
}

impl SiteConfigXPath for SiteConfig {
    fn extract_body_html(&self, _doc: &Document) -> Result<Option<String>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigLoader {
    _custom_dir: Option<PathBuf>,
    _standard_dir: Option<PathBuf>,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_for_url(&mut self, _url: &str) -> Result<SiteConfig> {
        Ok(SiteConfig::new())
    }

    pub fn load_for_html(&mut self, _html: &str) -> Result<SiteConfig> {
        Ok(SiteConfig::new())
    }

    pub fn load_merged_for_url(&mut self, _url: &str, _html: Option<&str>) -> Result<SiteConfig> {
        Ok(SiteConfig::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConfigLoaderBuilder {
    custom_dir: Option<PathBuf>,
    standard_dir: Option<PathBuf>,
}

impl ConfigLoaderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn custom_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.custom_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn standard_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.standard_dir = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> ConfigLoader {
        ConfigLoader { _custom_dir: self.custom_dir, _standard_dir: self.standard_dir }
    }
}

pub struct ConfigParser;

impl ConfigParser {
    pub fn parse_file<P: AsRef<Path>>(_path: P) -> Result<SiteConfig> {
        Ok(SiteConfig::new())
    }

    pub fn parse_string(_content: &str) -> Result<SiteConfig> {
        Ok(SiteConfig::new())
    }
}

#[derive(Debug, Clone, Default)]
pub struct FingerprintMatcher;

#[derive(Debug, Clone)]
pub enum Directive {}
