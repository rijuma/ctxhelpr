use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use crate::config::ConfigCache;
use crate::indexer::Indexer;
use crate::mcp::CtxhelprServer;
use crate::watcher;

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting ctxhelpr MCP server");

    let indexer = Arc::new(Indexer::new());
    let config_cache = Arc::new(ConfigCache::new());

    let watcher_handle = watcher::start(indexer.clone(), config_cache.clone()).await;

    let service = CtxhelprServer::new(indexer, config_cache, watcher_handle)
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
    Ok(())
}
