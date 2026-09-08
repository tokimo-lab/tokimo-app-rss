use serde::Serialize;
use tokimo_bus_client::BusClient;
use tokimo_bus_protocol::CallerCtx;

use crate::AppError;

#[derive(Serialize)]
pub struct NotifyRequest {
    pub user_id: String,
    pub app_id: &'static str,
    pub category_id: &'static str,
    pub category_label: &'static str,
    pub title: String,
    pub body: String,
    pub level: &'static str,
    pub action: Option<serde_json::Value>,
    pub dedupe_key: String,
}

pub async fn notify(client: &BusClient, request: NotifyRequest) -> Result<(), AppError> {
    let user_id = request.user_id.clone();
    let payload =
        serde_json::to_vec(&request).map_err(|error| AppError::internal(format!("serialize notify: {error}")))?;
    client
        .invoke(
            "notification_center",
            "notify",
            payload,
            CallerCtx {
                user_id: Some(user_id),
                caller_app_id: Some("rss".to_string()),
                request_id: format!("rss-notify-{}", request.dedupe_key),
                workspace: None,
            },
        )
        .await
        .map_err(|error| AppError::internal(format!("notification_center.notify: {error}")))?;
    Ok(())
}
