use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "rss", table_name = "entries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub source_id: Uuid,
    pub external_id: String,
    pub url: String,
    pub title: String,
    pub summary: Option<String>,
    pub categories: Vec<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub first_seen_at: DateTimeWithTimeZone,
    pub last_seen_at: DateTimeWithTimeZone,
    pub sort_at: DateTimeWithTimeZone,
    pub normalized_title: String,
    pub normalized_summary: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
