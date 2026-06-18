use serde::Serialize;

use crate::Article;

/// Format an article as Markdown with TOML frontmatter.
///
/// The frontmatter includes available metadata from [`Article`] plus the
/// optional source URL.
pub fn markdown_with_toml_frontmatter(article: &Article, source: Option<&str>) -> Result<String, toml::ser::Error> {
    let frontmatter = Frontmatter {
        title: non_empty(article.title.as_deref()),
        author: non_empty(article.byline.as_deref()),
        site: non_empty(article.site_name.as_deref()),
        published: non_empty(article.published_time.as_deref()),
        source: non_empty(source),
        domain: non_empty(article.domain.as_deref()),
        language: non_empty(article.lang.as_deref()),
        description: non_empty(article.excerpt.as_deref()),
        image: non_empty(article.image.as_deref()),
        favicon: non_empty(article.favicon.as_deref()),
        dir: non_empty(article.dir.as_deref()),
        length: article.length,
    };
    let metadata = toml::to_string(&frontmatter)?;
    Ok(format!("+++\n{}+++\n\n{}", metadata, article.markdown))
}

#[derive(Serialize)]
struct Frontmatter<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    site: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    favicon: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dir: Option<&'a str>,
    length: usize,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
