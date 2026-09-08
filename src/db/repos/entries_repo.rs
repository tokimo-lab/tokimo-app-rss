use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TryInsertResult,
    sea_query::{Expr, OnConflict, extension::postgres::PgBinOper},
};
use uuid::Uuid;

use crate::{
    AppError,
    db::entities::{entries, sources},
};

#[derive(Clone, Debug)]
pub struct NewEntry {
    pub external_id: String,
    pub url: String,
    pub title: String,
    pub summary: Option<String>,
    pub categories: Vec<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<FixedOffset>>,
    pub normalized_title: String,
    pub normalized_summary: Option<String>,
}

#[derive(Default)]
pub struct EntrySearch {
    pub query_terms: Vec<String>,
    pub source_id: Option<Uuid>,
    pub base_categories: Vec<String>,
    pub base_include_any: Vec<String>,
    pub base_exclude_any: Vec<String>,
    pub base_include_summary: bool,
    pub category: Option<String>,
    pub author: Option<String>,
    pub published_from: Option<DateTime<FixedOffset>>,
    pub published_to: Option<DateTime<FixedOffset>>,
    pub include_summary: bool,
    pub cursor: Option<(DateTime<FixedOffset>, Uuid)>,
    pub limit: u64,
}

pub struct EntriesRepo;

impl EntriesRepo {
    pub async fn count_for_source<C: ConnectionTrait>(db: &C, source_id: Uuid) -> Result<u64, AppError> {
        use sea_orm::PaginatorTrait;
        Ok(entries::Entity::find()
            .filter(entries::Column::SourceId.eq(source_id))
            .count(db)
            .await?)
    }

    pub async fn get_owned<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<(entries::Model, sources::Model)>, AppError> {
        let Some(entry) = entries::Entity::find_by_id(id).one(db).await? else {
            return Ok(None);
        };
        let source = sources::Entity::find()
            .filter(sources::Column::Id.eq(entry.source_id))
            .filter(sources::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        Ok(source.map(|source| (entry, source)))
    }

    pub async fn search<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        params: &EntrySearch,
    ) -> Result<Vec<(entries::Model, sources::Model)>, AppError> {
        let mut source_query = sources::Entity::find().filter(sources::Column::UserId.eq(user_id));
        if let Some(source_id) = params.source_id {
            source_query = source_query.filter(sources::Column::Id.eq(source_id));
        }
        let owned_sources = source_query.all(db).await?;
        if owned_sources.is_empty() {
            return Ok(Vec::new());
        }
        let source_ids = owned_sources.iter().map(|source| source.id).collect::<Vec<_>>();
        let mut query = entries::Entity::find().filter(entries::Column::SourceId.is_in(source_ids));
        for term in &params.query_terms {
            let pattern = format!("%{}%", escape_like(term));
            let condition = if params.include_summary {
                Condition::any()
                    .add(entries::Column::NormalizedTitle.like(&pattern))
                    .add(entries::Column::NormalizedSummary.like(&pattern))
            } else {
                Condition::all().add(entries::Column::NormalizedTitle.like(&pattern))
            };
            query = query.filter(condition);
        }
        if !params.base_categories.is_empty() {
            query = query.filter(
                Expr::col(entries::Column::Categories)
                    .binary(PgBinOper::Overlap, Expr::val(params.base_categories.clone())),
            );
        }
        if !params.base_include_any.is_empty() {
            let mut condition = Condition::any();
            for term in &params.base_include_any {
                let pattern = format!("%{}%", escape_like(term));
                condition = condition.add(entries::Column::NormalizedTitle.like(&pattern));
                if params.base_include_summary {
                    condition = condition.add(entries::Column::NormalizedSummary.like(&pattern));
                }
            }
            query = query.filter(condition);
        }
        if !params.base_exclude_any.is_empty() {
            let mut condition = Condition::all();
            for term in &params.base_exclude_any {
                let pattern = format!("%{}%", escape_like(term));
                condition = condition.add(entries::Column::NormalizedTitle.not_like(&pattern));
                if params.base_include_summary {
                    condition = condition.add(
                        Condition::any()
                            .add(entries::Column::NormalizedSummary.is_null())
                            .add(entries::Column::NormalizedSummary.not_like(&pattern)),
                    );
                }
            }
            query = query.filter(condition);
        }
        if let Some(category) = &params.category {
            query = query.filter(
                Expr::col(entries::Column::Categories).binary(PgBinOper::Contains, Expr::val(vec![category.clone()])),
            );
        }
        if let Some(author) = &params.author {
            query = query.filter(entries::Column::Author.contains(author));
        }
        if let Some(from) = params.published_from {
            query = query.filter(entries::Column::PublishedAt.gte(from));
        }
        if let Some(to) = params.published_to {
            query = query.filter(entries::Column::PublishedAt.lte(to));
        }
        if let Some((sort_at, id)) = params.cursor {
            query = query.filter(
                Condition::any().add(entries::Column::SortAt.lt(sort_at)).add(
                    Condition::all()
                        .add(entries::Column::SortAt.eq(sort_at))
                        .add(entries::Column::Id.lt(id)),
                ),
            );
        }
        let entries = query
            .order_by_desc(entries::Column::SortAt)
            .order_by_desc(entries::Column::Id)
            .limit(params.limit)
            .all(db)
            .await?;
        Ok(entries
            .into_iter()
            .filter_map(|entry| {
                owned_sources
                    .iter()
                    .find(|source| source.id == entry.source_id)
                    .cloned()
                    .map(|source| (entry, source))
            })
            .collect())
    }

    /// Inserts or refreshes an entry and reports whether this transaction discovered it.
    pub async fn upsert<C: ConnectionTrait>(
        db: &C,
        source_id: Uuid,
        input: NewEntry,
    ) -> Result<(entries::Model, bool), AppError> {
        let now = Utc::now().fixed_offset();
        let id = Uuid::new_v4();
        let insert = entries::Entity::insert(entries::ActiveModel {
            id: Set(id),
            source_id: Set(source_id),
            external_id: Set(input.external_id.clone()),
            url: Set(input.url.clone()),
            title: Set(input.title.clone()),
            summary: Set(input.summary.clone()),
            categories: Set(input.categories.clone()),
            author: Set(input.author.clone()),
            published_at: Set(input.published_at),
            first_seen_at: Set(now),
            last_seen_at: Set(now),
            sort_at: sea_orm::NotSet,
            normalized_title: Set(input.normalized_title.clone()),
            normalized_summary: Set(input.normalized_summary.clone()),
        })
        .on_conflict(
            OnConflict::columns([entries::Column::SourceId, entries::Column::ExternalId])
                .do_nothing()
                .to_owned(),
        )
        .try_insert()
        .exec(db)
        .await?;
        let inserted = matches!(insert, TryInsertResult::Inserted(_));
        if !inserted {
            entries::Entity::update_many()
                .filter(entries::Column::SourceId.eq(source_id))
                .filter(entries::Column::ExternalId.eq(&input.external_id))
                .col_expr(entries::Column::Url, sea_orm::sea_query::Expr::value(input.url))
                .col_expr(entries::Column::Title, sea_orm::sea_query::Expr::value(input.title))
                .col_expr(entries::Column::Summary, sea_orm::sea_query::Expr::value(input.summary))
                .col_expr(
                    entries::Column::Categories,
                    sea_orm::sea_query::Expr::value(input.categories),
                )
                .col_expr(entries::Column::Author, sea_orm::sea_query::Expr::value(input.author))
                .col_expr(
                    entries::Column::PublishedAt,
                    sea_orm::sea_query::Expr::value(input.published_at),
                )
                .col_expr(entries::Column::LastSeenAt, sea_orm::sea_query::Expr::value(now))
                .col_expr(
                    entries::Column::NormalizedTitle,
                    sea_orm::sea_query::Expr::value(input.normalized_title),
                )
                .col_expr(
                    entries::Column::NormalizedSummary,
                    sea_orm::sea_query::Expr::value(input.normalized_summary),
                )
                .exec(db)
                .await?;
        }
        let model = entries::Entity::find()
            .filter(entries::Column::SourceId.eq(source_id))
            .filter(entries::Column::ExternalId.eq(input.external_id))
            .one(db)
            .await?
            .ok_or_else(|| AppError::internal("entry missing after upsert"))?;
        Ok((model, inserted))
    }
}

fn escape_like(value: &str) -> String {
    value.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}
