use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokimo_bus_client::BusClient;

use crate::{
    AppError,
    bus_clients::notifications::{self, NotifyRequest},
    db::repos::deliveries_repo::{DeliveriesRepo, DeliveryView},
};

pub async fn dispatch_one(db: &DatabaseConnection, client: &BusClient) -> Result<bool, AppError> {
    let Some((view, token)) = DeliveriesRepo::acquire_due(db).await? else {
        return Ok(false);
    };
    if !view.source.enabled || view.source.archived_at.is_some() || view.active_rule_names.is_empty() {
        DeliveriesRepo::cancel(db, view.delivery.id, view.delivery.user_id, token).await?;
        return Ok(true);
    }
    let request = notification_request(&view);
    match notifications::notify(client, request).await {
        Ok(()) => {
            DeliveriesRepo::accepted(db, view.delivery.id, view.delivery.user_id, token).await?;
        }
        Err(error) => {
            DeliveriesRepo::failed(db, &view.delivery, token, error.message.clone()).await?;
            return Err(error);
        }
    }
    Ok(true)
}

pub fn start(db: DatabaseConnection, client: Arc<BusClient>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            for _ in 0..20 {
                match dispatch_one(&db, &client).await {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(error) => {
                        tracing::warn!(error = %error.message, "rss delivery dispatch failed");
                        break;
                    }
                }
            }
        }
    });
}

fn notification_request(view: &DeliveryView) -> NotifyRequest {
    let mut body = String::new();
    if let Some(summary) = &view.entry.summary
        && !summary.is_empty()
    {
        body.push_str(summary);
        body.push_str("\n\n");
    }
    body.push_str(&format!(
        "来源：{}\n命中规则：{}",
        view.source.name,
        view.active_rule_names.join("、")
    ));
    let action = url::Url::parse(&view.entry.url)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .map(|_| serde_json::json!({ "type": "open-url", "url": view.entry.url }));
    if action.is_some() {
        body.push('\n');
        body.push_str(&view.entry.url);
    }
    NotifyRequest {
        user_id: view.delivery.user_id.to_string(),
        app_id: "rss",
        category_id: "keyword_match",
        category_label: "rss.notifications.keywordMatch",
        title: view.entry.title.clone(),
        body,
        level: "info",
        action,
        dedupe_key: view.delivery.dedupe_key.clone(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;
    use crate::db::entities::{deliveries, entries, sources};

    fn delivery_view(entry_url: &str) -> DeliveryView {
        let now = Utc::now().fixed_offset();
        let user = Uuid::new_v4();
        let entry_id = Uuid::new_v4();
        DeliveryView {
            delivery: deliveries::Model {
                id: Uuid::new_v4(),
                user_id: user,
                entry_id,
                matched_rules: vec![Uuid::new_v4()],
                matched_rule_names: vec!["规则".into()],
                status: "pending".into(),
                attempts: 2,
                next_attempt_at: now,
                last_error: None,
                accepted_at: None,
                lease_token: None,
                lease_until: None,
                dedupe_key: format!("rss:{user}:{entry_id}"),
                created_at: now,
                updated_at: now,
            },
            entry: entries::Model {
                id: entry_id,
                source_id: Uuid::new_v4(),
                external_id: "x".into(),
                url: entry_url.into(),
                title: "标题".into(),
                summary: Some("帖子正文".into()),
                categories: vec![],
                author: None,
                published_at: None,
                first_seen_at: now,
                last_seen_at: now,
                sort_at: now,
                normalized_title: "标题".into(),
                normalized_summary: None,
            },
            source: sources::Model {
                id: Uuid::new_v4(),
                user_id: user,
                name: "源".into(),
                url: "https://example.com/feed".into(),
                normalized_url: "https://example.com/feed".into(),
                enabled: true,
                archived_at: None,
                poll_interval_seconds: 300,
                etag: None,
                last_modified: None,
                last_polled_at: None,
                last_success_at: None,
                next_poll_at: now,
                initialized_at: Some(now),
                last_window_ids: vec![],
                possible_gap: false,
                failure_count: 0,
                last_error: None,
                lease_token: None,
                lease_until: None,
                created_at: now,
                updated_at: now,
            },
            rule_names: vec!["规则".into()],
            active_rule_names: vec!["规则".into()],
        }
    }

    #[test]
    fn retries_keep_stable_dedupe_key() {
        let view = delivery_view("https://example.com/x");
        let first = notification_request(&view);
        let second = notification_request(&view);
        assert_eq!(first.dedupe_key, second.dedupe_key);
    }

    #[test]
    fn valid_entry_url_is_in_action_and_notification_body() {
        let entry_url = "https://example.com/posts/1?from=rss";
        let request = notification_request(&delivery_view(entry_url));

        assert_eq!(
            request.action,
            Some(serde_json::json!({ "type": "open-url", "url": entry_url }))
        );
        assert_eq!(
            request.body,
            format!("帖子正文\n\n来源：源\n命中规则：规则\n{entry_url}")
        );
    }

    #[test]
    fn missing_or_invalid_entry_url_does_not_create_a_link() {
        for entry_url in ["", "not a url", "javascript:alert(1)", "ftp://example.com/post"] {
            let request = notification_request(&delivery_view(entry_url));

            assert_eq!(request.action, None, "unexpected action for {entry_url:?}");
            assert_eq!(request.body, "帖子正文\n\n来源：源\n命中规则：规则");
        }
    }

    #[test]
    fn missing_summary_starts_with_source_metadata() {
        let mut view = delivery_view("");
        view.entry.summary = None;

        let request = notification_request(&view);

        assert_eq!(request.body, "来源：源\n命中规则：规则");
    }
}
