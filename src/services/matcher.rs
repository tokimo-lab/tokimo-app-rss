use unicode_normalization::UnicodeNormalization;

use crate::db::entities::{entries, rules};

pub fn normalize(value: &str) -> String {
    value.nfkc().flat_map(char::to_lowercase).collect::<String>()
}

pub fn matches(rule: &rules::Model, entry: &entries::Model) -> bool {
    if !rule.enabled {
        return false;
    }
    let normalized_categories = entry
        .categories
        .iter()
        .map(|value| normalize(value))
        .collect::<Vec<_>>();
    let categories_match = rule.categories.is_empty()
        || rule
            .categories
            .iter()
            .map(|value| normalize(value))
            .any(|wanted| normalized_categories.iter().any(|actual| actual == &wanted));
    if !categories_match {
        return false;
    }
    let mut haystack = entry.normalized_title.clone();
    if rule.match_scope == "title_summary"
        && let Some(summary) = &entry.normalized_summary
    {
        haystack.push('\n');
        haystack.push_str(summary);
    }
    let includes = !rule.include_any.is_empty()
        && rule
            .include_any
            .iter()
            .map(|keyword| normalize(keyword))
            .any(|keyword| haystack.contains(&keyword));
    let excludes = rule
        .exclude_any
        .iter()
        .map(|keyword| normalize(keyword))
        .any(|keyword| !keyword.is_empty() && haystack.contains(&keyword));
    includes && !excludes
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn entry(title: &str, summary: Option<&str>, categories: &[&str]) -> entries::Model {
        let now = Utc::now().fixed_offset();
        entries::Model {
            id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            external_id: "1".into(),
            url: "https://example.com/1".into(),
            title: title.into(),
            summary: summary.map(str::to_string),
            categories: categories.iter().map(|value| (*value).to_string()).collect(),
            author: None,
            published_at: None,
            first_seen_at: now,
            last_seen_at: now,
            sort_at: now,
            normalized_title: normalize(title),
            normalized_summary: summary.map(normalize),
        }
    }

    fn rule() -> rules::Model {
        let now = Utc::now().fixed_offset();
        rules::Model {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            source_id: Uuid::new_v4(),
            name: "trade".into(),
            enabled: true,
            categories: vec!["交易".into()],
            include_any: vec!["ＢＷＨ".into(), "搬瓦工".into()],
            exclude_any: vec!["已出".into()],
            match_scope: "title".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn nfkc_case_insensitive_literal_matching() {
        assert!(matches(&rule(), &entry("bwh 新套餐", None, &["交易"])));
        assert!(!matches(&rule(), &entry("BWH 已出", None, &["交易"])));
        assert!(!matches(&rule(), &entry("BWH", None, &["技术"])));
    }

    #[test]
    fn summary_is_only_used_when_selected() {
        let mut candidate = rule();
        let target = entry("普通标题", Some("搬瓦工补货"), &["交易"]);
        assert!(!matches(&candidate, &target));
        candidate.match_scope = "title_summary".into();
        assert!(matches(&candidate, &target));
    }
}
