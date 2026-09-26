use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use kura_server::{Registry, router};
use tracing_subscriber::EnvFilter;

/// kura のレジストリ API サーバー。
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Directory containing registry/*.json.
    #[arg(long, env = "KURA_REGISTRY_DIR", default_value = concat!(env!("CARGO_MANIFEST_DIR"), "/../../registry"))]
    registry: PathBuf,
    /// Address to listen on.
    #[arg(long, env = "KURA_ADDR", default_value = "127.0.0.1:8080")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kura_server=info,tower_http=info".into()),
        )
        .init();

    let args = Args::parse();
    let registry = Registry::load(&args.registry)
        .with_context(|| format!("loading {}", args.registry.display()))?;
    tracing::info!(parts = registry.len(), dir = %args.registry.display(), "registry loaded");

    let listener = tokio::net::TcpListener::bind(args.addr)
        .await
        .with_context(|| format!("binding {}", args.addr))?;
    tracing::info!(addr = %args.addr, "listening");
    axum::serve(listener, router(registry))
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("shutting down");
        })
        .await?;
    Ok(())
}
