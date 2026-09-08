use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "rss", table_name = "sources")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub url: String,
    pub normalized_url: String,
    pub enabled: bool,
    pub archived_at: Option<DateTimeWithTimeZone>,
    pub poll_interval_seconds: i32,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub last_polled_at: Option<DateTimeWithTimeZone>,
    pub last_success_at: Option<DateTimeWithTimeZone>,
    pub next_poll_at: DateTimeWithTimeZone,
    pub initialized_at: Option<DateTimeWithTimeZone>,
    pub last_window_ids: Vec<String>,
    pub possible_gap: bool,
    pub failure_count: i32,
    pub last_error: Option<String>,
    pub lease_token: Option<Uuid>,
    pub lease_until: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
