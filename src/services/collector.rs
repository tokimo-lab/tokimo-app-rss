use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, FixedOffset};
use feed_rs::parser;
use futures_util::StreamExt;
use reqwest::{StatusCode, header};
use sea_orm::{DatabaseConnection, TransactionTrait};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::{
    AppError,
    db::{
        entities::sources,
        repos::{
            deliveries_repo::DeliveriesRepo,
            entries_repo::{EntriesRepo, NewEntry},
            rules_repo::RulesRepo,
            sources_repo::SourcesRepo,
        },
    },
    services::matcher,
};

pub const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;
const MAX_ENTRIES: usize = 5_000;

#[derive(Clone, Debug)]
pub struct ParsedFeed {
    pub title: String,
    pub entries: Vec<NewEntry>,
    pub categories: Vec<String>,
    pub ttl_minutes: Option<u32>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Clone, Debug)]
pub enum FetchOutcome {
    NotModified,
    Feed(ParsedFeed),
}

#[derive(Debug)]
pub struct FetchError {
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for FetchError {}

pub fn normalize_url(raw: &str) -> Result<String, AppError> {
    let mut url = Url::parse(raw.trim()).map_err(|_| AppError::bad_request("invalid feed URL"))?;
    validate_url_shape(&url).map_err(|e| AppError::bad_request(e.message))?;
    url.set_fragment(None);
    Ok(url.to_string())
}

pub async fn fetch(url: &str, etag: Option<&str>, last_modified: Option<&str>) -> Result<FetchOutcome, FetchError> {
    tokio::time::timeout(Duration::from_secs(15), fetch_with_redirects(url, etag, last_modified))
        .await
        .map_err(|_| fetch_error("feed request exceeded 15 seconds"))?
}

async fn fetch_with_redirects(
    url: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<FetchOutcome, FetchError> {
    let mut current = Url::parse(url).map_err(|_| fetch_error("invalid feed URL"))?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| fetch_error(format!("HTTP client: {e}")))?;
    for redirect_count in 0..=MAX_REDIRECTS {
        validate_url_shape(&current)?;
        let mut request = client
            .get(current.clone())
            .header(
                header::USER_AGENT,
                "TokimoRSS/0.1 (+https://github.com/tokimo-lab/tokimo-app-rss)",
            )
            .header(
                header::ACCEPT,
                "application/rss+xml, application/atom+xml, application/xml, text/xml",
            );
        if let Some(value) = etag {
            request = request.header(header::IF_NONE_MATCH, value);
        }
        if let Some(value) = last_modified {
            request = request.header(header::IF_MODIFIED_SINCE, value);
        }
        let response = request
            .send()
            .await
            .map_err(|e| fetch_error(format!("feed request failed: {e}")))?;
        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(FetchOutcome::NotModified);
        }
        if response.status().is_redirection() {
            if redirect_count == MAX_REDIRECTS {
                return Err(fetch_error("too many redirects"));
            }
            let location = response
                .headers()
                .get(header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| fetch_error("redirect missing Location"))?;
            current = current
                .join(location)
                .map_err(|_| fetch_error("invalid redirect URL"))?;
            continue;
        }
        if !response.status().is_success() {
            let retry_after_seconds = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_retry_after);
            return Err(FetchError {
                message: format!("feed returned HTTP {}", response.status()),
                retry_after_seconds,
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_BODY_BYTES as u64)
        {
            return Err(fetch_error("feed response exceeds 5 MiB"));
        }
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let last_modified = response
            .headers()
            .get(header::LAST_MODIFIED)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let mut stream = response.bytes_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| fetch_error(format!("feed body: {e}")))?;
            if body.len().saturating_add(chunk.len()) > MAX_BODY_BYTES {
                return Err(fetch_error("feed response exceeds 5 MiB"));
            }
            body.extend_from_slice(&chunk);
        }
        return parse(&body, etag, last_modified).map(FetchOutcome::Feed);
    }
    Err(fetch_error("redirect loop"))
}

pub fn parse(body: &[u8], etag: Option<String>, last_modified: Option<String>) -> Result<ParsedFeed, FetchError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(fetch_error("feed response exceeds 5 MiB"));
    }
    let feed = parser::parse(body).map_err(|e| fetch_error(format!("invalid RSS/Atom feed: {e}")))?;
    let mut all_categories = HashSet::new();
    let mut entries = Vec::with_capacity(feed.entries.len().min(MAX_ENTRIES));
    for entry in feed.entries.into_iter().take(MAX_ENTRIES) {
        let title = clean_text(
            entry.title.as_ref().map(|value| value.content.as_str()).unwrap_or(""),
            1_000,
        );
        let summary = entry
            .summary
            .as_ref()
            .map(|value| clean_text(&value.content, 20_000))
            .filter(|value| !value.is_empty());
        let url = entry
            .links
            .iter()
            .find(|link| link.rel.as_deref().is_none_or(|rel| rel == "alternate"))
            .or_else(|| entry.links.first())
            .map(|link| link.href.clone())
            .filter(|value| Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https")))
            .unwrap_or_default();
        let published_at = entry.published.or(entry.updated).map(|value| value.fixed_offset());
        let external_id = stable_external_id(&entry.id, &url, &title, published_at);
        let categories = entry
            .categories
            .iter()
            .map(|category| clean_text(&category.term, 200))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        all_categories.extend(categories.iter().cloned());
        entries.push(NewEntry {
            external_id,
            url,
            normalized_title: matcher::normalize(&title),
            normalized_summary: summary.as_deref().map(matcher::normalize),
            title,
            summary,
            categories,
            author: entry
                .authors
                .first()
                .map(|person| clean_text(&person.name, 500))
                .filter(|value| !value.is_empty()),
            published_at,
        });
    }
    let mut categories = all_categories.into_iter().collect::<Vec<_>>();
    categories.sort();
    Ok(ParsedFeed {
        title: clean_text(
            feed.title.as_ref().map(|value| value.content.as_str()).unwrap_or(""),
            200,
        ),
        entries,
        categories,
        ttl_minutes: feed.ttl,
        etag,
        last_modified,
    })
}

pub async fn collect_leased(
    db: &DatabaseConnection,
    source: &sources::Model,
    lease_token: Uuid,
) -> Result<usize, AppError> {
    let outcome = match fetch(&source.url, source.etag.as_deref(), source.last_modified.as_deref()).await {
        Ok(outcome) => outcome,
        Err(error) => {
            SourcesRepo::fail(
                db,
                source,
                lease_token,
                error.message.clone(),
                error.retry_after_seconds,
            )
            .await?;
            return Err(AppError::internal(error.message));
        }
    };
    persist_leased(db, source, lease_token, outcome).await
}

pub async fn persist_leased(
    db: &DatabaseConnection,
    source: &sources::Model,
    lease_token: Uuid,
    outcome: FetchOutcome,
) -> Result<usize, AppError> {
    if matches!(outcome, FetchOutcome::NotModified) {
        if !SourcesRepo::complete(
            db,
            source,
            lease_token,
            source.etag.clone(),
            source.last_modified.clone(),
            source.last_window_ids.clone(),
            source.possible_gap,
        )
        .await?
        {
            return Err(AppError::conflict("source lease expired"));
        }
        return Ok(0);
    }
    let FetchOutcome::Feed(feed) = outcome else {
        unreachable!()
    };
    let txn = db.begin().await?;
    let rules = RulesRepo::active_for_source(&txn, source.user_id, source.id).await?;
    let initial_import = source.initialized_at.is_none();
    let mut inserted_count = 0;
    let mut window_ids = Vec::with_capacity(feed.entries.len());
    for new_entry in feed.entries {
        window_ids.push(new_entry.external_id.clone());
        let (entry, inserted) = EntriesRepo::upsert(&txn, source.id, new_entry).await?;
        if inserted {
            inserted_count += 1;
            if !initial_import {
                let matched = rules
                    .iter()
                    .filter(|rule| matcher::should_notify(rule, &entry))
                    .map(|rule| (rule.id, rule.name.clone()))
                    .collect::<Vec<_>>();
                if !matched.is_empty() {
                    DeliveriesRepo::create_pending(
                        &txn,
                        source.user_id,
                        entry.id,
                        matched.iter().map(|(id, _)| *id).collect(),
                        matched.into_iter().map(|(_, name)| name).collect(),
                    )
                    .await?;
                }
            }
        }
    }
    let previous = source.last_window_ids.iter().collect::<HashSet<_>>();
    let possible_gap = !previous.is_empty() && !window_ids.iter().any(|id| previous.contains(id));
    if !SourcesRepo::complete(
        &txn,
        source,
        lease_token,
        feed.etag,
        feed.last_modified,
        window_ids,
        possible_gap,
    )
    .await?
    {
        return Err(AppError::conflict("source lease expired"));
    }
    txn.commit().await?;
    Ok(inserted_count)
}

pub async fn fail_leased(
    db: &DatabaseConnection,
    source: &sources::Model,
    lease_token: Uuid,
    error: &FetchError,
) -> Result<(), AppError> {
    SourcesRepo::fail(
        db,
        source,
        lease_token,
        error.message.clone(),
        error.retry_after_seconds,
    )
    .await?;
    Ok(())
}

fn validate_url_shape(url: &Url) -> Result<(), FetchError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(fetch_error("only HTTP(S) feeds are supported"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(fetch_error("feed URL credentials are not allowed"));
    }
    Ok(())
}

fn parse_retry_after(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().or_else(|| {
        httpdate::parse_http_date(value)
            .ok()
            .and_then(|time| time.duration_since(std::time::SystemTime::now()).ok())
            .map(|duration| duration.as_secs())
    })
}

fn stable_external_id(id: &str, url: &str, title: &str, published_at: Option<DateTime<FixedOffset>>) -> String {
    if !id.trim().is_empty() {
        return id.trim().chars().take(2_000).collect();
    }
    if !url.is_empty() {
        return url.to_string();
    }
    let material = format!(
        "{}\0{}",
        matcher::normalize(title),
        published_at.map(|value| value.to_rfc3339()).unwrap_or_default()
    );
    format!("fallback:{:x}", Sha256::digest(material.as_bytes()))
}

fn clean_text(value: &str, max_chars: usize) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn fetch_error(message: impl Into<String>) -> FetchError {
    FetchError {
        message: message.into(),
        retry_after_seconds: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_url_validation_allows_network_policy_managed_hosts() {
        for value in [
            "http://198.18.0.1/feed",
            "http://127.0.0.1/feed",
            "http://localhost/feed",
            "https://reader.local/feed",
        ] {
            assert!(normalize_url(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn rejects_credentials_and_non_http() {
        assert!(normalize_url("http://user:pass@example.com/feed").is_err());
        assert!(normalize_url("ftp://example.com/feed").is_err());
        assert!(
            normalize_url("https://example.com/feed#fragment")
                .unwrap()
                .ends_with("/feed")
        );
    }

    #[test]
    fn parses_rss_atom_unicode_and_missing_fields() {
        let rss = r#"<rss version="2.0"><channel><title>中文源</title><ttl>60</ttl><item><guid>a</guid><title>搬瓦工</title><category>交易</category><pubDate>Mon, 08 Sep 2025 10:00:00 GMT</pubDate></item><item><guid>b</guid><title>无摘要</title></item></channel></rss>"#;
        let parsed = parse(rss.as_bytes(), None, None).unwrap();
        assert_eq!(parsed.title, "中文源");
        assert_eq!(parsed.ttl_minutes, Some(60));
        assert_eq!(parsed.entries.len(), 2);
        assert_eq!(parsed.entries[1].summary, None);

        let atom = br#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Atom</title><id>x</id><updated>2025-09-08T10:00:00Z</updated><entry><title>DMIT</title><id>same</id><updated>2025-09-08T10:00:00Z</updated><link href="https://example.com/1"/></entry></feed>"#;
        assert_eq!(parse(atom, None, None).unwrap().entries[0].external_id, "same");
    }

    #[test]
    fn duplicate_ids_remain_identical() {
        let xml = br#"<rss version="2.0"><channel><title>x</title><item><guid>dup</guid><title>a</title></item><item><guid>dup</guid><title>b</title></item></channel></rss>"#;
        let parsed = parse(xml, None, None).unwrap();
        assert_eq!(parsed.entries[0].external_id, parsed.entries[1].external_id);
    }

    #[tokio::test]
    #[ignore = "live NodeSeek network contract"]
    async fn nodeseek_is_fetchable_by_the_actual_rust_client() {
        let FetchOutcome::Feed(feed) = fetch("https://rss.nodeseek.com/", None, None).await.unwrap() else {
            panic!("unconditional request returned 304");
        };
        assert!(!feed.title.is_empty());
        assert!(!feed.entries.is_empty());
    }
}
