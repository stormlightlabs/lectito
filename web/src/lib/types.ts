export type ExtractFormat = 'html' | 'markdown' | 'text' | 'json';

export type LibrarySort = 'recent' | 'popular' | 'alpha';

export type CachedMetadata = {
	title?: string | null;
	author?: string | null;
	date?: string | null;
	excerpt?: string | null;
	site_name?: string | null;
	language?: string | null;
	word_count?: number | null;
	reading_time_minutes?: number | null;
	image?: string | null;
	favicon?: string | null;
};

export type ExtractResponse = {
	id?: string | null;
	url: string;
	format: ExtractFormat;
	content: string;
	metadata: CachedMetadata;
	cached: boolean;
	extracted_at: string;
};

export type ExtractRequest = {
	url: string;
	format: ExtractFormat;
	include_frontmatter?: boolean;
	include_references?: boolean;
	strip_images?: boolean;
	content_selector?: string | null;
};

export type LibraryItem = {
	id: string;
	url: string;
	domain: string;
	format: ExtractFormat;
	title?: string | null;
	author?: string | null;
	site_name?: string | null;
	favicon?: string | null;
	excerpt?: string | null;
	date?: string | null;
	word_count?: number | null;
	reading_time_minutes?: number | null;
	hit_count: number;
	fetched_at: string;
};

export type TopDomain = { domain: string; count: number };

export type LibraryStats = {
	total_articles: number;
	total_reads: number;
	unique_domains: number;
	total_reading_time_minutes: number;
	top_domains: TopDomain[];
};

export type LibraryResponse = {
	items: LibraryItem[];
	total: number;
	page: number;
	per_page: number;
	stats: LibraryStats;
};

export type LimitsResponse = {
	requests_remaining: number;
	requests_limit: number;
	window_seconds: number;
	reset_at: string;
};

export type RateLimitHeaders = { limit?: number; remaining?: number; reset?: number };
