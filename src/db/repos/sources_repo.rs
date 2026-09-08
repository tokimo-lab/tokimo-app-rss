use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::{
    AppError,
    db::entities::sources::{self, ActiveModel, Column, Entity},
};

pub struct SourcesRepo;

impl SourcesRepo {
    pub async fn list<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<sources::Model>, AppError> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_asc(Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn get<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<Option<sources::Model>, AppError> {
        Ok(Entity::find()
            .filter(Column::Id.eq(id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?)
    }

    pub async fn create<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        name: String,
        url: String,
        normalized_url: String,
        poll_interval_seconds: i32,
    ) -> Result<sources::Model, AppError> {
        let now = Utc::now().fixed_offset();
        Ok(ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            name: Set(name),
            url: Set(url),
            normalized_url: Set(normalized_url),
            enabled: Set(true),
            archived_at: Set(None),
            poll_interval_seconds: Set(poll_interval_seconds),
            etag: Set(None),
            last_modified: Set(None),
            last_polled_at: Set(None),
            last_success_at: Set(None),
            next_poll_at: Set(now),
            initialized_at: Set(None),
            last_window_ids: Set(Vec::new()),
            possible_gap: Set(false),
            failure_count: Set(0),
            last_error: Set(None),
            lease_token: Set(None),
            lease_until: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await?)
    }

    pub async fn update<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
        name: Option<String>,
        interval: Option<i32>,
        enabled: Option<bool>,
        archived: Option<bool>,
    ) -> Result<Option<sources::Model>, AppError> {
        let Some(model) = Self::get(db, user_id, id).await? else {
            return Ok(None);
        };
        let mut active: ActiveModel = model.into();
        if let Some(value) = name {
            active.name = Set(value);
        }
        if let Some(value) = interval {
            active.poll_interval_seconds = Set(value);
        }
        if let Some(value) = enabled {
            active.enabled = Set(value);
        }
        if let Some(value) = archived {
            active.archived_at = Set(value.then(|| Utc::now().fixed_offset()));
            if value {
                active.enabled = Set(false);
            }
        }
        active.updated_at = Set(Utc::now().fixed_offset());
        Ok(Some(active.update(db).await?))
    }

    pub async fn due<C: ConnectionTrait>(db: &C, limit: u64) -> Result<Vec<sources::Model>, AppError> {
        let now = Utc::now().fixed_offset();
        Ok(Entity::find()
            .filter(Column::Enabled.eq(true))
            .filter(Column::ArchivedAt.is_null())
            .filter(Column::NextPollAt.lte(now))
            .order_by_asc(Column::NextPollAt)
            .limit(limit)
            .all(db)
            .await?)
    }

    pub async fn acquire_lease<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
        lease_seconds: i64,
    ) -> Result<Option<(sources::Model, Uuid)>, AppError> {
        let now = Utc::now().fixed_offset();
        let token = Uuid::new_v4();
        let result = Entity::update_many()
            .filter(Column::Id.eq(id))
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Enabled.eq(true))
            .filter(Column::ArchivedAt.is_null())
            .filter(Column::LeaseUntil.is_null().or(Column::LeaseUntil.lt(now)))
            .col_expr(Column::LeaseToken, sea_orm::sea_query::Expr::value(token))
            .col_expr(
                Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(now + Duration::seconds(lease_seconds)),
            )
            .exec(db)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }
        Ok(Self::get(db, user_id, id).await?.map(|source| (source, token)))
    }

    pub async fn complete<C: ConnectionTrait>(
        db: &C,
        source: &sources::Model,
        token: Uuid,
        etag: Option<String>,
        last_modified: Option<String>,
        window_ids: Vec<String>,
        possible_gap: bool,
    ) -> Result<bool, AppError> {
        let now = Utc::now().fixed_offset();
        let result = Entity::update_many()
            .filter(Column::Id.eq(source.id))
            .filter(Column::UserId.eq(source.user_id))
            .filter(Column::LeaseToken.eq(token))
            .col_expr(Column::Etag, sea_orm::sea_query::Expr::value(etag))
            .col_expr(Column::LastModified, sea_orm::sea_query::Expr::value(last_modified))
            .col_expr(Column::LastPolledAt, sea_orm::sea_query::Expr::value(now))
            .col_expr(Column::LastSuccessAt, sea_orm::sea_query::Expr::value(now))
            .col_expr(
                Column::NextPollAt,
                sea_orm::sea_query::Expr::value(
                    now + Duration::seconds(
                        i64::from(source.poll_interval_seconds)
                            + jitter_seconds(source.id, u64::try_from(source.poll_interval_seconds).unwrap_or(60)),
                    ),
                ),
            )
            .col_expr(
                Column::InitializedAt,
                sea_orm::sea_query::Expr::value(source.initialized_at.or(Some(now))),
            )
            .col_expr(Column::LastWindowIds, sea_orm::sea_query::Expr::value(window_ids))
            .col_expr(Column::PossibleGap, sea_orm::sea_query::Expr::value(possible_gap))
            .col_expr(Column::FailureCount, sea_orm::sea_query::Expr::value(0))
            .col_expr(
                Column::LastError,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .col_expr(
                Column::LeaseToken,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .col_expr(
                Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<chrono::FixedOffset>>::None),
            )
            .col_expr(Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .exec(db)
            .await?;
        Ok(result.rows_affected == 1)
    }

    pub async fn fail<C: ConnectionTrait>(
        db: &C,
        source: &sources::Model,
        token: Uuid,
        error: String,
        retry_after_seconds: Option<u64>,
    ) -> Result<bool, AppError> {
        let now = Utc::now().fixed_offset();
        let failures = source.failure_count.saturating_add(1);
        let exponential = 60_u64.saturating_mul(2_u64.saturating_pow(failures.clamp(0, 8) as u32));
        let delay = retry_after_seconds.unwrap_or(exponential).clamp(60, 86_400);
        let jittered_delay = delay.saturating_add(jitter_seconds(source.id, delay) as u64);
        let result = Entity::update_many()
            .filter(Column::Id.eq(source.id))
            .filter(Column::UserId.eq(source.user_id))
            .filter(Column::LeaseToken.eq(token))
            .col_expr(Column::LastPolledAt, sea_orm::sea_query::Expr::value(now))
            .col_expr(
                Column::NextPollAt,
                sea_orm::sea_query::Expr::value(now + Duration::seconds(jittered_delay as i64)),
            )
            .col_expr(Column::FailureCount, sea_orm::sea_query::Expr::value(failures))
            .col_expr(Column::LastError, sea_orm::sea_query::Expr::value(Some(error)))
            .col_expr(
                Column::LeaseToken,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .col_expr(
                Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<chrono::FixedOffset>>::None),
            )
            .col_expr(Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .exec(db)
            .await?;
        Ok(result.rows_affected == 1)
    }
}

fn jitter_seconds(id: Uuid, base_seconds: u64) -> i64 {
    let spread = (base_seconds / 10).clamp(1, 30);
    let seed = id
        .as_bytes()
        .iter()
        .fold(0_u64, |acc, byte| acc.wrapping_mul(31).wrapping_add(u64::from(*byte)));
    i64::try_from(seed % (spread + 1)).unwrap_or_default()
}
