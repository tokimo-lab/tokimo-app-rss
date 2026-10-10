use std::sync::{Arc, OnceLock};

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use tokimo_bus_auth::TokimoUser;
use tokimo_bus_client::BusClient;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError,
    bus_clients::notifications::{self, NotifyRequest},
    db::{
        entities::{entries, rules, saved_views, sources},
        repos::{
            deliveries_repo::{DeliveriesRepo, DeliveryView},
            entries_repo::{EntriesRepo, EntrySearch},
            rules_repo::{RuleInput, RulePatch, RulesRepo},
            saved_views_repo::{SavedViewInput, SavedViewPatch, SavedViewsRepo},
            sources_repo::SourcesRepo,
        },
    },
    services::{collector, matcher},
};

pub struct AppCtx {
    pub db: DatabaseConnection,
    pub client: Arc<OnceLock<Arc<BusClient>>>,
}

macro_rules! export_dto {
    (#[derive($($derive:ident),+)] $item:item) => {
        #[derive($($derive),+)]
        #[serde(rename_all = "camelCase")]
        #[ts(export, export_to = "../ui/src/generated/rust-types/")]
        $item
    };
}

export_dto! { #[derive(Serialize, TS)] pub struct SourceDto {
    #[ts(type = "string")] pub id: Uuid, pub name: String, pub url: String, pub enabled: bool,
    pub archived_at: Option<String>, pub poll_interval_seconds: i32, pub etag: Option<String>,
    pub last_modified: Option<String>, pub last_polled_at: Option<String>, pub last_success_at: Option<String>,
    pub next_poll_at: String, pub initialized_at: Option<String>, pub failure_count: i32,
    pub last_error: Option<String>, pub gap_suspected: bool, #[ts(type = "number")] pub entry_count: u64,
} }
export_dto! { #[derive(Serialize, TS)] pub struct SourcesListResp { pub sources: Vec<SourceDto> } }
export_dto! { #[derive(Deserialize, TS)] pub struct CreateSourceReq { pub name: String, pub url: String, pub poll_interval_seconds: i32 } }
export_dto! { #[derive(Deserialize, TS)] pub struct PatchSourceReq { pub name: Option<String>, pub poll_interval_seconds: Option<i32>, pub enabled: Option<bool>, pub archived: Option<bool> } }
export_dto! { #[derive(Deserialize, TS)] pub struct TestSourceReq { pub url: String } }
export_dto! { #[derive(Serialize, TS)] pub struct TestSourceResp { pub title: String, pub entry_count: usize, pub categories: Vec<String>, pub ttl_minutes: Option<u32>, pub limited_window: bool } }
export_dto! { #[derive(Serialize, TS)] pub struct RefreshSourceResp { pub status: String } }

export_dto! { #[derive(Clone, Serialize, TS)] pub struct EntryDto {
    #[ts(type = "string")] pub id: Uuid, #[ts(type = "string")] pub source_id: Uuid,
    pub source_name: String, pub external_id: String, pub url: String, pub title: String,
    pub summary: Option<String>, pub categories: Vec<String>, pub author: Option<String>,
    pub published_at: Option<String>, pub first_seen_at: String, pub last_seen_at: String,
} }
export_dto! { #[derive(Serialize, TS)] pub struct EntriesListResp { pub entries: Vec<EntryDto>, pub next_cursor: Option<String> } }

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EntriesQuery {
    pub q: Option<String>,
    pub source_id: Option<Uuid>,
    pub view_id: Option<Uuid>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub published_from: Option<String>,
    pub published_to: Option<String>,
    pub match_scope: Option<String>,
    pub cursor: Option<String>,
}

export_dto! { #[derive(Serialize, TS)] pub struct SavedViewDto {
    #[ts(type = "string")] pub id: Uuid, #[ts(type = "string")] pub source_id: Uuid,
    pub name: String, pub categories: Vec<String>, pub include_any: Vec<String>,
    pub exclude_any: Vec<String>, pub match_scope: String, pub created_at: String, pub updated_at: String,
} }
export_dto! { #[derive(Serialize, TS)] pub struct SavedViewsListResp { pub views: Vec<SavedViewDto> } }
export_dto! { #[derive(Deserialize, TS)] pub struct CreateSavedViewReq {
    #[ts(type = "string")] pub source_id: Uuid, pub name: String, #[serde(default)] pub categories: Vec<String>,
    #[serde(default)] pub include_any: Vec<String>, #[serde(default)] pub exclude_any: Vec<String>, pub match_scope: String,
} }
export_dto! { #[derive(Deserialize, TS)] pub struct PatchSavedViewReq {
    pub name: Option<String>, pub categories: Option<Vec<String>>, pub include_any: Option<Vec<String>>,
    pub exclude_any: Option<Vec<String>>, pub match_scope: Option<String>,
} }

export_dto! { #[derive(Serialize, TS)] pub struct RuleDto {
    #[ts(type = "string")] pub id: Uuid, #[ts(type = "string")] pub source_id: Uuid,
    pub name: String, pub enabled: bool, pub notify_enabled: bool, pub categories: Vec<String>, pub include_any: Vec<String>,
    pub exclude_any: Vec<String>, pub match_scope: String, pub created_at: String, pub updated_at: String,
} }
export_dto! { #[derive(Serialize, TS)] pub struct RulesListResp { pub rules: Vec<RuleDto> } }
export_dto! { #[derive(Deserialize, TS)] pub struct CreateRuleReq {
    #[ts(type = "string")] pub source_id: Uuid, pub name: String, #[serde(default = "default_true")] pub enabled: bool,
    #[serde(default = "default_true")] pub notify_enabled: bool,
    #[serde(default)] pub categories: Vec<String>, pub include_any: Vec<String>, #[serde(default)] pub exclude_any: Vec<String>, pub match_scope: String,
} }
export_dto! { #[derive(Deserialize, TS)] pub struct PatchRuleReq {
    pub name: Option<String>, pub enabled: Option<bool>, pub notify_enabled: Option<bool>, pub categories: Option<Vec<String>>,
    pub include_any: Option<Vec<String>>, pub exclude_any: Option<Vec<String>>, pub match_scope: Option<String>,
} }
export_dto! { #[derive(Deserialize, TS)] pub struct RulePreviewReq {
    #[ts(type = "string")] pub source_id: Uuid, pub name: Option<String>, #[serde(default = "default_true")] pub enabled: bool,
    #[serde(default)] pub categories: Vec<String>, pub include_any: Vec<String>, #[serde(default)] pub exclude_any: Vec<String>,
    pub match_scope: String, pub cursor: Option<String>,
} }
pub type RulePreviewResp = EntriesListResp;

export_dto! { #[derive(Serialize, TS)] pub struct DeliveryDto {
    #[ts(type = "string")] pub id: Uuid, #[ts(type = "string")] pub entry_id: Uuid,
    pub entry_title: String, pub entry_url: String, pub source_name: String, pub matched_rules: Vec<String>, pub status: String,
    pub attempts: i32, pub next_attempt_at: String, pub last_error: Option<String>, pub accepted_at: Option<String>, pub created_at: String,
} }
export_dto! { #[derive(Serialize, TS)] pub struct DeliveriesListResp { pub deliveries: Vec<DeliveryDto> } }
export_dto! { #[derive(Serialize, TS)] pub struct DeleteResp { #[ts(type = "number")] pub deleted: u64 } }
export_dto! { #[derive(Serialize, TS)] pub struct NotificationTestResp { pub status: String } }

pub async fn sources_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<SourcesListResp>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let mut result = Vec::new();
    for source in SourcesRepo::list(&ctx.db, user_id).await? {
        result.push(source_dto(&ctx.db, source).await?);
    }
    Ok(Json(SourcesListResp { sources: result }))
}

pub async fn sources_create(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<CreateSourceReq>,
) -> Result<Json<SourceDto>, AppError> {
    validate_name(&req.name)?;
    validate_interval(req.poll_interval_seconds)?;
    let normalized_url = collector::normalize_url(&req.url)?;
    let source = SourcesRepo::create(
        &ctx.db,
        parse_user_id(&user_id)?,
        req.name.trim().into(),
        normalized_url.clone(),
        normalized_url,
        req.poll_interval_seconds,
    )
    .await?;
    Ok(Json(source_dto(&ctx.db, source).await?))
}

pub async fn sources_patch(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<PatchSourceReq>,
) -> Result<Json<SourceDto>, AppError> {
    if let Some(name) = &req.name {
        validate_name(name)?;
    }
    if let Some(value) = req.poll_interval_seconds {
        validate_interval(value)?;
    }
    let source = SourcesRepo::update(
        &ctx.db,
        parse_user_id(&user_id)?,
        id,
        req.name.map(|v| v.trim().into()),
        req.poll_interval_seconds,
        req.enabled,
        req.archived,
    )
    .await?
    .ok_or_else(|| AppError::not_found("source not found"))?;
    Ok(Json(source_dto(&ctx.db, source).await?))
}

pub async fn sources_test(
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<TestSourceReq>,
) -> Result<Json<TestSourceResp>, AppError> {
    parse_user_id(&user_id)?;
    let url = collector::normalize_url(&req.url)?;
    let collector::FetchOutcome::Feed(feed) = collector::fetch(&url, None, None)
        .await
        .map_err(|e| AppError::bad_request(e.message))?
    else {
        return Err(AppError::bad_request("unexpected 304"));
    };
    Ok(Json(TestSourceResp {
        title: feed.title,
        entry_count: feed.entries.len(),
        categories: feed.categories,
        ttl_minutes: feed.ttl_minutes,
        limited_window: true,
    }))
}

pub async fn sources_refresh(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<RefreshSourceResp>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let source = SourcesRepo::get(&ctx.db, user_id, id)
        .await?
        .ok_or_else(|| AppError::not_found("source not found"))?;
    let Some((source, token)) =
        SourcesRepo::acquire_lease(&ctx.db, user_id, source.id, collector::COLLECTION_LEASE_SECONDS).await?
    else {
        return Ok(Json(RefreshSourceResp {
            status: "already-running".into(),
        }));
    };
    let db = ctx.db.clone();
    tokio::spawn(async move {
        if let Err(e) = collector::collect_leased(&db, &source, token).await {
            tracing::warn!(source_id=%source.id, error=%e.message, "manual RSS refresh failed");
        }
    });
    Ok(Json(RefreshSourceResp {
        status: "started".into(),
    }))
}

pub async fn entries_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Query(query): Query<EntriesQuery>,
) -> Result<Json<EntriesListResp>, AppError> {
    Ok(Json(
        search_entries(&ctx.db, parse_user_id(&user_id)?, query, 51).await?,
    ))
}
pub async fn entries_get(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<EntryDto>, AppError> {
    let (entry, source) = EntriesRepo::get_owned(&ctx.db, parse_user_id(&user_id)?, id)
        .await?
        .ok_or_else(|| AppError::not_found("entry not found"))?;
    Ok(Json(entry_dto(entry, &source.name)))
}

pub async fn saved_views_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<SavedViewsListResp>, AppError> {
    Ok(Json(SavedViewsListResp {
        views: SavedViewsRepo::list(&ctx.db, parse_user_id(&user_id)?)
            .await?
            .into_iter()
            .map(SavedViewDto::from)
            .collect(),
    }))
}

pub async fn saved_views_create(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<CreateSavedViewReq>,
) -> Result<Json<SavedViewDto>, AppError> {
    let categories = clean_values(req.categories)?;
    let include_any = clean_values(req.include_any)?;
    let exclude_any = clean_values(req.exclude_any)?;
    validate_saved_view(&req.name, &categories, &include_any, &exclude_any, &req.match_scope)?;
    let view = SavedViewsRepo::create(
        &ctx.db,
        parse_user_id(&user_id)?,
        SavedViewInput {
            source_id: req.source_id,
            name: req.name.trim().into(),
            categories,
            include_any,
            exclude_any,
            match_scope: req.match_scope,
        },
    )
    .await?;
    Ok(Json(SavedViewDto::from(view)))
}

pub async fn saved_views_patch(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<PatchSavedViewReq>,
) -> Result<Json<SavedViewDto>, AppError> {
    let user_id = parse_user_id(&user_id)?;
    let existing = SavedViewsRepo::get(&ctx.db, user_id, id)
        .await?
        .ok_or_else(|| AppError::not_found("saved view not found"))?;
    let categories = req.categories.map(clean_values).transpose()?;
    let include_any = req.include_any.map(clean_values).transpose()?;
    let exclude_any = req.exclude_any.map(clean_values).transpose()?;
    validate_saved_view(
        req.name.as_deref().unwrap_or(&existing.name),
        categories.as_deref().unwrap_or(&existing.categories),
        include_any.as_deref().unwrap_or(&existing.include_any),
        exclude_any.as_deref().unwrap_or(&existing.exclude_any),
        req.match_scope.as_deref().unwrap_or(&existing.match_scope),
    )?;
    let view = SavedViewsRepo::update(
        &ctx.db,
        user_id,
        id,
        SavedViewPatch {
            name: req.name.map(|value| value.trim().into()),
            categories,
            include_any,
            exclude_any,
            match_scope: req.match_scope,
        },
    )
    .await?
    .ok_or_else(|| AppError::not_found("saved view not found"))?;
    Ok(Json(SavedViewDto::from(view)))
}

pub async fn saved_views_delete(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<DeleteResp>, AppError> {
    let deleted = SavedViewsRepo::delete(&ctx.db, parse_user_id(&user_id)?, id).await?;
    if deleted == 0 {
        return Err(AppError::not_found("saved view not found"));
    }
    Ok(Json(DeleteResp { deleted }))
}

pub async fn rules_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<RulesListResp>, AppError> {
    Ok(Json(RulesListResp {
        rules: RulesRepo::list(&ctx.db, parse_user_id(&user_id)?)
            .await?
            .into_iter()
            .map(RuleDto::from)
            .collect(),
    }))
}
pub async fn rules_create(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<CreateRuleReq>,
) -> Result<Json<RuleDto>, AppError> {
    validate_rule(&req.name, &req.include_any, &req.exclude_any, &req.match_scope)?;
    let input = RuleInput {
        source_id: req.source_id,
        name: req.name.trim().into(),
        enabled: req.enabled,
        notify_enabled: req.notify_enabled,
        categories: clean_values(req.categories)?,
        include_any: clean_values(req.include_any)?,
        exclude_any: clean_values(req.exclude_any)?,
        match_scope: req.match_scope,
    };
    Ok(Json(RuleDto::from(
        RulesRepo::create(&ctx.db, parse_user_id(&user_id)?, input).await?,
    )))
}
pub async fn rules_patch(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<PatchRuleReq>,
) -> Result<Json<RuleDto>, AppError> {
    if let Some(v) = &req.name {
        validate_name(v)?;
    }
    if let Some(v) = &req.match_scope {
        validate_scope(v)?;
    }
    let categories = req.categories.map(clean_values).transpose()?;
    let include_any = req.include_any.map(clean_values).transpose()?;
    let exclude_any = req.exclude_any.map(clean_values).transpose()?;
    if include_any.as_ref().is_some_and(Vec::is_empty) {
        return Err(AppError::bad_request("includeAny requires a keyword"));
    }
    let rule = RulesRepo::update(
        &ctx.db,
        parse_user_id(&user_id)?,
        id,
        RulePatch {
            name: req.name.map(|v| v.trim().into()),
            enabled: req.enabled,
            notify_enabled: req.notify_enabled,
            categories,
            include_any,
            exclude_any,
            match_scope: req.match_scope,
        },
    )
    .await?
    .ok_or_else(|| AppError::not_found("rule not found"))?;
    Ok(Json(RuleDto::from(rule)))
}
pub async fn rules_delete(
    State(ctx): State<Arc<AppCtx>>,
    Path(id): Path<Uuid>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<DeleteResp>, AppError> {
    let deleted = RulesRepo::delete(&ctx.db, parse_user_id(&user_id)?, id).await?;
    if deleted == 0 {
        return Err(AppError::not_found("rule not found"));
    }
    Ok(Json(DeleteResp { deleted }))
}
pub async fn rules_preview(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
    Json(req): Json<RulePreviewReq>,
) -> Result<Json<RulePreviewResp>, AppError> {
    validate_rule(
        req.name.as_deref().unwrap_or("preview"),
        &req.include_any,
        &req.exclude_any,
        &req.match_scope,
    )?;
    let user_id = parse_user_id(&user_id)?;
    SourcesRepo::get(&ctx.db, user_id, req.source_id)
        .await?
        .ok_or_else(|| AppError::not_found("source not found"))?;
    let query = EntriesQuery {
        source_id: Some(req.source_id),
        cursor: req.cursor,
        match_scope: Some("title_summary".into()),
        ..Default::default()
    };
    let candidates = search_entries(&ctx.db, user_id, query, 501).await?;
    let now = Utc::now().fixed_offset();
    let draft = rules::Model {
        id: Uuid::nil(),
        user_id,
        source_id: req.source_id,
        name: req.name.unwrap_or_else(|| "preview".into()),
        enabled: req.enabled,
        notify_enabled: true,
        categories: req.categories,
        include_any: req.include_any,
        exclude_any: req.exclude_any,
        match_scope: req.match_scope,
        created_at: now,
        updated_at: now,
    };
    let mut matched = candidates
        .entries
        .into_iter()
        .filter(|dto| matcher::matches(&draft, &entry_for_match(dto)))
        .take(51)
        .collect::<Vec<_>>();
    let next_cursor = if matched.len() > 50 {
        matched.truncate(50);
        matched.last().map(dto_cursor).transpose()?
    } else {
        None
    };
    Ok(Json(EntriesListResp {
        entries: matched,
        next_cursor,
    }))
}

pub async fn deliveries_list(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<DeliveriesListResp>, AppError> {
    Ok(Json(DeliveriesListResp {
        deliveries: DeliveriesRepo::list(&ctx.db, parse_user_id(&user_id)?, 100)
            .await?
            .into_iter()
            .map(DeliveryDto::from)
            .collect(),
    }))
}
pub async fn notifications_test(
    State(ctx): State<Arc<AppCtx>>,
    TokimoUser { user_id }: TokimoUser,
) -> Result<Json<NotificationTestResp>, AppError> {
    let parsed = parse_user_id(&user_id)?;
    let client = ctx
        .client
        .get()
        .ok_or_else(|| AppError::internal("BusClient not yet bound"))?;
    notifications::notify(
        client,
        NotifyRequest {
            user_id,
            app_id: "rss",
            category_id: "keyword_match",
            category_label: "rss.notifications.keywordMatch",
            title: "RSS 订阅测试".into(),
            body: "此通知已提交给 Tokimo 系统通知中心。".into(),
            level: "info",
            action: Some(serde_json::json!({"type":"open-window","windowType":"rss"})),
            dedupe_key: format!("rss:{parsed}:test:{}", Uuid::new_v4()),
        },
    )
    .await?;
    Ok(Json(NotificationTestResp {
        status: "accepted".into(),
    }))
}

async fn source_dto(db: &DatabaseConnection, source: sources::Model) -> Result<SourceDto, AppError> {
    let entry_count = EntriesRepo::count_for_source(db, source.id).await?;
    Ok(SourceDto {
        id: source.id,
        name: source.name,
        url: source.url,
        enabled: source.enabled,
        archived_at: date_opt(source.archived_at),
        poll_interval_seconds: source.poll_interval_seconds,
        etag: source.etag,
        last_modified: source.last_modified,
        last_polled_at: date_opt(source.last_polled_at),
        last_success_at: date_opt(source.last_success_at),
        next_poll_at: source.next_poll_at.to_rfc3339(),
        initialized_at: date_opt(source.initialized_at),
        failure_count: source.failure_count,
        last_error: source.last_error,
        gap_suspected: source.possible_gap,
        entry_count,
    })
}
async fn search_entries(
    db: &DatabaseConnection,
    user_id: Uuid,
    query: EntriesQuery,
    limit: u64,
) -> Result<EntriesListResp, AppError> {
    if let Some(v) = &query.match_scope {
        validate_scope(v)?;
    }
    let saved_view = if let Some(view_id) = query.view_id {
        Some(
            SavedViewsRepo::get(db, user_id, view_id)
                .await?
                .ok_or_else(|| AppError::not_found("saved view not found"))?,
        )
    } else {
        None
    };
    let source_id = saved_view.as_ref().map(|view| view.source_id).or(query.source_id);
    let base_categories = saved_view
        .as_ref()
        .map(|view| view.categories.clone())
        .unwrap_or_default();
    let base_include_any = saved_view
        .as_ref()
        .map(|view| view.include_any.iter().map(|value| matcher::normalize(value)).collect())
        .unwrap_or_default();
    let base_exclude_any = saved_view
        .as_ref()
        .map(|view| view.exclude_any.iter().map(|value| matcher::normalize(value)).collect())
        .unwrap_or_default();
    let params = EntrySearch {
        query_terms: query
            .q
            .as_deref()
            .map(matcher::normalize)
            .unwrap_or_default()
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        source_id,
        base_categories,
        base_include_any,
        base_exclude_any,
        base_include_summary: saved_view
            .as_ref()
            .is_some_and(|view| view.match_scope == "title_summary"),
        category: query.category,
        author: query.author,
        published_from: parse_date(query.published_from)?,
        published_to: parse_date(query.published_to)?,
        include_summary: query.match_scope.as_deref() == Some("title_summary"),
        cursor: parse_cursor(query.cursor)?,
        limit,
    };
    let mut rows = EntriesRepo::search(db, user_id, &params).await?;
    let next_cursor = if rows.len() > 50 {
        rows.truncate(50);
        rows.last().map(|(e, _)| format!("{}|{}", e.sort_at.to_rfc3339(), e.id))
    } else {
        None
    };
    Ok(EntriesListResp {
        entries: rows.into_iter().map(|(e, s)| entry_dto(e, &s.name)).collect(),
        next_cursor,
    })
}
fn entry_dto(e: entries::Model, source_name: &str) -> EntryDto {
    EntryDto {
        id: e.id,
        source_id: e.source_id,
        source_name: source_name.into(),
        external_id: e.external_id,
        url: e.url,
        title: e.title,
        summary: e.summary,
        categories: e.categories,
        author: e.author,
        published_at: date_opt(e.published_at),
        first_seen_at: e.first_seen_at.to_rfc3339(),
        last_seen_at: e.last_seen_at.to_rfc3339(),
    }
}
fn entry_for_match(dto: &EntryDto) -> entries::Model {
    let first = DateTime::parse_from_rfc3339(&dto.first_seen_at).unwrap_or_else(|_| Utc::now().fixed_offset());
    let published = dto
        .published_at
        .as_deref()
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok());
    entries::Model {
        id: dto.id,
        source_id: dto.source_id,
        external_id: dto.external_id.clone(),
        url: dto.url.clone(),
        title: dto.title.clone(),
        summary: dto.summary.clone(),
        categories: dto.categories.clone(),
        author: dto.author.clone(),
        published_at: published,
        first_seen_at: first,
        last_seen_at: first,
        sort_at: published.unwrap_or(first),
        normalized_title: matcher::normalize(&dto.title),
        normalized_summary: dto.summary.as_deref().map(matcher::normalize),
    }
}
fn dto_cursor(dto: &EntryDto) -> Result<String, AppError> {
    let time = dto.published_at.as_deref().unwrap_or(&dto.first_seen_at);
    DateTime::parse_from_rfc3339(time).map_err(|_| AppError::internal("invalid entry time"))?;
    Ok(format!("{time}|{}", dto.id))
}
impl From<rules::Model> for RuleDto {
    fn from(r: rules::Model) -> Self {
        Self {
            id: r.id,
            source_id: r.source_id,
            name: r.name,
            enabled: r.enabled,
            notify_enabled: r.notify_enabled,
            categories: r.categories,
            include_any: r.include_any,
            exclude_any: r.exclude_any,
            match_scope: r.match_scope,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
        }
    }
}
impl From<saved_views::Model> for SavedViewDto {
    fn from(view: saved_views::Model) -> Self {
        Self {
            id: view.id,
            source_id: view.source_id,
            name: view.name,
            categories: view.categories,
            include_any: view.include_any,
            exclude_any: view.exclude_any,
            match_scope: view.match_scope,
            created_at: view.created_at.to_rfc3339(),
            updated_at: view.updated_at.to_rfc3339(),
        }
    }
}
impl From<DeliveryView> for DeliveryDto {
    fn from(v: DeliveryView) -> Self {
        Self {
            id: v.delivery.id,
            entry_id: v.delivery.entry_id,
            entry_title: v.entry.title,
            entry_url: v.entry.url,
            source_name: v.source.name,
            matched_rules: v.rule_names,
            status: v.delivery.status,
            attempts: v.delivery.attempts,
            next_attempt_at: v.delivery.next_attempt_at.to_rfc3339(),
            last_error: v.delivery.last_error,
            accepted_at: date_opt(v.delivery.accepted_at),
            created_at: v.delivery.created_at.to_rfc3339(),
        }
    }
}
fn validate_name(v: &str) -> Result<(), AppError> {
    if v.trim().is_empty() || v.chars().count() > 200 {
        Err(AppError::bad_request("name must be 1..=200 characters"))
    } else {
        Ok(())
    }
}
fn validate_interval(v: i32) -> Result<(), AppError> {
    if (60..=86_400).contains(&v) {
        Ok(())
    } else {
        Err(AppError::bad_request("pollIntervalSeconds must be 60..=86400"))
    }
}
fn validate_scope(v: &str) -> Result<(), AppError> {
    if matches!(v, "title" | "title_summary") {
        Ok(())
    } else {
        Err(AppError::bad_request("matchScope must be title or title_summary"))
    }
}
fn validate_rule(name: &str, include: &[String], exclude: &[String], scope: &str) -> Result<(), AppError> {
    validate_name(name)?;
    validate_scope(scope)?;
    if include.is_empty() || include.iter().all(|v| v.trim().is_empty()) {
        return Err(AppError::bad_request("includeAny requires a keyword"));
    }
    if include.len() > 100 || exclude.len() > 100 {
        return Err(AppError::bad_request("too many keywords"));
    }
    Ok(())
}
fn validate_saved_view(
    name: &str,
    categories: &[String],
    include: &[String],
    exclude: &[String],
    scope: &str,
) -> Result<(), AppError> {
    validate_name(name)?;
    validate_scope(scope)?;
    if categories.is_empty() && include.is_empty() {
        return Err(AppError::bad_request(
            "saved view requires a category or includeAny keyword",
        ));
    }
    if categories.len() > 100 || include.len() > 100 || exclude.len() > 100 {
        return Err(AppError::bad_request("too many filter values"));
    }
    Ok(())
}
fn clean_values(v: Vec<String>) -> Result<Vec<String>, AppError> {
    if v.iter().any(|x| x.chars().count() > 200) {
        return Err(AppError::bad_request("filter value is too long"));
    }
    Ok(v.into_iter()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect())
}
fn parse_date(v: Option<String>) -> Result<Option<DateTime<FixedOffset>>, AppError> {
    v.map(|x| DateTime::parse_from_rfc3339(&x).map_err(|_| AppError::bad_request("invalid RFC3339 date")))
        .transpose()
}
fn parse_cursor(v: Option<String>) -> Result<Option<(DateTime<FixedOffset>, Uuid)>, AppError> {
    v.map(|x| {
        let (t, id) = x
            .rsplit_once('|')
            .ok_or_else(|| AppError::bad_request("invalid cursor"))?;
        Ok((
            DateTime::parse_from_rfc3339(t).map_err(|_| AppError::bad_request("invalid cursor"))?,
            id.parse().map_err(|_| AppError::bad_request("invalid cursor"))?,
        ))
    })
    .transpose()
}
fn date_opt(v: Option<DateTime<FixedOffset>>) -> Option<String> {
    v.map(|x| x.to_rfc3339())
}
fn parse_user_id(v: &str) -> Result<Uuid, AppError> {
    v.parse()
        .map_err(|_| AppError::bad_request("invalid authenticated user id"))
}
fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::validate_saved_view;

    #[test]
    fn saved_view_accepts_category_or_keywords() {
        assert!(validate_saved_view("trade", &["trade".into()], &[], &[], "title").is_ok());
        assert!(validate_saved_view("providers", &[], &["DMIT".into(), "搬瓦工".into()], &[], "title").is_ok());
        assert!(
            validate_saved_view(
                "trade providers",
                &["trade".into()],
                &["DMIT".into(), "搬瓦工".into()],
                &[],
                "title_summary",
            )
            .is_ok()
        );
    }

    #[test]
    fn saved_view_rejects_empty_or_exclude_only_filters() {
        assert!(validate_saved_view("empty", &[], &[], &[], "title").is_err());
        assert!(validate_saved_view("exclude only", &[], &[], &["求购".into()], "title").is_err());
    }
}
