//! Resident RSS sidecar: axum over UDS, PostgreSQL persistence, and background scheduling.

use clap::{CommandFactory, Parser};
use std::sync::{Arc, OnceLock};
use tokimo_app_rss::{app_server, db, handlers, scheduler};
use tokimo_bus_client::{BusClient, ClientConfig};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "tokimo-app-rss", about = "Tokimo RSS resident sidecar")]
struct Cli {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    if std::env::var_os("TOKIMO_BUS_SOCKET").is_none() {
        let mut command = Cli::command();
        tokimo_bus_cli::print_help_unified(&mut command);
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tokimo_app_rss=debug".into()),
        )
        .init();
    if let Err(error) = run_server().await {
        error!(%error, "rss: fatal");
        std::process::exit(1);
    }
    Ok(())
}

async fn run_server() -> anyhow::Result<()> {
    let cfg = ClientConfig::from_env().map_err(|error| anyhow::anyhow!("ClientConfig: {error}"))?;
    let db = db::init_pool().await?;
    let client_slot: Arc<OnceLock<Arc<BusClient>>> = Arc::new(OnceLock::new());
    let ctx = Arc::new(handlers::AppCtx {
        db: db.clone(),
        client: Arc::clone(&client_slot),
    });
    let socket = app_server::spawn("rss", ctx).await?;
    let client = BusClient::builder(cfg)
        .service("rss", env!("CARGO_PKG_VERSION"))
        .data_plane(socket)
        .build()
        .await
        .map_err(|error| anyhow::anyhow!("bus build: {error}"))?;
    client_slot
        .set(Arc::clone(&client))
        .map_err(|_| anyhow::anyhow!("bus client already set"))?;
    scheduler::start(db, Arc::clone(&client));
    info!("rss: registered with broker; scheduler restored");
    let shutdown = tokio::spawn({
        let client = Arc::clone(&client);
        async move { client.run_until_shutdown().await }
    });
    tokio::select! { _ = tokio::signal::ctrl_c() => client.shutdown(), _ = shutdown => {} }
    Ok(())
}
