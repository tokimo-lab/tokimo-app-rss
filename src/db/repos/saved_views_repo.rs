use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use uuid::Uuid;

use crate::{
    AppError,
    db::entities::{saved_views, sources},
};

pub struct SavedViewInput {
    pub source_id: Uuid,
    pub name: String,
    pub categories: Vec<String>,
    pub include_any: Vec<String>,
    pub exclude_any: Vec<String>,
    pub match_scope: String,
}

pub struct SavedViewPatch {
    pub name: Option<String>,
    pub categories: Option<Vec<String>>,
    pub include_any: Option<Vec<String>>,
    pub exclude_any: Option<Vec<String>>,
    pub match_scope: Option<String>,
}

pub struct SavedViewsRepo;

impl SavedViewsRepo {
    pub async fn list<C: ConnectionTrait>(db: &C, user_id: Uuid) -> Result<Vec<saved_views::Model>, AppError> {
        Ok(saved_views::Entity::find()
            .filter(saved_views::Column::UserId.eq(user_id))
            .order_by_asc(saved_views::Column::CreatedAt)
            .all(db)
            .await?)
    }

    pub async fn get<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<Option<saved_views::Model>, AppError> {
        Ok(saved_views::Entity::find()
            .filter(saved_views::Column::Id.eq(id))
            .filter(saved_views::Column::UserId.eq(user_id))
            .one(db)
            .await?)
    }

    pub async fn create<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        input: SavedViewInput,
    ) -> Result<saved_views::Model, AppError> {
        let source_owned = sources::Entity::find()
            .filter(sources::Column::Id.eq(input.source_id))
            .filter(sources::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some();
        if !source_owned {
            return Err(AppError::not_found("source not found"));
        }
        let now = Utc::now().fixed_offset();
        Ok(saved_views::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            source_id: Set(input.source_id),
            name: Set(input.name),
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
        patch: SavedViewPatch,
    ) -> Result<Option<saved_views::Model>, AppError> {
        let Some(model) = Self::get(db, user_id, id).await? else {
            return Ok(None);
        };
        let mut active: saved_views::ActiveModel = model.into();
        if let Some(value) = patch.name {
            active.name = Set(value);
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
        Ok(saved_views::Entity::delete_many()
            .filter(saved_views::Column::Id.eq(id))
            .filter(saved_views::Column::UserId.eq(user_id))
            .exec(db)
            .await?
            .rows_affected)
    }
}
