//! RSS control plane and embedded UI assets over the app's local data-plane socket.

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch, post},
};
use tokimo_bus_protocol::{BusListener, DataPlaneSocket};
use tracing::{error, info};

use crate::{assets, handlers, handlers::AppCtx};

/// 起 axum server 监听本地 socket，返回 `DataPlaneSocket` 用于上报 broker。
pub async fn spawn(service: &str, ctx: Arc<AppCtx>) -> anyhow::Result<DataPlaneSocket> {
    let (listener, socket) = BusListener::bind_for_app(service)?;
    info!(?socket, "rss: app server listening");

    let router = build_router(ctx);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            error!(error = %e, "rss: app server stopped");
        }
    });

    Ok(socket)
}

fn build_router(ctx: Arc<AppCtx>) -> Router {
    Router::new()
        .route("/sources", get(handlers::sources_list).post(handlers::sources_create))
        .route("/sources/test", post(handlers::sources_test))
        .route("/sources/{id}", patch(handlers::sources_patch))
        .route("/sources/{id}/refresh", post(handlers::sources_refresh))
        .route("/entries", get(handlers::entries_list))
        .route("/entries/{id}", get(handlers::entries_get))
        .route(
            "/views",
            get(handlers::saved_views_list).post(handlers::saved_views_create),
        )
        .route(
            "/views/{id}",
            patch(handlers::saved_views_patch).delete(handlers::saved_views_delete),
        )
        .route("/rules", get(handlers::rules_list).post(handlers::rules_create))
        .route("/rules/preview", post(handlers::rules_preview))
        .route(
            "/rules/{id}",
            patch(handlers::rules_patch).delete(handlers::rules_delete),
        )
        .route("/deliveries", get(handlers::deliveries_list))
        .route("/notifications/test", post(handlers::notifications_test))
        .route("/assets/{*path}", get(assets::serve))
        .with_state(ctx)
}
