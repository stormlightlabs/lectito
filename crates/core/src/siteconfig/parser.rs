use crate::error::{LectitoError, Result};
use crate::siteconfig::directives::{SiteConfig, parse_directive};
use std::io::{BufRead, BufReader};
use std::path::Path;

/// FTR config file parser
#[derive(Debug)]
pub struct ConfigParser;

impl ConfigParser {
    /// Parse a single FTR config file
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<SiteConfig> {
        let file = std::fs::File::open(&path).map_err(|e| {
            LectitoError::SiteConfigError(format!("Cannot open file {}: {}", path.as_ref().display(), e))
        })?;

        let reader = BufReader::new(file);
        Self::parse_reader(reader)
    }

    /// Parse FTR config from a reader
    pub fn parse_reader<R: BufRead>(reader: R) -> Result<SiteConfig> {
        let mut config = SiteConfig::new();
        let mut line_number = 0;

        for line in reader.lines() {
            line_number += 1;
            let line =
                line.map_err(|e| LectitoError::SiteConfigError(format!("Read error at line {}: {}", line_number, e)))?;

            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match parse_directive(line) {
                Ok(directive) => config.add_directive(directive),
                Err(e) => {
                    return Err(LectitoError::SiteConfigError(format!(
                        "Parse error at line {}: {}",
                        line_number, e
                    )));
                }
            }
        }

        Ok(config)
    }

    /// Parse FTR config from a string
    pub fn parse_string(content: &str) -> Result<SiteConfig> {
        let mut config = SiteConfig::new();
        let mut line_number = 0;

        for line in content.lines() {
            line_number += 1;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match parse_directive(line) {
                Ok(directive) => config.add_directive(directive),
                Err(e) => {
                    return Err(LectitoError::SiteConfigError(format!(
                        "Parse error at line {}: {}",
                        line_number, e
                    )));
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_string_basic() {
        let content = r#"
# Example config
title: //h1[@class='title']
body: //div[@id='content']
strip: //div[@class='sidebar']
tidy: yes
"#;

        let config = ConfigParser::parse_string(content).unwrap();

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.body.len(), 1);
        assert_eq!(config.strip.len(), 1);
        assert_eq!(config.tidy, Some(true));

        assert_eq!(config.title[0], "//h1[@class='title']");
        assert_eq!(config.body[0], "//div[@id='content']");
        assert_eq!(config.strip[0], "//div[@class='sidebar']");
    }

    #[test]
    fn test_parse_string_multiple_directives() {
        let content = r#"
title: //h1
title: //meta[@property='og:title']/@content
body: //article
body: //div[@class='post-body']
strip_id_or_class: sidebar
strip_id_or_class: advertisement
"#;

        let config = ConfigParser::parse_string(content).unwrap();

        assert_eq!(config.title.len(), 2);
        assert_eq!(config.body.len(), 2);
        assert_eq!(config.strip_id_or_class.len(), 2);
    }

    #[test]
    fn test_parse_string_http_headers() {
        let content = r#"
http_header(User-Agent): Mozilla/5.0 (compatible; Lectito/1.0)
http_header(Cookie): euConsent=true
"#;

        let config = ConfigParser::parse_string(content).unwrap();

        assert_eq!(config.http_headers.len(), 2);
        assert_eq!(
            config.http_headers.get("User-Agent"),
            Some(&"Mozilla/5.0 (compatible; Lectito/1.0)".to_string())
        );
        assert_eq!(config.http_headers.get("Cookie"), Some(&"euConsent=true".to_string()));
    }

    #[test]
    fn test_parse_string_text_replacement() {
        let content = r#"
find_string: <p />
replace_string: <br /><br />
"#;

        let config = ConfigParser::parse_string(content).unwrap();

        assert_eq!(config.text_replacements.len(), 1);
        assert_eq!(
            config.text_replacements[0],
            ("<p />".to_string(), "<br /><br />".to_string())
        );
    }

    #[test]
    fn test_parse_reader() {
        let content = r#"
title: //h1
body: //article
"#;

        let cursor = Cursor::new(content);
        let config = ConfigParser::parse_reader(cursor).unwrap();

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.body.len(), 1);
    }

    #[test]
    fn test_parse_invalid_directive() {
        let content = "invalid_directive_without_colon";

        let result = ConfigParser::parse_string(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_and_comments() {
        let content = r#"
# This is a comment
# Another comment

title: //h1

# Final comment
"#;

        let config = ConfigParser::parse_string(content).unwrap();

        assert_eq!(config.title.len(), 1);
        assert_eq!(config.title[0], "//h1");
    }
}
