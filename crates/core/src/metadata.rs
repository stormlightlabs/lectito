use crate::Document;
use regex::Regex;
use serde::Serialize;
use url::Url;

/// Represents all extracted metadata from a document
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub image: Option<String>,
    pub favicon: Option<String>,
    pub word_count: Option<usize>,
    pub reading_time_minutes: Option<f64>,
    /// Detected language code (e.g., "en", "es", "fr")
    pub language: Option<String>,
}

impl Metadata {
    /// Merge a partial metadata patch, preferring patch values when present.
    pub fn apply_patch(&mut self, patch: &Metadata) {
        if patch.title.is_some() {
            self.title = patch.title.clone();
        }
        if patch.author.is_some() {
            self.author = patch.author.clone();
        }
        if patch.date.is_some() {
            self.date = patch.date.clone();
        }
        if patch.excerpt.is_some() {
            self.excerpt = patch.excerpt.clone();
        }
        if patch.site_name.is_some() {
            self.site_name = patch.site_name.clone();
        }
        if patch.image.is_some() {
            self.image = patch.image.clone();
        }
        if patch.favicon.is_some() {
            self.favicon = patch.favicon.clone();
        }
        if patch.word_count.is_some() {
            self.word_count = patch.word_count;
        }
        if patch.reading_time_minutes.is_some() {
            self.reading_time_minutes = patch.reading_time_minutes;
        }
        if patch.language.is_some() {
            self.language = patch.language.clone();
        }
    }

    /// Return a new metadata value with a partial patch applied.
    pub fn with_patch(mut self, patch: &Metadata) -> Self {
        self.apply_patch(patch);
        self
    }
}

impl Document {
    /// Extract title with priority fallback:
    /// 1. `<title>` element (page title)
    /// 2. Open Graph `og:title`
    /// 3. Twitter `twitter:title`
    /// 4. Meta `title` / `DC.title`
    /// 5. JSON-LD `headline`
    /// 6. First `<h1>` element
    pub fn extract_title(&self) -> Option<String> {
        let raw_title = self.extract_title_raw()?;
        let (title, _) = clean_title(&raw_title, self.extract_site_name_raw().as_deref());
        Some(title)
    }

    /// Extract author with priority fallback:
    /// 1. JSON-LD `author.name`
    /// 2. Meta `author` / `DC.creator`
    /// 3. `[rel="author"]` link text
    /// 4. `[itemprop="author"]` content
    /// 5. Class/ID containing "author", "byline"
    pub fn extract_author(&self) -> Option<String> {
        for json_ld in self.extract_json_ld_values() {
            if let Some(author) = find_json_ld_field(&json_ld, &["author"])
                && let Some(name) = Self::extract_author_from_json_ld(author)
            {
                return Some(name);
            }
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
        if let Some(value) = self.first_json_ld_string(&["datePublished"]) {
            return Some(value);
        }

        if let Some(value) = self.first_json_ld_string(&["dateCreated"]) {
            return Some(value);
        }

        if let Some(date) = self.get_meta_content("article:published_time") {
            return Some(date);
        }

        if let Some(date) = self.get_meta_content("publishDate") {
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
        if let Some(value) = self.first_json_ld_string(&["description"]) {
            return Some(value);
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
        let raw_site_name = self.extract_site_name_raw();
        let detected_site_name = self
            .extract_title_raw()
            .and_then(|title| clean_title(&title, raw_site_name.as_deref()).1);

        raw_site_name.or(detected_site_name)
    }

    /// Extract representative image with priority fallback:
    /// 1. Open Graph `og:image`
    /// 2. Twitter `twitter:image`
    /// 3. JSON-LD `image.url` / `image`
    pub fn extract_image(&self) -> Option<String> {
        self.get_meta_content("og:image")
            .or_else(|| self.get_meta_content("twitter:image"))
            .or_else(|| self.first_json_ld_string(&["image", "url"]))
            .or_else(|| self.first_json_ld_string(&["image"]))
            .and_then(|value| self.resolve_url(&value))
    }

    /// Extract favicon URL from `<link rel=icon>` tags with `/favicon.ico` fallback.
    pub fn extract_favicon(&self) -> Option<String> {
        if let Ok(elements) = self.select("link[href]") {
            for element in elements {
                let rel = element.attr("rel").unwrap_or("").to_ascii_lowercase();
                let is_icon = rel == "shortcut icon"
                    || rel == "apple-touch-icon"
                    || rel.split_whitespace().any(|token| token == "icon");

                if is_icon
                    && let Some(href) = element.attr("href")
                    && let Some(url) = self.resolve_url(href)
                {
                    return Some(url);
                }
            }
        }

        self.base_url()
            .and_then(|base_url| base_url.join("/favicon.ico").ok())
            .map(|url| url.to_string())
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

    /// Extract language with priority fallback:
    /// 1. HTML `lang` attribute on `<html>` element
    /// 2. Meta `http-equiv="Content-Language"` content
    /// 3. Meta `og:locale` property
    /// 4. JSON-LD `inLanguage` property
    pub fn extract_language(&self) -> Option<String> {
        let root_el = self.html().root_element();
        if let Some(lang) = root_el.value().attr("lang") {
            let lang_code = lang.split('-').next().unwrap_or(lang);
            if !lang_code.is_empty() {
                return Some(lang_code.to_lowercase());
            }
        }

        if let Ok(elements) = self.select("meta[http-equiv=\"Content-Language\"]")
            && let Some(el) = elements.first()
            && let Some(lang) = el.attr("content")
        {
            let lang_code = lang.split('-').next().unwrap_or(lang);
            if !lang_code.is_empty() {
                return Some(lang_code.to_lowercase());
            }
        }

        if let Some(locale) = self.get_meta_content("og:locale") {
            let lang_code = locale.split('_').next().unwrap_or(&locale);
            if !lang_code.is_empty() {
                return Some(lang_code.to_lowercase());
            }
        }

        if let Some(lang_str) = self.first_json_ld_string(&["inLanguage"]) {
            let lang_code = lang_str.split('-').next().unwrap_or(lang_str.as_str());
            if !lang_code.is_empty() {
                return Some(lang_code.to_lowercase());
            }
        }

        None
    }

    /// Detect language from text content using common word patterns
    ///
    /// This is a basic heuristic that looks for common words in major languages.
    /// Returns a 2-letter ISO 639-1 code if detected.
    fn detect_language_from_content(&self) -> Option<String> {
        let text = self.text_content();
        let text_lower = text.to_lowercase();

        let common_words = [
            ("en", &["the", "be", "to", "of", "and", "a", "in", "that", "have", "i"]),
            ("es", &["el", "la", "de", "que", "y", "a", "en", "un", "ser", "se"]),
            ("fr", &["le", "de", "un", "etre", "et", "a", "il", "avoir", "ne", "je"]),
            (
                "de",
                &["der", "die", "und", "in", "den", "von", "das", "mit", "sich", "des"],
            ),
            ("it", &["il", "di", "che", "e", "la", "un", "a", "per", "non", "in"]),
            ("pt", &["o", "de", "a", "e", "do", "da", "em", "um", "para", "e"]),
            ("ru", &["и", "в", "не", "на", "я", "быть", "он", "с", "что", "а"]),
            ("ja", &["の", "に", "は", "を", "た", "が", "で", "て", "だ", "する"]),
            ("zh", &["的", "是", "在", "了", "和", "有", "大", "这", "主", "为"]),
        ];

        let mut scores: Vec<(i32, &str)> = Vec::new();

        for (lang, words) in &common_words {
            let mut score = 0;
            for word in *words {
                if text_lower.contains(word) {
                    score += 1;
                }
            }
            if score > 0 {
                scores.push((score, lang));
            }
        }

        scores.sort_by(|a, b| b.0.cmp(&a.0));

        scores
            .into_iter()
            .filter(|(score, _)| *score >= 3)
            .map(|(_, lang)| lang.to_string())
            .next()
    }

    /// Extract all metadata at once
    pub fn extract_metadata(&self) -> Metadata {
        let raw_site_name = self.extract_site_name_raw();
        let raw_title = self.extract_title_raw();
        let (title, detected_site_name) = raw_title
            .as_deref()
            .map(|value| clean_title(value, raw_site_name.as_deref()))
            .unwrap_or_else(|| (String::new(), None));

        Metadata {
            title: (!title.is_empty()).then_some(title),
            author: self.extract_author(),
            date: self.extract_date(),
            excerpt: self.extract_excerpt(),
            site_name: raw_site_name.or(detected_site_name),
            image: self.extract_image(),
            favicon: self.extract_favicon(),
            word_count: Some(self.calculate_word_count()),
            reading_time_minutes: Some(self.calculate_reading_time()),
            language: self.extract_language().or_else(|| self.detect_language_from_content()),
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

    fn extract_title_raw(&self) -> Option<String> {
        if let Some(title) = self.title() {
            let trimmed = title.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        for key in ["og:title", "twitter:title", "title", "DC.title"] {
            if let Some(title) = self.get_meta_content(key) {
                let trimmed = title.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }

        if let Some(title) = self.first_json_ld_string(&["headline"]) {
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

    fn extract_site_name_raw(&self) -> Option<String> {
        let candidate = self
            .first_json_ld_string(&["publisher", "name"])
            .or_else(|| self.get_meta_content("og:site_name"))
            .or_else(|| self.get_meta_content("application-name"))
            .or_else(|| {
                self.base_url()
                    .and_then(|base_url| base_url.domain().map(|domain| domain.to_string()))
            });

        candidate.filter(|value| count_words(value) <= 6)
    }

    /// Extract and parse JSON-LD from script tags
    fn extract_json_ld_values(&self) -> Vec<serde_json::Value> {
        let mut values = Vec::new();
        if let Ok(elements) = self.select("script[type=\"application/ld+json\"]") {
            for el in elements.iter() {
                let text = el.text();
                let json_str = text.trim();
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    values.push(value);
                }
            }
        }
        values
    }

    fn first_json_ld_string(&self, path: &[&str]) -> Option<String> {
        for value in self.extract_json_ld_values() {
            if let Some(field) = find_json_ld_field(&value, path)
                && let Some(text) = json_ld_value_as_string(field)
            {
                return Some(text);
            }
        }
        None
    }

    fn resolve_url(&self, value: &str) -> Option<String> {
        normalize_url_value(self.base_url(), value)
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

fn clean_title(title: &str, site_name: Option<&str>) -> (String, Option<String>) {
    let trimmed_title = title.trim();
    if trimmed_title.is_empty() {
        return (String::new(), None);
    }

    let separators = r"\s*(?:[|\-–—/·])\s*";

    if let Some(site_name) = site_name
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case(trimmed_title) && count_words(value) <= 6)
    {
        let site_name_escaped = regex::escape(site_name);
        for pattern in [
            format!(r"{separators}{site_name_escaped}$"),
            format!(r"^{site_name_escaped}{separators}"),
        ] {
            let regex = Regex::new(&pattern).unwrap();
            if regex.is_match(trimmed_title) {
                let cleaned = regex.replace(trimmed_title, "").trim().to_string();
                if !cleaned.is_empty() {
                    return (cleaned, Some(site_name.to_string()));
                }
            }
        }

        let positions = separator_positions(trimmed_title, &Regex::new(r"\s+[|\-–—/·]\s+").unwrap());
        if !positions.is_empty() {
            let site_name_lower = site_name.to_lowercase();

            let (last_start, last_end) = positions[positions.len() - 1];
            let last_segment = trimmed_title[last_end..].trim().to_lowercase();
            if !last_segment.is_empty() && site_name_lower.contains(&last_segment) {
                let mut cut_index = last_start;
                for (start, end) in positions.iter().rev().skip(1) {
                    let segment = trimmed_title[*end..cut_index].trim();
                    if count_words(segment) > 3 {
                        break;
                    }
                    cut_index = *start;
                }
                let cleaned = trimmed_title[..cut_index].trim().to_string();
                if !cleaned.is_empty() {
                    return (cleaned, Some(site_name.to_string()));
                }
            }

            let (first_start, first_end) = positions[0];
            let first_segment = trimmed_title[..first_start].trim().to_lowercase();
            if !first_segment.is_empty() && site_name_lower.contains(&first_segment) {
                let mut cut_index = first_end;
                for (start, end) in positions.iter().skip(1) {
                    let segment = trimmed_title[cut_index..*start].trim();
                    if count_words(segment) > 3 {
                        break;
                    }
                    cut_index = *end;
                }
                let cleaned = trimmed_title[cut_index..].trim().to_string();
                if !cleaned.is_empty() {
                    return (cleaned, Some(site_name.to_string()));
                }
            }
        }
    }

    if let Some((cleaned, detected_site_name)) = try_separator_split(
        trimmed_title,
        &Regex::new(r"\s+[|/·]\s+").unwrap(),
        false,
        |title_words, site_words| site_words <= 3 && title_words >= 2 && title_words >= site_words.saturating_mul(2),
    ) {
        return (cleaned, Some(detected_site_name));
    }

    if let Some((cleaned, detected_site_name)) = try_separator_split(
        trimmed_title,
        &Regex::new(r"\s+[-–—]\s+").unwrap(),
        true,
        |title_words, site_words| site_words <= 2 && title_words >= 2 && title_words > site_words,
    ) {
        return (cleaned, Some(detected_site_name));
    }

    (trimmed_title.to_string(), None)
}

fn separator_positions(title: &str, pattern: &Regex) -> Vec<(usize, usize)> {
    pattern
        .find_iter(title)
        .map(|match_| (match_.start(), match_.end()))
        .collect()
}

fn try_separator_split<F>(title: &str, pattern: &Regex, suffix_only: bool, guard: F) -> Option<(String, String)>
where
    F: Fn(usize, usize) -> bool,
{
    let positions = separator_positions(title, pattern);
    if positions.is_empty() {
        return None;
    }

    let (last_start, last_end) = positions[positions.len() - 1];
    let suffix_title = title[..last_start].trim();
    let suffix_site = title[last_end..].trim();
    if !suffix_title.is_empty() && !suffix_site.is_empty() && guard(count_words(suffix_title), count_words(suffix_site))
    {
        return Some((suffix_title.to_string(), suffix_site.to_string()));
    }

    if suffix_only {
        return None;
    }

    let (first_start, first_end) = positions[0];
    let prefix_site = title[..first_start].trim();
    let prefix_title = title[first_end..].trim();
    if !prefix_title.is_empty() && !prefix_site.is_empty() && guard(count_words(prefix_title), count_words(prefix_site))
    {
        return Some((prefix_title.to_string(), prefix_site.to_string()));
    }

    None
}

fn find_json_ld_field<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    if path.is_empty() {
        return Some(value);
    }

    match value {
        serde_json::Value::Object(map) => {
            if let Some(next) = map.get(path[0])
                && let Some(found) = find_json_ld_field(next, &path[1..])
            {
                return Some(found);
            }

            for nested in map.values() {
                if let Some(found) = find_json_ld_field(nested, path) {
                    return Some(found);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(found) = find_json_ld_field(item, path) {
                    return Some(found);
                }
            }
        }
        _ => {}
    }

    None
}

fn json_ld_value_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        serde_json::Value::Object(map) => map
            .get("url")
            .or_else(|| map.get("name"))
            .and_then(json_ld_value_as_string),
        serde_json::Value::Array(items) => items.iter().find_map(json_ld_value_as_string),
        _ => None,
    }
}

fn normalize_url_value(base_url: Option<&Url>, value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Url::parse(trimmed)
        .ok()
        .or_else(|| base_url.and_then(|base| base.join(trimmed).ok()))
        .map(|url| url.to_string())
        .or_else(|| Some(trimmed.to_string()))
}

fn is_cjk_character(ch: char) -> bool {
    matches!(
        ch,
        '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{AC00}'..='\u{D7AF}'
    )
}

/// Count words in text, handling CJK scripts that do not use spaces between words.
pub(crate) fn count_words(text: &str) -> usize {
    let mut cjk_count = 0usize;
    let mut word_count = 0usize;
    let mut in_word = false;

    for ch in text.chars() {
        if is_cjk_character(ch) {
            cjk_count += 1;
            in_word = false;
        } else if ch.is_alphanumeric() {
            if !in_word {
                word_count += 1;
                in_word = true;
            }
        } else if matches!(ch, '\'' | '’' | '-') && in_word {
            continue;
        } else {
            in_word = false;
        }
    }

    cjk_count + word_count
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
            <meta property="og:image" content="/images/cover.png">
            <meta property="article:published_time" content="2024-01-15T10:30:00Z">
            <link rel="icon" href="/favicon.ico">
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
        assert_eq!(title, Some("Test Page Title".to_string()));
    }

    #[test]
    fn test_extract_title_fallback() {
        let doc = Document::parse(HTML_WITHOUT_META).unwrap();
        let title = doc.extract_title();
        assert_eq!(title, Some("Simple Page".to_string()));
    }

    #[test]
    fn test_extract_title_strips_site_suffix() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Rust ownership explained | Example Docs</title>
                <meta property="og:site_name" content="Example Docs">
            </head>
            <body></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_title(), Some("Rust ownership explained".to_string()));
        assert_eq!(doc.extract_site_name(), Some("Example Docs".to_string()));
    }

    #[test]
    fn test_extract_title_detects_site_name_from_separator_heuristics() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Understanding borrow checking | MDN</title>
            </head>
            <body></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_title(), Some("Understanding borrow checking".to_string()));
        assert_eq!(doc.extract_site_name(), Some("MDN".to_string()));
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
    fn test_extract_image_and_favicon_with_base_url() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta property="og:image" content="/images/article.png">
                <link rel="shortcut icon" href="/assets/favicon.png">
            </head>
            <body></body>
            </html>
        "#;
        let base_url = Url::parse("https://example.com/posts/test").unwrap();
        let doc = Document::parse_with_base_url(html, Some(base_url)).unwrap();

        assert_eq!(
            doc.extract_image(),
            Some("https://example.com/images/article.png".to_string())
        );
        assert_eq!(
            doc.extract_favicon(),
            Some("https://example.com/assets/favicon.png".to_string())
        );
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
        assert!(metadata.image.is_some());
        assert!(metadata.favicon.is_some());
        assert!(metadata.word_count.is_some());
        assert!(metadata.reading_time_minutes.is_some());

        assert_eq!(metadata.title, Some("Test Page Title".to_string()));
        assert_eq!(metadata.author, Some("Jane Smith".to_string()));
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("one"), 1);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("a b c d e"), 5);
        assert_eq!(count_words("word's with-apostrophe"), 2);
        assert_eq!(count_words("日本語abc"), 4);
        assert_eq!(count_words("漢字かなカナ"), 6);
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

    #[test]
    fn test_extract_language_from_html_lang() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="en">
            <head><title>Test</title></head>
            <body><p>Content</p></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_language(), Some("en".to_string()));
    }

    #[test]
    fn test_extract_language_from_content_language_meta() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta http-equiv="Content-Language" content="es">
            </head>
            <body><p>Contenido</p></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_language(), Some("es".to_string()));
    }

    #[test]
    fn test_extract_language_from_og_locale() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <meta property="og:locale" content="fr_FR">
            </head>
            <body><p>Contenu</p></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_language(), Some("fr".to_string()));
    }

    #[test]
    fn test_extract_language_from_json_ld() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <script type="application/ld+json">
                {
                    "@context": "https://schema.org",
                    "@type": "Article",
                    "inLanguage": "de-DE"
                }
                </script>
            </head>
            <body><p>Inhalt</p></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_language(), Some("de".to_string()));
    }

    #[test]
    fn test_detect_language_from_english_content() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <body>
                <p>The quick brown fox jumps over the lazy dog. This is a test of the language detection system.</p>
                <p>We have to be sure that this works properly and that it can detect the language.</p>
            </body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.detect_language_from_content(), Some("en".to_string()));
    }

    #[test]
    fn test_extract_metadata_includes_language() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta name="author" content="Test Author">
                <meta name="description" content="Test description">
            </head>
            <body>
                <h1>Test Title</h1>
                <p>The article content goes here and should be detected as English content.</p>
            </body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        let metadata = doc.extract_metadata();
        assert_eq!(metadata.language, Some("en".to_string()));
    }

    #[test]
    fn test_language_normalization() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="en-US">
            <body><p>Content</p></body>
            </html>
        "#;
        let doc = Document::parse(html).unwrap();
        assert_eq!(doc.extract_language(), Some("en".to_string()));
    }

    #[test]
    fn test_extract_metadata_serialization() {
        let metadata = Metadata {
            title: Some("Test Title".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2024-01-15".to_string()),
            excerpt: Some("Test excerpt".to_string()),
            site_name: Some("Test Site".to_string()),
            image: Some("https://example.com/image.png".to_string()),
            favicon: Some("https://example.com/favicon.ico".to_string()),
            word_count: Some(500),
            reading_time_minutes: Some(2.5),
            language: Some("en".to_string()),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains(r#""title":"Test Title""#));
        assert!(json.contains(r#""image":"https://example.com/image.png""#));
        assert!(json.contains(r#""language":"en""#));
    }
}
