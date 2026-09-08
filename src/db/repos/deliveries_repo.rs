use chrono::{Duration, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect, Set, TryInsertResult,
    sea_query::OnConflict,
};
use uuid::Uuid;

use crate::{
    AppError,
    db::entities::{deliveries, entries, rules, sources},
};

#[derive(Clone)]
pub struct DeliveryView {
    pub delivery: deliveries::Model,
    pub entry: entries::Model,
    pub source: sources::Model,
    pub rule_names: Vec<String>,
    pub active_rule_names: Vec<String>,
}

pub struct DeliveriesRepo;

impl DeliveriesRepo {
    pub async fn create_pending<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        entry_id: Uuid,
        matched_rules: Vec<Uuid>,
        matched_rule_names: Vec<String>,
    ) -> Result<bool, AppError> {
        let now = Utc::now().fixed_offset();
        let result = deliveries::Entity::insert(deliveries::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            entry_id: Set(entry_id),
            matched_rules: Set(matched_rules),
            matched_rule_names: Set(matched_rule_names),
            status: Set("pending".to_string()),
            attempts: Set(0),
            next_attempt_at: Set(now),
            last_error: Set(None),
            accepted_at: Set(None),
            lease_token: Set(None),
            lease_until: Set(None),
            dedupe_key: Set(format!("rss:{user_id}:{entry_id}")),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::columns([deliveries::Column::UserId, deliveries::Column::EntryId])
                .do_nothing()
                .to_owned(),
        )
        .try_insert()
        .exec(db)
        .await?;
        Ok(matches!(result, TryInsertResult::Inserted(_)))
    }

    pub async fn list<C: ConnectionTrait>(db: &C, user_id: Uuid, limit: u64) -> Result<Vec<DeliveryView>, AppError> {
        let rows = deliveries::Entity::find()
            .filter(deliveries::Column::UserId.eq(user_id))
            .order_by_desc(deliveries::Column::CreatedAt)
            .limit(limit)
            .all(db)
            .await?;
        let mut output = Vec::with_capacity(rows.len());
        for delivery in rows {
            if let Some(view) = Self::hydrate(db, delivery, Some(user_id)).await? {
                output.push(view);
            }
        }
        Ok(output)
    }

    pub async fn acquire_due<C: ConnectionTrait>(db: &C) -> Result<Option<(DeliveryView, Uuid)>, AppError> {
        let now = Utc::now().fixed_offset();
        let candidates = deliveries::Entity::find()
            .filter(deliveries::Column::Status.is_in(["pending", "submitting"]))
            .filter(deliveries::Column::NextAttemptAt.lte(now))
            .filter(
                deliveries::Column::LeaseUntil
                    .is_null()
                    .or(deliveries::Column::LeaseUntil.lt(now)),
            )
            .order_by_asc(deliveries::Column::NextAttemptAt)
            .limit(8)
            .all(db)
            .await?;
        for candidate in candidates {
            let token = Uuid::new_v4();
            let updated = deliveries::Entity::update_many()
                .filter(deliveries::Column::Id.eq(candidate.id))
                .filter(deliveries::Column::UserId.eq(candidate.user_id))
                .filter(deliveries::Column::Status.is_in(["pending", "submitting"]))
                .filter(
                    deliveries::Column::LeaseUntil
                        .is_null()
                        .or(deliveries::Column::LeaseUntil.lt(now)),
                )
                .col_expr(
                    deliveries::Column::Status,
                    sea_orm::sea_query::Expr::value("submitting"),
                )
                .col_expr(deliveries::Column::LeaseToken, sea_orm::sea_query::Expr::value(token))
                .col_expr(
                    deliveries::Column::LeaseUntil,
                    sea_orm::sea_query::Expr::value(now + Duration::seconds(60)),
                )
                .col_expr(deliveries::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
                .exec(db)
                .await?;
            if updated.rows_affected == 1 {
                let leased = deliveries::Entity::find_by_id(candidate.id)
                    .one(db)
                    .await?
                    .ok_or_else(|| AppError::internal("delivery disappeared after lease"))?;
                if let Some(view) = Self::hydrate(db, leased, None).await? {
                    return Ok(Some((view, token)));
                }
            }
        }
        Ok(None)
    }

    async fn hydrate<C: ConnectionTrait>(
        db: &C,
        delivery: deliveries::Model,
        expected_user: Option<Uuid>,
    ) -> Result<Option<DeliveryView>, AppError> {
        if expected_user.is_some_and(|user| user != delivery.user_id) {
            return Ok(None);
        }
        let Some(entry) = entries::Entity::find_by_id(delivery.entry_id).one(db).await? else {
            return Ok(None);
        };
        let Some(source) = sources::Entity::find()
            .filter(sources::Column::Id.eq(entry.source_id))
            .filter(sources::Column::UserId.eq(delivery.user_id))
            .one(db)
            .await?
        else {
            return Ok(None);
        };
        let active_rules = rules::Entity::find()
            .filter(rules::Column::UserId.eq(delivery.user_id))
            .filter(rules::Column::SourceId.eq(source.id))
            .filter(rules::Column::Id.is_in(delivery.matched_rules.clone()))
            .filter(rules::Column::Enabled.eq(true))
            .filter(rules::Column::NotifyEnabled.eq(true))
            .all(db)
            .await?;
        Ok(Some(DeliveryView {
            rule_names: delivery.matched_rule_names.clone(),
            delivery,
            entry,
            source,
            active_rule_names: active_rules.into_iter().map(|rule| rule.name).collect(),
        }))
    }

    pub async fn cancel<C: ConnectionTrait>(db: &C, id: Uuid, user_id: Uuid, token: Uuid) -> Result<bool, AppError> {
        Self::finish(db, id, user_id, token, "cancelled", None).await
    }

    pub async fn accepted<C: ConnectionTrait>(db: &C, id: Uuid, user_id: Uuid, token: Uuid) -> Result<bool, AppError> {
        Self::finish(db, id, user_id, token, "accepted", None).await
    }

    pub async fn failed<C: ConnectionTrait>(
        db: &C,
        delivery: &deliveries::Model,
        token: Uuid,
        error: String,
    ) -> Result<bool, AppError> {
        let now = Utc::now().fixed_offset();
        let attempts = delivery.attempts.saturating_add(1);
        let delay = 30_i64
            .saturating_mul(2_i64.saturating_pow(attempts.clamp(0, 10) as u32))
            .clamp(60, 3600);
        let result = deliveries::Entity::update_many()
            .filter(deliveries::Column::Id.eq(delivery.id))
            .filter(deliveries::Column::UserId.eq(delivery.user_id))
            .filter(deliveries::Column::LeaseToken.eq(token))
            .col_expr(deliveries::Column::Status, sea_orm::sea_query::Expr::value("pending"))
            .col_expr(deliveries::Column::Attempts, sea_orm::sea_query::Expr::value(attempts))
            .col_expr(
                deliveries::Column::NextAttemptAt,
                sea_orm::sea_query::Expr::value(now + Duration::seconds(delay)),
            )
            .col_expr(
                deliveries::Column::LastError,
                sea_orm::sea_query::Expr::value(Some(error)),
            )
            .col_expr(
                deliveries::Column::LeaseToken,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .col_expr(
                deliveries::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<chrono::FixedOffset>>::None),
            )
            .col_expr(deliveries::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .exec(db)
            .await?;
        Ok(result.rows_affected == 1)
    }

    async fn finish<C: ConnectionTrait>(
        db: &C,
        id: Uuid,
        user_id: Uuid,
        token: Uuid,
        status: &str,
        error: Option<String>,
    ) -> Result<bool, AppError> {
        let now = Utc::now().fixed_offset();
        let result = deliveries::Entity::update_many()
            .filter(deliveries::Column::Id.eq(id))
            .filter(deliveries::Column::UserId.eq(user_id))
            .filter(deliveries::Column::LeaseToken.eq(token))
            .col_expr(deliveries::Column::Status, sea_orm::sea_query::Expr::value(status))
            .col_expr(
                deliveries::Column::AcceptedAt,
                sea_orm::sea_query::Expr::value((status == "accepted").then_some(now)),
            )
            .col_expr(deliveries::Column::LastError, sea_orm::sea_query::Expr::value(error))
            .col_expr(
                deliveries::Column::LeaseToken,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .col_expr(
                deliveries::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<chrono::FixedOffset>>::None),
            )
            .col_expr(deliveries::Column::UpdatedAt, sea_orm::sea_query::Expr::value(now))
            .exec(db)
            .await?;
        Ok(result.rows_affected == 1)
    }
}
