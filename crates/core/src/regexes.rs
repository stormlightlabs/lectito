use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegexPattern {
    /// Removes raw `<script>` blocks before expensive DOM scoring.
    RawScript,
    /// Matches JSON-LD schema types that can describe article content.
    JsonLdArticleType,
    /// Flags nodes that are usually navigation, ads, comments, or chrome.
    UnlikelyCandidates,
    /// Allows otherwise unlikely nodes when their class/id still looks article-like.
    MaybeCandidate,
    /// Scores class/id names that usually indicate primary content.
    Positive,
    /// Scores class/id names that usually indicate page chrome.
    Negative,
    /// Collapses runs of whitespace during text normalization.
    NormalizeWhitespace,
    /// Counts comma variants used by readability scoring.
    Comma,
    /// Detects social sharing widgets by class/id.
    ShareElements,
    /// Detects placeholder ad/loading text in empty-ish blocks.
    AdOrLoadingWords,
    /// Detects lazy image URL attributes that contain a direct image URL.
    LazyImageUrl,
    /// Detects lazy image URL attributes that contain a `srcset`-like value.
    LazyImageSrcset,
    /// Detects trailing chrome by class/id/role attributes.
    TrailingChromeAttrs,
    /// Detects trailing chrome by heading or short block text.
    TrailingChromeText,
    /// Detects footnote/reference sections by class/id/role attributes.
    FootnoteReferenceAttrs,
    /// Detects footnote/reference headings by text.
    FootnoteReferenceText,
    /// Detects leading byline dates that should be stripped from article text.
    LeadingDateText,
    /// Finds mobile media queries that may reveal hidden mobile content.
    MobileMediaBlock,
    /// Finds CSS rules with display declarations inside mobile media blocks.
    CssRule,
    /// Extracts a `display` value from a CSS declaration block.
    DisplayDecl,
    /// Finds declarative shadow DOM templates in raw HTML snapshots.
    ShadowTemplateHtml,
    /// Extracts a stable numeric label from footnote ids.
    FootnoteTrailingNumber,
    /// Removes leading byline words such as `by` or `written by`.
    BylinePrefix,
    /// Removes dates and update text from bylines.
    BylineTrailingDate,
}

impl RegexPattern {
    pub fn to_regex(self) -> &'static Regex {
        match self {
            Self::RawScript => &RAW_SCRIPT,
            Self::JsonLdArticleType => &JSON_LD_ARTICLE_TYPE,
            Self::UnlikelyCandidates => &UNLIKELY_CANDIDATES,
            Self::MaybeCandidate => &MAYBE_CANDIDATE,
            Self::Positive => &POSITIVE,
            Self::Negative => &NEGATIVE,
            Self::NormalizeWhitespace => &NORMALIZE_WHITESPACE,
            Self::Comma => &COMMA,
            Self::ShareElements => &SHARE_ELEMENTS,
            Self::AdOrLoadingWords => &AD_OR_LOADING_WORDS,
            Self::LazyImageUrl => &LAZY_IMAGE_URL,
            Self::LazyImageSrcset => &LAZY_IMAGE_SRCSET,
            Self::TrailingChromeAttrs => &TRAILING_CHROME_ATTRS,
            Self::TrailingChromeText => &TRAILING_CHROME_TEXT,
            Self::FootnoteReferenceAttrs => &FOOTNOTE_REFERENCE_ATTRS,
            Self::FootnoteReferenceText => &FOOTNOTE_REFERENCE_TEXT,
            Self::LeadingDateText => &LEADING_DATE_TEXT,
            Self::MobileMediaBlock => &MOBILE_MEDIA_BLOCK,
            Self::CssRule => &CSS_RULE,
            Self::DisplayDecl => &DISPLAY_DECL,
            Self::ShadowTemplateHtml => &SHADOW_TEMPLATE_HTML,
            Self::FootnoteTrailingNumber => &FOOTNOTE_TRAILING_NUMBER,
            Self::BylinePrefix => &BYLINE_PREFIX,
            Self::BylineTrailingDate => &BYLINE_TRAILING_DATE,
        }
    }
}

impl From<RegexPattern> for &'static Regex {
    fn from(value: RegexPattern) -> Self {
        value.to_regex()
    }
}

static RAW_SCRIPT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>").expect("valid script regex"));

static JSON_LD_ARTICLE_TYPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^Article|AdvertiserContentArticle|NewsArticle|AnalysisNewsArticle|AskPublicNewsArticle|BackgroundNewsArticle|OpinionNewsArticle|ReportageNewsArticle|ReviewNewsArticle|Report|SatiricalArticle|ScholarlyArticle|MedicalScholarlyArticle|SocialMediaPosting|BlogPosting|LiveBlogPosting|DiscussionForumPosting|TechArticle|APIReference$")
        .expect("valid json-ld article type regex")
});

static UNLIKELY_CANDIDATES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)-ad-|ai2html|banner|breadcrumbs|combx|comment|community|cover-wrap|disqus|extra|footer|gdpr|header|legends|menu|related|remark|replies|rss|shoutbox|sidebar|skyscraper|social|sponsor|supplemental|ad-break|agegate|pagination|pager|popup|yom-remote",
    )
    .expect("valid unlikely-candidates regex")
});

static MAYBE_CANDIDATE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)and|article|body|column|content|main|mathjax|shadow").expect("valid ok-maybe regex"));

static POSITIVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)article|body|content|entry|hentry|h-entry|main|page|pagination|post|text|blog|story")
        .expect("valid positive regex")
});

static NEGATIVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)-ad-|hidden|^hid$| hid$| hid |^hid |banner|combx|comment|com-|contact|footer|gdpr|masthead|media|meta|outbrain|promo|related|scroll|share|shoutbox|sidebar|skyscraper|sponsor|shopping|tags|widget")
        .expect("valid negative regex")
});

static NORMALIZE_WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").expect("valid whitespace regex"));

static COMMA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\u{002C}|\u{060C}|\u{FE50}|\u{FE10}|\u{FE11}|\u{2E41}|\u{2E34}|\u{2E32}|\u{FF0C}")
        .expect("valid comma regex")
});

static SHARE_ELEMENTS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(\b|_)(share|sharedaddy)(\b|_)").expect("valid share regex"));

static AD_OR_LOADING_WORDS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?iu)^(ad(vertising|vertisement)?|pub(licité)?|werb(ung)?|广告|Реклама|Anuncio|(loading|正在加载|Загрузка|chargement|cargando)(…|\.\.\.)?)$").expect("valid ad/loading regex")
});

static LAZY_IMAGE_URL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*\S+\.(jpg|jpeg|png|webp)(\?\S*)?\s*$").expect("valid image url regex"));

static LAZY_IMAGE_SRCSET: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\.(jpg|jpeg|png|webp)\S*\s+\d").expect("valid image srcset regex"));

static TRAILING_CHROME_ATTRS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \b(
            after[-_]?post|bottom[-_]?of[-_]?article|comment(s|ary)?|comment[-_]?thread|court[-_]?case|
            discussion|disqus|finance|follow[-_]?up|job(s)?|keep[-_]?reading|
            mortgage|most[-_]?popular|most[-_]?read|most[-_]?viewed|newsletter|next[-_]?article|next[-_]?up|
            onward[-_]?journey|outbrain|partner[-_]?offer|popular|promo|
            read[-_]?also|read[-_]?more|read[-_]?next|recommend(ed|ation|ations)?|
            recirc|related|signup|sign[-_]?up|sponsor(ed)?|subscribe|
            subscription|taboola|widget|yarpp
        )\b",
    )
    .expect("valid trailing chrome attribute regex")
});

static TRAILING_CHROME_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^\s*(comments?|join the (conversation|discussion)|related( articles| posts| stories)?|also in\b|court case:|affiliate:|continue reading|explore press release|more (from|in|on)|recommended|most (popular|read|viewed)|next article|read (also|more|next)|sponsored|partner offers?|(the )?\w*\s*newsletter|sign up|subscribe|jobs?|mortgage|finance)",
    )
    .expect("valid trailing chrome text regex")
});

static FOOTNOTE_REFERENCE_ATTRS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(footnotes?|endnotes?|references?|bibliography|citations?)\b")
        .expect("valid footnote reference attribute regex")
});

static FOOTNOTE_REFERENCE_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^\s*(footnotes?|notes|references|bibliography|citations?)\s*$")
        .expect("valid footnote reference text regex")
});

static LEADING_DATE_TEXT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)^\s*
        (
            (jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}
            |
            \d{1,2}\s+(jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{4}
            |
            \d{4}[-/]\d{1,2}[-/]\d{1,2}
        )
        \s*$",
    )
    .expect("valid leading date text regex")
});

static MOBILE_MEDIA_BLOCK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)@media[^{]*max-width\s*:\s*(\d+)px[^{]*\{").expect("valid media regex"));

static CSS_RULE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)(?P<selectors>[^{}]+)\{(?P<body>[^{}]*display\s*:\s*[^;}]+[^{}]*)\}")
        .expect("valid css rule regex")
});

static DISPLAY_DECL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)display\s*:\s*(?P<display>[a-z-]+)").expect("valid display regex"));

static SHADOW_TEMPLATE_HTML: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?is)<template\s+[^>]*(?:shadowrootmode|shadowroot)\s*=\s*["']?[^"'\s>]+["']?[^>]*>(?P<body>.*?)</template>"#)
        .expect("valid shadow template regex")
});

static FOOTNOTE_TRAILING_NUMBER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:^|[-_:])(?:fn|ftnt|note|ref)?(\d+)$|(\d+)$").expect("valid footnote label regex"));

static BYLINE_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*(by|author|authors|written by)\s*:?\s+").expect("valid byline prefix regex"));

static BYLINE_TRAILING_DATE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        \s+
        (
            (published|updated|last\s+updated|posted|on)\b.*$
            |
            (jan|feb|mar|apr|may|jun|jul|aug|sep|sept|oct|nov|dec)[a-z]*\.?\s+\d{1,2},?\s+\d{4}.*$
            |
            \d{4}[-/]\d{1,2}[-/]\d{1,2}.*$
        )
        ",
    )
    .expect("valid byline trailing date regex")
});
