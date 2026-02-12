use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::io::stdio;
use tracing_subscriber::EnvFilter;

use crate::mcp::CtxhelprServer;

pub async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting ctxhelpr MCP server");

    let service = CtxhelprServer::new()
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
    Ok(())
}
