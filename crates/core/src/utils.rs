pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn strip_tags(value: &str) -> String {
    static TAG_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    TAG_RE
        .get_or_init(|| regex::Regex::new(r"<[^>]+>").unwrap())
        .replace_all(value, " ")
        .to_string()
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
pub fn count_words(text: &str) -> usize {
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

    #[test]
    fn test_strip_tags() {
        let html = r#"<p>This is <strong>bold</strong> text</p>"#;
        let result = strip_tags(html);
        assert_eq!(result, "This is bold text");
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
}
