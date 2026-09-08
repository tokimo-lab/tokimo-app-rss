use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "rss", table_name = "deliveries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub entry_id: Uuid,
    pub matched_rules: Vec<Uuid>,
    pub matched_rule_names: Vec<String>,
    pub status: String,
    pub attempts: i32,
    pub next_attempt_at: DateTimeWithTimeZone,
    pub last_error: Option<String>,
    pub accepted_at: Option<DateTimeWithTimeZone>,
    pub lease_token: Option<Uuid>,
    pub lease_until: Option<DateTimeWithTimeZone>,
    pub dedupe_key: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
