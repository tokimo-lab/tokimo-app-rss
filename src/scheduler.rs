use std::{collections::BTreeMap, sync::Arc, time::Duration};

use sea_orm::DatabaseConnection;
use tokimo_bus_client::BusClient;
use tracing::{info, warn};

use crate::{
    db::{entities::sources, repos::sources_repo::SourcesRepo},
    services::{collector, dispatcher},
};

pub fn start(db: DatabaseConnection, bus_client: Arc<BusClient>) {
    info!("rss scheduler: starting");
    dispatcher::start(db.clone(), bus_client);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            interval.tick().await;
            if let Err(error) = run_collection_cycle(&db).await {
                warn!(error = %error.message, "rss scheduler collection cycle failed");
            }
        }
    });
}

async fn run_collection_cycle(db: &DatabaseConnection) -> Result<(), crate::AppError> {
    let due = SourcesRepo::due(db, 100).await?;
    let mut groups: BTreeMap<String, Vec<sources::Model>> = BTreeMap::new();
    for source in due {
        groups.entry(source.normalized_url.clone()).or_default().push(source);
    }
    for sources in groups.into_values() {
        let mut leased = Vec::new();
        for source in sources {
            if let Some(pair) =
                SourcesRepo::acquire_lease(db, source.user_id, source.id, collector::COLLECTION_LEASE_SECONDS).await?
            {
                leased.push(pair);
            }
        }
        if leased.is_empty() {
            continue;
        }
        let all_initialized = leased.iter().all(|(source, _)| source.initialized_at.is_some());
        let same_etag = leased.iter().all(|(source, _)| source.etag == leased[0].0.etag);
        let same_modified = leased
            .iter()
            .all(|(source, _)| source.last_modified == leased[0].0.last_modified);
        let etag = (all_initialized && same_etag)
            .then(|| leased[0].0.etag.as_deref())
            .flatten();
        let last_modified = (all_initialized && same_modified)
            .then(|| leased[0].0.last_modified.as_deref())
            .flatten();
        match collector::fetch(&leased[0].0.url, etag, last_modified).await {
            Ok(outcome) => {
                for (source, token) in leased {
                    if let Err(error) = collector::persist_leased(db, &source, token, outcome.clone()).await {
                        warn!(source_id = %source.id, error = %error.message, "rss source persistence failed");
                    }
                }
            }
            Err(error) => {
                for (source, token) in leased {
                    if let Err(db_error) = collector::fail_leased(db, &source, token, &error).await {
                        warn!(source_id = %source.id, error = %db_error.message, "rss source failure state update failed");
                    }
                }
            }
        }
    }
    Ok(())
}
