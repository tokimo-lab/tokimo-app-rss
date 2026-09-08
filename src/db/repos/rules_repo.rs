use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use uuid::Uuid;

use crate::{
    AppError,
    db::entities::{rules, sources},
};

#[derive(Clone)]
pub struct RuleInput {
    pub source_id: Uuid,
    pub name: String,
    pub enabled: bool,
    pub categories: Vec<String>,
    pub include_any: Vec<String>,
    pub exclude_any: Vec<String>,
    pub match_scope: String,
}

pub struct RulePatch {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub categories: Option<Vec<String>>,
    pub include_any: Option<Vec<String>>,
    pub exclude_any: Option<Vec<String>>,
    pub match_scope: Option<String>,
}

pub struct RulesRepo;

impl RulesRepo {
    async fn source_is_owned<C: ConnectionTrait>(db: &C, user_id: Uuid, source_id: Uuid) -> Result<bool, AppError> {
        Ok(sources::Entity::find()
            .filter(sources::Column::Id.eq(source_id))
            .filter(sources::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some())
    }

    pub async fn list<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<rules::Model>, AppError> {
        Ok(rules::Entity::find()
            .filter(rules::Column::UserId.eq(user_id))
            .order_by_asc(rules::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn active_for_source<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        source_id: Uuid,
    ) -> Result<Vec<rules::Model>, AppError> {
        Ok(rules::Entity::find()
            .filter(rules::Column::UserId.eq(user_id))
            .filter(rules::Column::SourceId.eq(source_id))
            .filter(rules::Column::Enabled.eq(true))
            .all(db)
            .await?)
    }

    pub async fn create<C: ConnectionTrait>(db: &C, user_id: Uuid, input: RuleInput) -> Result<rules::Model, AppError> {
        if !Self::source_is_owned(db, user_id, input.source_id).await? {
            return Err(AppError::not_found("source not found"));
        }
        let now = Utc::now().fixed_offset();
        Ok(rules::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            source_id: Set(input.source_id),
            name: Set(input.name),
            enabled: Set(input.enabled),
            categories: Set(input.categories),
            include_any: Set(input.include_any),
            exclude_any: Set(input.exclude_any),
            match_scope: Set(input.match_scope),
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
        patch: RulePatch,
    ) -> Result<Option<rules::Model>, AppError> {
        let Some(model) = rules::Entity::find()
            .filter(rules::Column::Id.eq(id))
            .filter(rules::Column::UserId.eq(user_id))
            .one(db)
            .await?
        else {
            return Ok(None);
        };
        let mut active: rules::ActiveModel = model.into();
        if let Some(value) = patch.name {
            active.name = Set(value);
        }
        if let Some(value) = patch.enabled {
            active.enabled = Set(value);
        }
        if let Some(value) = patch.categories {
            active.categories = Set(value);
        }
        if let Some(value) = patch.include_any {
            active.include_any = Set(value);
        }
        if let Some(value) = patch.exclude_any {
            active.exclude_any = Set(value);
        }
        if let Some(value) = patch.match_scope {
            active.match_scope = Set(value);
        }
        active.updated_at = Set(Utc::now().fixed_offset());
        Ok(Some(active.update(db).await?))
    }

    pub async fn delete<C: ConnectionTrait>(db: &C, user_id: Uuid, id: Uuid) -> Result<u64, AppError> {
        Ok(rules::Entity::delete_many()
            .filter(rules::Column::Id.eq(id))
            .filter(rules::Column::UserId.eq(user_id))
            .exec(db)
            .await?
            .rows_affected)
    }
}
