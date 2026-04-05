-- Migration 001: initial schema

CREATE TABLE IF NOT EXISTS extracted_articles (
    id          UUID PRIMARY KEY,
    url         TEXT NOT NULL,
    url_hash    BYTEA NOT NULL,
    format      TEXT NOT NULL DEFAULT 'markdown',
    content     TEXT NOT NULL,
    metadata    JSONB NOT NULL DEFAULT '{}',
    fetched_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    hit_count   INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_url_hash_format
    ON extracted_articles (url_hash, format);

CREATE INDEX IF NOT EXISTS idx_articles_expires_at
    ON extracted_articles (expires_at);

CREATE TABLE IF NOT EXISTS rate_limits (
    ip              INET NOT NULL,
    window_start    TIMESTAMPTZ NOT NULL,
    window_seconds  INTEGER NOT NULL,
    request_count   INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (ip, window_start, window_seconds)
);

CREATE INDEX IF NOT EXISTS idx_rate_limits_expiry
    ON rate_limits (window_start);

CREATE TABLE IF NOT EXISTS blocked_domains (
    domain      TEXT PRIMARY KEY,
    reason      TEXT,
    blocked_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS ip_bans (
    ip          INET PRIMARY KEY,
    reason      TEXT,
    banned_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ
);
