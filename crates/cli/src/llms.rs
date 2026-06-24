use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use lectito::ReadabilityOptions;
use reqwest::Url;
use scraper::{Html, Selector};
use serde::Serialize;
use sitemap::reader::{SiteMapEntity, SiteMapReader};

use crate::cli::{LlmsArgs, LlmsCommands, LlmsExpandArgs, LlmsFetchArgs, LlmsGenerateArgs, LlmsParseArgs};
use crate::{echo, fetch};

#[derive(Debug, Serialize)]
pub struct LlmsDocument {
    pub title: String,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub sections: Vec<LlmsSection>,
}

#[derive(Debug, Serialize)]
pub struct LlmsSection {
    pub name: String,
    pub optional: bool,
    pub links: Vec<LlmsLink>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LlmsLink {
    pub title: String,
    pub url: String,
    pub notes: Option<String>,
}

struct LlmsSource {
    text: String,
    base: Option<String>,
}

struct CrawlItem {
    target: String,
    depth: usize,
    sitemap_lastmod: Option<String>,
}

impl CrawlItem {
    fn new(target: String, depth: usize) -> Self {
        Self { target, depth, sitemap_lastmod: None }
    }
}

struct CrawlPage {
    id: String,
    html: String,
    base_url: Option<String>,
    last_modified: Option<String>,
}

struct CrawledEntry {
    title: String,
    url: String,
    notes: Option<String>,
    last_modified: Option<String>,
    markdown: String,
    source_index: usize,
    rank_score: i32,
}

struct SitemapUrl {
    url: String,
    lastmod: Option<String>,
}

struct GenerateFilters {
    rules: Vec<FilterRule>,
}

impl GenerateFilters {
    fn new(args: &LlmsGenerateArgs) -> Result<Self> {
        Ok(Self { rules: parse_filter_rules(&args.filters)? })
    }
}

struct FilterRule {
    include: bool,
    pattern: String,
    kind: FilterRuleKind,
}

enum FilterRuleKind {
    PathPrefix,
    PathGlob,
    TargetGlob,
}

struct FetchThrottle {
    delay: Duration,
    last_fetch: Option<Instant>,
}

impl FetchThrottle {
    fn new(delay_ms: u64) -> Self {
        Self { delay: Duration::from_millis(delay_ms), last_fetch: None }
    }

    fn wait(&mut self) {
        if self.delay.is_zero() {
            self.last_fetch = Some(Instant::now());
            return;
        }

        if let Some(last_fetch) = self.last_fetch {
            let elapsed = last_fetch.elapsed();
            if elapsed < self.delay {
                std::thread::sleep(self.delay - elapsed);
            }
        }
        self.last_fetch = Some(Instant::now());
    }
}

struct RobotsCache {
    user_agent: String,
    ignore: bool,
    origins: HashMap<String, Option<RobotsRules>>,
}

impl RobotsCache {
    fn new(user_agent: &str, ignore: bool) -> Self {
        Self { user_agent: user_agent.to_string(), ignore, origins: HashMap::new() }
    }

    fn allowed(&mut self, target: &str) -> bool {
        if self.ignore {
            return true;
        }
        let Some(origin) = robots_origin(target) else {
            return true;
        };

        if !self.origins.contains_key(&origin) {
            let rules = read_robots_txt(&origin)
                .ok()
                .and_then(|text| RobotsRules::parse(&text, &self.user_agent));
            self.origins.insert(origin.clone(), rules);
        }

        self.origins
            .get(&origin)
            .and_then(|rules| rules.as_ref())
            .is_none_or(|rules| rules.allowed(target))
    }
}

#[derive(Debug)]
struct RobotsRules {
    rules: Vec<RobotsRule>,
}

#[derive(Debug)]
struct RobotsRule {
    allow: bool,
    pattern: String,
}

impl RobotsRules {
    fn parse(text: &str, user_agent: &str) -> Option<Self> {
        let groups = parse_robots_groups(text);
        let user_agent = user_agent.to_ascii_lowercase();
        let best_len = groups
            .iter()
            .filter(|group| group.matches(&user_agent))
            .flat_map(|group| group.agents.iter())
            .filter(|agent| agent_matches(agent, &user_agent))
            .map(|agent| if agent == "*" { 0 } else { agent.len() })
            .max()?;
        let rules = groups
            .into_iter()
            .filter(|group| group.matches(&user_agent) && group.best_match_len(&user_agent) == Some(best_len))
            .flat_map(|group| group.rules)
            .collect::<Vec<_>>();
        Some(Self { rules })
    }

    fn allowed(&self, target: &str) -> bool {
        let path = match Url::parse(target) {
            Ok(url) => {
                let mut path = url.path().to_string();
                if let Some(query) = url.query() {
                    path.push('?');
                    path.push_str(query);
                }
                path
            }
            Err(_) => target.to_string(),
        };
        let mut best: Option<(usize, bool)> = None;

        for rule in &self.rules {
            if rule.pattern.is_empty() || !robots_pattern_match(&rule.pattern, &path) {
                continue;
            }
            let len: usize = rule.pattern.chars().filter(|ch| !matches!(ch, '*' | '$')).count();
            match best {
                Some((best_len, best_allow)) if best_len > len || (best_len == len && best_allow) => {}
                _ => best = Some((len, rule.allow)),
            }
        }

        best.map(|(_, allow)| allow).unwrap_or(true)
    }
}

struct RobotsGroup {
    agents: Vec<String>,
    rules: Vec<RobotsRule>,
}

impl RobotsGroup {
    fn matches(&self, user_agent: &str) -> bool {
        self.agents.iter().any(|agent| agent_matches(agent, user_agent))
    }

    fn best_match_len(&self, user_agent: &str) -> Option<usize> {
        self.agents
            .iter()
            .filter(|agent| agent_matches(agent, user_agent))
            .map(|agent| if agent == "*" { 0 } else { agent.len() })
            .max()
    }
}

pub fn run(args: LlmsArgs) -> Result<ExitCode> {
    match args.command {
        LlmsCommands::Fetch(args) => run_fetch(args),
        LlmsCommands::Parse(args) => run_parse(args),
        LlmsCommands::Expand(args) => run_expand(args),
        LlmsCommands::Generate(args) => run_generate(args),
    }
}

pub fn parse_llms_txt(text: &str) -> Result<LlmsDocument> {
    let normalized = text
        .trim_start_matches('\u{feff}')
        .replace(" ## ", "\n## ")
        .replace(" # ", "\n# ")
        .replace(" > ", "\n> ")
        .replace(" - [", "\n- [");
    let mut title = None;
    let mut summary = Vec::new();
    let mut details = Vec::new();
    let mut sections = Vec::new();
    let mut current: Option<LlmsSection> = None;
    let mut before_sections = true;

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(value) = line.strip_prefix("# ").filter(|_| title.is_none()) {
            title = Some(value.trim().to_string());
            continue;
        }

        if let Some(value) = line.strip_prefix("## ") {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            let name = value.trim().to_string();
            current = Some(LlmsSection { optional: name.eq_ignore_ascii_case("optional"), name, links: Vec::new() });
            before_sections = false;
            continue;
        }

        if let Some(section) = current.as_mut() {
            if let Some(link) = parse_link_line(line) {
                section.links.push(link);
            }
            continue;
        }

        if before_sections && line.starts_with('>') {
            summary.push(line.trim_start_matches('>').trim().to_string());
        } else if before_sections {
            details.push(line.to_string());
        }
    }

    if let Some(section) = current {
        sections.push(section);
    }

    let title = title
        .filter(|title| !title.is_empty())
        .ok_or_else(|| anyhow::anyhow!("llms.txt is missing an H1 title"))?;
    Ok(LlmsDocument { title, summary: non_empty_join(summary), details: non_empty_join(details), sections })
}

fn run_fetch(args: LlmsFetchArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    write_output(args.output.as_ref(), &source.text)?;
    Ok(ExitCode::SUCCESS)
}

fn run_parse(args: LlmsParseArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    let document = parse_llms_txt(&source.text)?;
    let output = if args.pretty {
        serde_json::to_string_pretty(&document).context("failed to serialize llms.txt JSON")?
    } else {
        serde_json::to_string(&document).context("failed to serialize llms.txt JSON")?
    };
    println!("{output}");
    Ok(ExitCode::SUCCESS)
}

fn run_expand(args: LlmsExpandArgs) -> Result<ExitCode> {
    let source = read_llms_source(&args.input)?;
    let document = parse_llms_txt(&source.text)?;
    let links = selected_links(&document, args.include_optional, args.max_links);
    let mut output = String::new();

    output.push_str("# ");
    output.push_str(&document.title);
    output.push_str("\n\n");
    if let Some(summary) = document.summary.as_deref() {
        output.push_str("> ");
        output.push_str(summary);
        output.push_str("\n\n");
    }
    if let Some(details) = document.details.as_deref() {
        output.push_str(details);
        output.push_str("\n\n");
    }

    for link in links {
        let resolved = resolve_link(&link.url, source.base.as_deref())?;
        let content =
            read_resource_md(&resolved, args.timeout).with_context(|| format!("failed to expand {}", link.url))?;

        output.push_str("---\n\n");
        output.push_str("# Source: ");
        output.push_str(&link.title);
        output.push('\n');
        output.push_str("URL: ");
        output.push_str(&resolved);
        if let Some(notes) = link.notes.as_deref() {
            output.push('\n');
            output.push_str("Notes: ");
            output.push_str(notes);
        }
        output.push_str("\n\n");
        output.push_str(content.trim());
        output.push_str("\n\n");
    }

    write_output(args.output.as_ref(), &output)?;
    Ok(ExitCode::SUCCESS)
}

fn run_generate(args: LlmsGenerateArgs) -> Result<ExitCode> {
    if args.max_pages == 0 {
        anyhow::bail!("--max-pages must be greater than zero");
    }
    if args.max_sitemaps == 0 {
        anyhow::bail!("--max-sitemaps must be greater than zero");
    }
    validate_filters(&args.filters, "filter")?;
    if args.robots_user_agent.trim().is_empty() {
        anyhow::bail!("--robots-agent must not be empty");
    }

    let entries = generate_entries(&args)?;
    let title = args
        .title
        .clone()
        .or_else(|| entries.first().map(|entry| entry.title.clone()))
        .unwrap_or_else(|| "Site documentation".to_string());
    let source = args.sitemap.as_deref().or(args.input.as_deref()).unwrap_or("sitemap");
    let summary = args
        .summary
        .clone()
        .unwrap_or_else(|| format!("Readable pages discovered from {}.", source.trim_end_matches('/')));
    let output = render_generated_llms_txt(&title, &summary, &args.section, &entries);

    write_output(args.output.as_ref(), &output)?;
    if let Some(path) = args.full_output.as_ref() {
        write_output(Some(path), &render_generated_full_context(&title, &summary, &entries))?;
    }
    Ok(if entries.is_empty() { ExitCode::from(1) } else { ExitCode::SUCCESS })
}

fn generate_entries(args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    match (args.input.as_deref(), args.sitemap.as_deref(), args.discover_sitemap) {
        (Some(_), Some(_), _) => anyhow::bail!("pass either a crawl seed or --sitemap, not both"),
        (_, Some(_), true) => anyhow::bail!("cannot combine --sitemap with --discover"),
        (Some(input), None, true) => discovered_sitemap_entries(input, args),
        (Some(_), None, false) => crawl_entries(args),
        (None, Some(sitemap), false) => sitemap_entries(sitemap, args),
        (None, None, true) => anyhow::bail!("pass a seed URL when using --discover"),
        (None, None, false) => anyhow::bail!("pass a seed URL/path or --sitemap"),
    }
}

fn crawl_entries(args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let input = args.input.as_deref().context("missing crawl seed")?;
    let seed = if input.starts_with("http://") || input.starts_with("https://") {
        let mut url = Url::parse(input).with_context(|| format!("invalid URL: {input}"))?;
        url.set_fragment(None);
        url.to_string()
    } else {
        PathBuf::from(input).to_string_lossy().to_string()
    };
    let mut queue = VecDeque::from([CrawlItem::new(seed.clone(), 0)]);
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut throttle = FetchThrottle::new(args.delay_ms);
    let filters = GenerateFilters::new(args)?;
    let mut robots = RobotsCache::new(&args.robots_user_agent, args.ignore_robots);

    while let Some(item) = queue.pop_front() {
        if seen.len() >= args.max_pages {
            break;
        }
        if !seen.insert(item.target.clone()) {
            continue;
        }

        if !robots.allowed(&item.target) {
            eprintln!("lectito: skipping {}: disallowed by robots.txt", item.target);
            continue;
        }

        throttle.wait();
        let page = match read_crawl_page(&item.target) {
            Ok(page) => page,
            Err(error) => {
                eprintln!("lectito: skipping {}: {error:#}", item.target);
                continue;
            }
        };

        if let Some(entry) = crawled_entry(&page, args.timeout, entries.len(), item.sitemap_lastmod.as_deref())? {
            entries.push(entry);
        }

        if item.depth >= args.max_depth {
            continue;
        }

        for link in discover_links(&page, &seed) {
            if !seen.contains(&link) && passes_filters(&link, &filters) {
                queue.push_back(CrawlItem::new(link, item.depth + 1));
            }
        }
    }

    Ok(ranked_entries(entries))
}

fn sitemap_entries(input: &str, args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let urls = sitemap_urls_from_inputs(vec![input.to_string()], args.max_sitemaps, args.max_pages)?;
    entries_from_urls(urls, args)
}

fn discovered_sitemap_entries(input: &str, args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let sitemaps = discover_sitemaps(input)?;
    let urls = sitemap_urls_from_inputs(sitemaps, args.max_sitemaps, args.max_pages)?;
    entries_from_urls(urls, args)
}

fn entries_from_urls(urls: Vec<SitemapUrl>, args: &LlmsGenerateArgs) -> Result<Vec<CrawledEntry>> {
    let mut entries = Vec::new();
    let mut throttle = FetchThrottle::new(args.delay_ms);
    let filters = GenerateFilters::new(args)?;
    let mut robots = RobotsCache::new(&args.robots_user_agent, args.ignore_robots);

    for candidate in urls.into_iter().take(args.max_pages) {
        if !passes_filters(&candidate.url, &filters) {
            continue;
        }
        if !Url::parse(&candidate.url)
            .ok()
            .is_none_or(|parsed| crawlable_url_path(parsed.path()))
        {
            continue;
        }

        if !robots.allowed(&candidate.url) {
            eprintln!("lectito: skipping {}: disallowed by robots.txt", candidate.url);
            continue;
        }

        throttle.wait();
        let page = match read_crawl_page(&candidate.url) {
            Ok(page) => page,
            Err(error) => {
                eprintln!("lectito: skipping {}: {error:#}", candidate.url);
                continue;
            }
        };

        if let Some(entry) = crawled_entry(&page, args.timeout, entries.len(), candidate.lastmod.as_deref())? {
            entries.push(entry);
        }
    }

    Ok(ranked_entries(entries))
}

fn validate_filters(values: &[String], name: &str) -> Result<()> {
    if values.iter().any(|value| value.is_empty()) {
        anyhow::bail!("--{name} values must not be empty");
    }
    Ok(())
}

fn passes_filters(url: &str, filters: &GenerateFilters) -> bool {
    if filters
        .rules
        .iter()
        .any(|rule| !rule.include && filter_rule_matches(rule, url))
    {
        return false;
    }
    if filters.rules.iter().any(|rule| rule.include)
        && !filters
            .rules
            .iter()
            .any(|rule| rule.include && filter_rule_matches(rule, url))
    {
        return false;
    }
    true
}

fn parse_filter_rules(values: &[String]) -> Result<Vec<FilterRule>> {
    values
        .iter()
        .map(|value| {
            let (include, pattern) = value
                .strip_prefix('!')
                .map(|pattern| (false, pattern))
                .unwrap_or((true, value.as_str()));
            if pattern.is_empty() {
                anyhow::bail!("--filter pattern must not be empty");
            }
            let kind = if pattern.starts_with('/') && (pattern.contains('*') || pattern.contains('?')) {
                FilterRuleKind::PathGlob
            } else if pattern.starts_with('/') {
                FilterRuleKind::PathPrefix
            } else {
                FilterRuleKind::TargetGlob
            };
            Ok(FilterRule { include, pattern: pattern.to_string(), kind })
        })
        .collect()
}

fn filter_rule_matches(rule: &FilterRule, target: &str) -> bool {
    match rule.kind {
        FilterRuleKind::PathPrefix => filter_path(target).starts_with(&rule.pattern),
        FilterRuleKind::PathGlob => wildcard_match(&rule.pattern, &filter_path(target)),
        FilterRuleKind::TargetGlob => wildcard_match(&rule.pattern, target),
    }
}

fn filter_path(value: &str) -> String {
    if let Ok(url) = Url::parse(value)
        && matches!(url.scheme(), "http" | "https" | "file")
    {
        return url.path().to_string();
    }

    value.to_string()
}

fn parse_robots_groups(text: &str) -> Vec<RobotsGroup> {
    let mut groups = Vec::new();
    let mut agents = Vec::new();
    let mut rules = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            push_robots_group(&mut groups, &mut agents, &mut rules);
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "user-agent" => {
                if !rules.is_empty() {
                    push_robots_group(&mut groups, &mut agents, &mut rules);
                }
                agents.push(value.to_ascii_lowercase());
            }
            "allow" if !agents.is_empty() => rules.push(RobotsRule { allow: true, pattern: value.to_string() }),
            "disallow" if !agents.is_empty() && !value.is_empty() => {
                rules.push(RobotsRule { allow: false, pattern: value.to_string() });
            }
            _ => {}
        }
    }
    push_robots_group(&mut groups, &mut agents, &mut rules);
    groups
}

fn push_robots_group(groups: &mut Vec<RobotsGroup>, agents: &mut Vec<String>, rules: &mut Vec<RobotsRule>) {
    if agents.is_empty() {
        rules.clear();
        return;
    }
    groups.push(RobotsGroup { agents: std::mem::take(agents), rules: std::mem::take(rules) });
}

fn agent_matches(agent: &str, user_agent: &str) -> bool {
    agent == "*" || user_agent.contains(agent)
}

fn robots_origin(target: &str) -> Option<String> {
    let url = Url::parse(target).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    Some(format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str()?,
        url.port().map(|port| format!(":{port}")).unwrap_or_default()
    ))
}

fn read_robots_txt(origin: &str) -> Result<String> {
    let url = format!("{}/robots.txt", origin.trim_end_matches('/'));
    let document = fetch::InputDocument::read_src(Some(&url), false, None)?;
    Ok(document.html().to_string())
}

fn robots_pattern_match(pattern: &str, path: &str) -> bool {
    match pattern.strip_suffix('$') {
        Some(prefix) => wildcard_match(prefix, path),
        None => wildcard_prefix_match(pattern, path),
    }
}

fn wildcard_prefix_match(pattern: &str, value: &str) -> bool {
    (0..=value.len()).any(|index| value.is_char_boundary(index) && wildcard_match(pattern, &value[..index]))
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let mut p = 0;
    let mut v = 0;
    let mut star = None;
    let mut star_value = 0;

    while v < value.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == value[v]) {
            p += 1;
            v += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            star_value = v;
        } else if let Some(star_index) = star {
            p = star_index + 1;
            star_value += 1;
            v = star_value;
        } else {
            return false;
        }
    }

    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }

    p == pattern.len()
}

fn sitemap_urls_from_inputs(inputs: Vec<String>, max_sitemaps: usize, max_urls: usize) -> Result<Vec<SitemapUrl>> {
    let mut sitemap_queue = inputs
        .into_iter()
        .map(|input| {
            let origin = sitemap_origin(&input);
            (input, origin)
        })
        .collect::<VecDeque<_>>();
    let mut seen_sitemaps = HashSet::new();
    let mut seen_urls = HashSet::new();
    let mut urls = Vec::new();

    while let Some((sitemap, origin)) = sitemap_queue.pop_front() {
        if seen_sitemaps.len() >= max_sitemaps || urls.len() >= max_urls {
            break;
        }
        if !seen_sitemaps.insert(sitemap.clone()) {
            continue;
        }

        let source = read_text_source(&sitemap).with_context(|| format!("failed to read sitemap {sitemap}"))?;
        for entity in SiteMapReader::new(source.text.as_bytes()) {
            match entity {
                SiteMapEntity::Url(entry) => {
                    let Some(url) = entry.loc.get_url().map(|url| url.to_string()) else {
                        continue;
                    };
                    if origin.as_deref().is_some_and(|origin| !same_origin(origin, &url)) {
                        continue;
                    }
                    if seen_urls.insert(url.clone()) {
                        urls.push(SitemapUrl { url, lastmod: entry.lastmod.get_time().map(|time| time.to_rfc3339()) });
                        if urls.len() >= max_urls {
                            break;
                        }
                    }
                }
                SiteMapEntity::SiteMap(entry) => {
                    let Some(url) = entry.loc.get_url().map(|url| url.to_string()) else {
                        continue;
                    };
                    if origin.as_deref().is_some_and(|origin| !same_origin(origin, &url)) {
                        continue;
                    }
                    if seen_sitemaps.len() + sitemap_queue.len() < max_sitemaps {
                        sitemap_queue.push_back((url, origin.clone()));
                    }
                }
                SiteMapEntity::Err(error) => {
                    eprintln!("lectito: sitemap parse error in {sitemap}: {error}");
                }
            }
        }
    }

    Ok(urls)
}

fn discover_sitemaps(input: &str) -> Result<Vec<String>> {
    let origin = robots_origin(input).ok_or_else(|| anyhow::anyhow!("--discover requires an HTTP URL seed"))?;
    let discovered = read_robots_txt(&origin)
        .map(|text| sitemap_locations_from_robots(&text, &origin))
        .unwrap_or_default();
    if !discovered.is_empty() {
        return Ok(discovered);
    }

    Ok(vec![format!("{}/sitemap.xml", origin.trim_end_matches('/'))])
}

fn sitemap_locations_from_robots(text: &str, origin: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut locations = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("sitemap") {
            continue;
        }
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let location = Url::parse(value)
            .or_else(|_| Url::parse(origin).and_then(|origin| origin.join(value)))
            .map(|url| url.to_string());
        if let Ok(location) = location
            && seen.insert(location.clone())
        {
            locations.push(location);
        }
    }
    locations
}

fn read_text_source(input: &str) -> Result<LlmsSource> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_src(Some(input), false, None)?;
        return Ok(LlmsSource { text: document.html().to_string(), base: document.base_url().map(str::to_string) });
    }

    let path = Path::new(input);
    let text = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let base = path.parent().map(|path| path.to_string_lossy().to_string());
    Ok(LlmsSource { text, base })
}

fn sitemap_origin(input: &str) -> Option<String> {
    let url = Url::parse(input).ok()?;
    Some(format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str()?,
        url.port().map(|port| format!(":{port}")).unwrap_or_default()
    ))
}

fn read_crawl_page(input: &str) -> Result<CrawlPage> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_src(Some(input), false, None)?;
        let base_url = document.base_url().map(str::to_string);
        let id = base_url.clone().unwrap_or_else(|| input.to_string());
        let last_modified = document.last_modified().map(str::to_string);
        return Ok(CrawlPage { id, html: document.html().to_string(), base_url, last_modified });
    }

    if input.starts_with("file://") {
        let url = Url::parse(input).with_context(|| format!("invalid file URL: {input}"))?;
        let path = url
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("file URL cannot be converted to a local path: {input}"))?;
        let html = fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
        return Ok(CrawlPage { id: input.to_string(), html, base_url: None, last_modified: None });
    }

    let path = Path::new(input);
    let html = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(CrawlPage { id: path.to_string_lossy().to_string(), html, base_url: None, last_modified: None })
}

fn crawled_entry(
    page: &CrawlPage, timeout: u64, source_index: usize, sitemap_lastmod: Option<&str>,
) -> Result<Option<CrawledEntry>> {
    let options = ReadabilityOptions::default();
    let Some(report) = super::extract_with_timeout(&page.html, page.base_url.as_deref(), options, timeout)? else {
        eprintln!("lectito: extraction timed out for {}", page.id);
        return Ok(None);
    };
    let Some(article) = report.article else {
        return Ok(None);
    };

    let title = article
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or(page.id.as_str())
        .trim()
        .to_string();
    let last_modified = page.last_modified.as_deref().or(sitemap_lastmod).map(str::to_string);
    let notes = entry_notes(article.excerpt.as_deref(), last_modified.as_deref());
    let url = canonical_page_url(&page.html, page.base_url.as_deref()).unwrap_or_else(|| page.id.clone());
    let markdown = echo::render_article(
        Some(&article),
        echo::RenderOptions::new(
            crate::cli::OutputFormat::Markdown,
            false,
            page.base_url.as_deref(),
            false,
        ),
    )?;
    let rank_score = entry_rank_score(&title, &url, &notes, last_modified.as_deref());

    Ok(Some(CrawledEntry {
        title,
        url,
        notes,
        last_modified,
        markdown,
        source_index,
        rank_score,
    }))
}

fn canonical_page_url(html: &str, base_url: Option<&str>) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("link[rel][href]").expect("valid canonical selector");

    document.select(&selector).find_map(|link| {
        let rel = link.value().attr("rel")?;
        if !rel
            .split_whitespace()
            .any(|value| value.eq_ignore_ascii_case("canonical"))
        {
            return None;
        }
        let href = link.value().attr("href")?.trim();
        if href.is_empty() {
            return None;
        }
        normalized_page_url(href, base_url)
    })
}

fn normalized_page_url(value: &str, base_url: Option<&str>) -> Option<String> {
    let mut url = if let Ok(url) = Url::parse(value) { url } else { Url::parse(base_url?).ok()?.join(value).ok()? };
    url.set_fragment(None);
    Some(url.to_string())
}

fn entry_notes(excerpt: Option<&str>, last_modified: Option<&str>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(last_modified) = last_modified.and_then(clean_modified_time) {
        parts.push(format!("Updated: {last_modified}."));
    }
    if let Some(excerpt) = excerpt.map(clean_note).filter(|note| !note.is_empty()) {
        parts.push(excerpt);
    }
    (!parts.is_empty()).then(|| clean_note(&parts.join(" ")))
}

fn clean_modified_time(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value.to_string()) }
}

fn ranked_entries(entries: Vec<CrawledEntry>) -> Vec<CrawledEntry> {
    let mut best_by_url = HashMap::<String, CrawledEntry>::new();
    for entry in entries {
        match best_by_url.get(&entry.url) {
            Some(existing) if compare_entries(existing, &entry) != Ordering::Greater => {}
            _ => {
                best_by_url.insert(entry.url.clone(), entry);
            }
        }
    }

    let mut entries = best_by_url.into_values().collect::<Vec<_>>();
    entries.sort_by(compare_entries);
    entries
}

fn compare_entries(left: &CrawledEntry, right: &CrawledEntry) -> Ordering {
    right
        .rank_score
        .cmp(&left.rank_score)
        .then_with(|| right.last_modified.cmp(&left.last_modified))
        .then_with(|| left.source_index.cmp(&right.source_index))
        .then_with(|| left.title.cmp(&right.title))
}

fn entry_rank_score(title: &str, url: &str, notes: &Option<String>, last_modified: Option<&str>) -> i32 {
    let title = title.to_ascii_lowercase();
    let path = Url::parse(url)
        .ok()
        .map(|url| url.path().trim_matches('/').to_ascii_lowercase())
        .unwrap_or_else(|| url.to_ascii_lowercase());
    let segments = path.split('/').filter(|segment| !segment.is_empty()).count() as i32;
    let mut score = 100 - (segments * 4);

    for signal in [
        "quickstart",
        "quick-start",
        "getting-started",
        "guide",
        "docs",
        "reference",
        "api",
    ] {
        if title.contains(signal) || path.contains(signal) {
            score += 10;
        }
    }
    for weak_signal in ["tag", "category", "archive", "page/", "feed", "changelog", "release"] {
        if path.contains(weak_signal) {
            score -= 12;
        }
    }
    if path.is_empty() || path == "docs" || path.ends_with("/index") || path.ends_with("/index.html") {
        score += 16;
    }
    if notes.as_deref().is_some_and(|note| note.len() > 40) {
        score += 4;
    }
    if last_modified.is_some() {
        score += 2;
    }

    score
}

fn discover_links(page: &CrawlPage, seed: &str) -> Vec<String> {
    let document = Html::parse_document(&page.html);
    let selector = Selector::parse("a[href]").expect("valid link selector");
    let mut links = Vec::new();
    let mut seen = HashSet::new();

    for element in document.select(&selector) {
        let Some(href) = element.value().attr("href") else {
            continue;
        };
        let Some(link) = resolve_crawl_link(href, page, seed) else {
            continue;
        };
        if seen.insert(link.clone()) {
            links.push(link);
        }
    }

    links
}

fn resolve_crawl_link(href: &str, page: &CrawlPage, seed: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty()
        || href.starts_with('#')
        || href.starts_with("mailto:")
        || href.starts_with("tel:")
        || href.starts_with("javascript:")
    {
        return None;
    }

    if seed.starts_with("http://") || seed.starts_with("https://") {
        let base = page.base_url.as_deref().unwrap_or(page.id.as_str());
        let resolved = Url::parse(base).ok()?.join(href).ok()?;
        if !same_origin(seed, resolved.as_str()) || !crawlable_url_path(resolved.path()) {
            return None;
        }
        let mut resolved = resolved;
        resolved.set_fragment(None);
        resolved.set_query(None);
        return Some(resolved.to_string());
    }

    if href.starts_with("http://") || href.starts_with("https://") {
        return None;
    }

    let href = href.split('#').next().unwrap_or(href);
    let href = href.split('?').next().unwrap_or(href);
    if href.is_empty() || !crawlable_url_path(href) {
        return None;
    }
    let base = Path::new(&page.id).parent().unwrap_or_else(|| Path::new("."));
    Some(base.join(href).to_string_lossy().to_string())
}

fn same_origin(seed: &str, candidate: &str) -> bool {
    let Ok(seed) = Url::parse(seed) else {
        return false;
    };
    let Ok(candidate) = Url::parse(candidate) else {
        return false;
    };
    seed.scheme() == candidate.scheme()
        && seed.host_str() == candidate.host_str()
        && seed.port_or_known_default() == candidate.port_or_known_default()
}

fn crawlable_url_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    !matches!(
        Path::new(&lower).extension().and_then(|ext| ext.to_str()),
        Some(
            "avif"
                | "css"
                | "gif"
                | "ico"
                | "jpeg"
                | "jpg"
                | "js"
                | "json"
                | "pdf"
                | "png"
                | "svg"
                | "webp"
                | "woff"
                | "woff2"
                | "xml"
                | "zip"
        )
    )
}

fn render_generated_llms_txt(title: &str, summary: &str, section: &str, entries: &[CrawledEntry]) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(&clean_heading(title));
    output.push_str("\n\n> ");
    output.push_str(&clean_note(summary));
    output.push_str("\n\nGenerated by Lectito from crawled readable pages.\n\n## ");
    output.push_str(&clean_heading(section));
    output.push_str("\n\n");

    for entry in entries {
        output.push_str("- [");
        output.push_str(
            &entry
                .title
                .replace('\\', "\\\\")
                .replace('[', "\\[")
                .replace(']', "\\]"),
        );
        output.push_str("](");
        output.push_str(&entry.url.replace(' ', "%20").replace(')', "%29"));
        output.push(')');
        if let Some(notes) = entry.notes.as_deref() {
            output.push_str(": ");
            output.push_str(notes);
        }
        output.push('\n');
    }

    output
}

fn render_generated_full_context(title: &str, summary: &str, entries: &[CrawledEntry]) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(&clean_heading(title));
    output.push_str("\n\n> ");
    output.push_str(&clean_note(summary));
    output.push_str("\n\nGenerated by Lectito from crawled readable pages.\n\n");

    for entry in entries {
        output.push_str("---\n\n# Source: ");
        output.push_str(&clean_heading(&entry.title));
        output.push('\n');
        output.push_str("URL: ");
        output.push_str(&entry.url);
        if let Some(last_modified) = entry.last_modified.as_deref() {
            output.push('\n');
            output.push_str("Updated: ");
            output.push_str(last_modified);
        }
        if let Some(notes) = entry.notes.as_deref() {
            output.push('\n');
            output.push_str("Notes: ");
            output.push_str(notes);
        }
        output.push_str("\n\n");
        output.push_str(entry.markdown.trim());
        output.push_str("\n\n");
    }

    output
}

fn selected_links(document: &LlmsDocument, include_optional: bool, max_links: usize) -> Vec<LlmsLink> {
    document
        .sections
        .iter()
        .filter(|section| include_optional || !section.optional)
        .flat_map(|section| section.links.iter().cloned())
        .take(max_links)
        .collect()
}

fn read_llms_source(input: &str) -> Result<LlmsSource> {
    if input == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text).context("failed to read stdin")?;
        return Ok(LlmsSource { text, base: None });
    }

    if input.starts_with("http://") || input.starts_with("https://") {
        let url = llms_url(input)?;
        let document = fetch::InputDocument::read_src(Some(url.as_str()), false, None)?;
        return Ok(LlmsSource { text: document.html().to_string(), base: Some(url.to_string()) });
    }

    let path = Path::new(input);
    let text = fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let base = path.parent().map(|path| path.to_string_lossy().to_string());
    Ok(LlmsSource { text, base })
}

fn llms_url(input: &str) -> Result<Url> {
    let mut url = Url::parse(input).with_context(|| format!("invalid URL: {input}"))?;
    let path = url.path();
    if path == "/" || path.is_empty() {
        url.set_path("/llms.txt");
    }
    Ok(url)
}

fn read_resource_md(input: &str, timeout: u64) -> Result<String> {
    if input.starts_with("http://") || input.starts_with("https://") {
        let document = fetch::InputDocument::read_src(Some(input), false, None)?;
        if looks_like_md(input, document.content_type(), document.html()) {
            return Ok(document.html().to_string());
        }
        return extract_resource_md(document.html(), document.base_url(), timeout);
    }

    let text = fs::read_to_string(input).with_context(|| format!("failed to read {input}"))?;
    if looks_like_md(input, None, &text) {
        return Ok(text);
    }
    extract_resource_md(&text, None, timeout)
}

fn extract_resource_md(html: &str, base_url: Option<&str>, timeout: u64) -> Result<String> {
    let options = ReadabilityOptions::default();
    let Some(report) = super::extract_with_timeout(html, base_url, options, timeout)? else {
        anyhow::bail!("extraction timed out after {timeout}s");
    };
    echo::render_article(
        report.article.as_ref(),
        echo::RenderOptions::new(crate::cli::OutputFormat::Markdown, false, base_url, false),
    )
}

fn looks_like_md(input: &str, content_type: Option<&str>, text: &str) -> bool {
    if let Some(content_type) = content_type.and_then(|value| value.split(';').next()) {
        let content_type = content_type.trim().to_ascii_lowercase();
        if matches!(content_type.as_str(), "text/markdown" | "text/plain") {
            return true;
        }
        if matches!(content_type.as_str(), "text/html" | "application/xhtml+xml") {
            return false;
        }
    }

    let lower = input.to_ascii_lowercase();
    lower.ends_with(".md")
        || lower.ends_with(".mdx")
        || lower.ends_with(".txt")
        || text.trim_start().starts_with("# ")
        || text.trim_start().starts_with("---\n")
}

fn resolve_link(value: &str, base: Option<&str>) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(value.to_string());
    }

    if let Some(base) = base {
        if base.starts_with("http://") || base.starts_with("https://") {
            return Url::parse(base)
                .and_then(|url| url.join(value))
                .map(|url| url.to_string())
                .with_context(|| format!("failed to resolve {value} against {base}"));
        }

        let path = PathBuf::from(base).join(value);
        return Ok(path.to_string_lossy().to_string());
    }

    Ok(value.to_string())
}

fn write_output(path: Option<&PathBuf>, output: &str) -> Result<()> {
    match path {
        Some(path) => fs::write(path, output).with_context(|| format!("failed to write {}", path.display())),
        None => {
            print!("{output}");
            if !output.ends_with('\n') {
                println!();
            }
            Ok(())
        }
    }
}

fn parse_link_line(line: &str) -> Option<LlmsLink> {
    let line = line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))?.trim();
    let start = line.find('[')?;
    let after_start = &line[start + 1..];
    let title_end = after_start.find("](")?;
    let title = after_start[..title_end].trim();
    let after_title = &after_start[title_end + 2..];
    let url_end = after_title.find(')')?;
    let url = after_title[..url_end].trim();
    let notes = after_title[url_end + 1..]
        .trim()
        .strip_prefix(':')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if title.is_empty() || url.is_empty() {
        return None;
    }

    Some(LlmsLink { title: title.to_string(), url: url.to_string(), notes })
}

fn non_empty_join(values: Vec<String>) -> Option<String> {
    let joined = values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!joined.is_empty()).then_some(joined)
}

fn clean_heading(value: &str) -> String {
    let value = value.replace(['\n', '\r'], " ");
    let value = value.trim().trim_start_matches('#').trim();
    if value.is_empty() { "Untitled".to_string() } else { value.to_string() }
}

fn clean_note(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let limit = 220;
    if collapsed.chars().count() <= limit {
        return collapsed;
    }

    let mut clipped = collapsed.chars().take(limit).collect::<String>();
    if let Some((prefix, _)) = clipped.rsplit_once(' ') {
        clipped = prefix.to_string();
    }
    clipped.push_str("...");
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn parses_sections_and_optional_links() {
        let document = parse_llms_txt(
            r#"# Lectito

> Readability extraction tools.

Use Markdown outputs for agent context.

## Docs

- [Quick start](https://example.com/quick.md): First steps

## Optional

- [Archive](https://example.com/archive.md)
"#,
        )
        .expect("llms.txt should parse");

        assert_eq!(document.title, "Lectito");
        assert_eq!(document.summary.as_deref(), Some("Readability extraction tools."));
        assert_eq!(document.sections.len(), 2);
        assert!(!document.sections[0].optional);
        assert!(document.sections[1].optional);
        assert_eq!(document.sections[0].links[0].notes.as_deref(), Some("First steps"));
    }

    #[test]
    fn parses_minified_common_shape() {
        let document = parse_llms_txt(
            "# Docs > Links point to Markdown. ## Guides - [Start](https://example.com/start.md): Begin ## Optional - [API](https://example.com/api.md)",
        )
        .expect("minified llms.txt should parse");

        assert_eq!(document.title, "Docs");
        assert_eq!(document.sections.len(), 2);
        assert_eq!(document.sections[0].links[0].title, "Start");
        assert!(document.sections[1].optional);
    }

    #[test]
    fn selected_links_skip_optional_by_default() {
        let document = parse_llms_txt("# Docs\n\n## Main\n- [A](a.md)\n\n## Optional\n- [B](b.md)\n")
            .expect("llms.txt should parse");

        let links = selected_links(&document, false, 10);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].title, "A");
    }

    #[test]
    fn discovers_same_origin_html_links_only() {
        let page = CrawlPage {
            id: "https://example.com/docs/index.html".to_string(),
            base_url: Some("https://example.com/docs/index.html".to_string()),
            last_modified: None,
            html: r##"
                <a href="/docs/guide.html#intro">Guide</a>
                <a href="https://other.example/docs/offsite.html">Offsite</a>
                <a href="/assets/site.css">CSS</a>
                <a href="mailto:test@example.com">Mail</a>
            "##
            .to_string(),
        };

        let links = discover_links(&page, "https://example.com/docs/");

        assert_eq!(links, vec!["https://example.com/docs/guide.html"]);
    }

    #[test]
    fn renders_generated_llms_txt() {
        let output = render_generated_llms_txt(
            "Example",
            "Readable pages.",
            "Guides",
            &[CrawledEntry {
                title: "A [guide]".to_string(),
                url: "https://example.com/a guide.html".to_string(),
                notes: Some("Short note.".to_string()),
                last_modified: None,
                markdown: "# A guide\n\nBody.".to_string(),
                source_index: 0,
                rank_score: 0,
            }],
        );

        assert!(output.contains("# Example"));
        assert!(output.contains("## Guides"));
        assert!(output.contains("- [A \\[guide\\]](https://example.com/a%20guide.html): Short note."));
    }

    #[test]
    fn canonical_page_url_resolves_relative_href() {
        let url = canonical_page_url(
            r#"<html><head><link rel="canonical" href="../guide/"></head></html>"#,
            Some("https://example.com/docs/page.html"),
        );

        assert_eq!(url.as_deref(), Some("https://example.com/guide/"));
    }

    #[test]
    fn entry_notes_include_modified_time_before_excerpt() {
        let notes = entry_notes(Some("A concise page summary."), Some("Wed, 01 May 2024 10:00:00 GMT"));

        assert_eq!(
            notes.as_deref(),
            Some("Updated: Wed, 01 May 2024 10:00:00 GMT. A concise page summary.")
        );
    }

    #[test]
    fn ranked_entries_prefers_important_and_recent_pages() {
        let entries = ranked_entries(vec![
            CrawledEntry {
                title: "Archive".to_string(),
                url: "https://example.com/blog/archive/page".to_string(),
                notes: None,
                last_modified: None,
                markdown: "Archive".to_string(),
                source_index: 0,
                rank_score: entry_rank_score("Archive", "https://example.com/blog/archive/page", &None, None),
            },
            CrawledEntry {
                title: "API Reference".to_string(),
                url: "https://example.com/docs/api".to_string(),
                notes: Some("Reference docs for the API.".to_string()),
                last_modified: Some("2025-01-01T00:00:00+00:00".to_string()),
                markdown: "API".to_string(),
                source_index: 1,
                rank_score: entry_rank_score(
                    "API Reference",
                    "https://example.com/docs/api",
                    &Some("Reference docs for the API.".to_string()),
                    Some("2025-01-01T00:00:00+00:00"),
                ),
            },
        ]);

        assert_eq!(entries[0].title, "API Reference");
    }

    #[test]
    fn renders_generated_full_context() {
        let output = render_generated_full_context(
            "Example",
            "Readable pages.",
            &[CrawledEntry {
                title: "Guide".to_string(),
                url: "https://example.com/guide".to_string(),
                notes: Some("Updated: 2025-01-01. Summary.".to_string()),
                last_modified: Some("2025-01-01".to_string()),
                markdown: "# Guide\n\nBody.".to_string(),
                source_index: 0,
                rank_score: 0,
            }],
        );

        assert!(output.contains("# Example"));
        assert!(output.contains("# Source: Guide"));
        assert!(output.contains("URL: https://example.com/guide"));
        assert!(output.contains("# Guide\n\nBody."));
    }

    #[test]
    fn parses_sitemap_urls_from_file() {
        let dir = std::env::temp_dir().join(format!("lectito-sitemap-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let sitemap = dir.join("sitemap.xml");
        std::fs::write(
            &sitemap,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/docs/a.html</loc><lastmod>2024-05-01</lastmod></url>
  <url><loc>https://example.com/docs/b.html</loc></url>
</urlset>
"#,
        )
        .expect("write sitemap");

        let urls = sitemap_urls_from_inputs(vec![sitemap.to_str().expect("utf-8 path").to_string()], 5, 10)
            .expect("parse sitemap");

        assert_eq!(urls[0].url, "https://example.com/docs/a.html");
        assert_eq!(urls[0].lastmod.as_deref(), Some("2024-05-01T00:00:00+00:00"));
        assert_eq!(urls[1].url, "https://example.com/docs/b.html");
        assert_eq!(urls[1].lastmod, None);
    }

    #[test]
    fn content_type_controls_markdown_detection() {
        assert!(looks_like_md(
            "https://example.com/page",
            Some("text/markdown; charset=utf-8"),
            "<p>No</p>"
        ));
        assert!(!looks_like_md("https://example.com/page.md", Some("text/html"), "# No"));
    }

    #[test]
    fn layered_filters_match_url_path_and_globs() {
        let raw_filters = vec!["/docs/reference/".to_string(), "!/docs/reference/archive/".to_string()];
        let filters = GenerateFilters { rules: parse_filter_rules(&raw_filters).expect("filter rules") };

        assert!(!passes_filters("https://example.com/docs/guide", &filters));
        assert!(!passes_filters("https://example.com/blog/post", &filters));
        assert!(passes_filters("https://example.com/docs/reference/page", &filters));
        assert!(!passes_filters(
            "https://example.com/docs/reference/archive/page",
            &filters
        ));
    }

    #[test]
    fn sitemap_locations_from_robots_resolves_absolute_and_relative_urls() {
        let locations = sitemap_locations_from_robots(
            r#"
User-agent: *
Disallow: /private/
Sitemap: https://example.com/sitemap.xml
Sitemap: /docs-sitemap.xml
Sitemap: https://example.com/sitemap.xml
"#,
            "https://example.com",
        );

        assert_eq!(
            locations,
            vec![
                "https://example.com/sitemap.xml",
                "https://example.com/docs-sitemap.xml"
            ]
        );
    }

    #[test]
    fn discovers_sitemaps_from_robots_txt() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let body = format!("User-agent: *\nSitemap: http://{address}/sitemap.xml\n");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).expect("write response");
        });

        let sitemaps = discover_sitemaps(&format!("http://{address}/docs/")).expect("discover sitemaps");

        server.join().expect("join test server");
        assert_eq!(sitemaps, vec![format!("http://{address}/sitemap.xml")]);
    }

    #[test]
    fn robots_rules_use_longest_match_and_allow_wins_ties() {
        let rules = RobotsRules::parse(
            r#"
User-agent: *
Disallow: /

User-agent: Lectito
Disallow: /private/
Allow: /private/public/
Disallow: /tmp$
"#,
            "Lectito",
        )
        .expect("robots rules");

        assert!(rules.allowed("https://example.com/docs/guide"));
        assert!(!rules.allowed("https://example.com/private/secret"));
        assert!(rules.allowed("https://example.com/private/public/page"));
        assert!(!rules.allowed("https://example.com/tmp"));
        assert!(rules.allowed("https://example.com/tmp/file"));
    }

    #[test]
    fn robots_ignore_allows_remote_targets() {
        let mut robots = RobotsCache::new("Lectito", true);

        assert!(robots.allowed("https://example.com/private/page"));
    }
}
