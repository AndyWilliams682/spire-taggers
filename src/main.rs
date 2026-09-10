//! Main entry point for the Spire Tagger application.
//!
//! This module sets up logging, loads the application state (the catalog of cards/tags)
//! from a SQLite database, configures the Axum web server, and starts listening for requests.

use anyhow::Context;
use spire_tagger::{AppState, data::Catalog, web};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::trace::TraceLayer;

/// The main entry point of the async application.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure logging using the `RUST_LOG` environment variable.
    // Defaults to INFO level for this app and the HTTP tower layer.
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "spire_tagger=info,tower_http=info".to_string()),
        )
        .init();

    // Determine the database path from the `SPIRE_DB` env var, falling back to a default asset path.
    let db_path = std::env::var("SPIRE_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("assets/spire_tagger.db"));
        
    // Determine the host and port to bind to from `SPIRE_BIND`, falling back to localhost:8000.
    let bind = std::env::var("SPIRE_BIND").unwrap_or_else(|_| "127.0.0.1:8000".to_string());

    // Load the catalog of cards and tags into memory from the SQLite DB.
    let catalog = Catalog::load(&db_path)
        .with_context(|| format!("failed to load catalog from {}", db_path.display()))?;
        
    // Wrap the catalog in an Arc within AppState so it can be shared across multiple request threads safely.
    let state = AppState::new(Arc::new(catalog));
    
    // Set up the Axum router with our application state and an HTTP tracing middleware layer.
    let app = web::router(state).layer(TraceLayer::new_for_http());

    let addr: SocketAddr = bind.parse().context("SPIRE_BIND must be host:port")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Spire Tagger running at http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Waits for a `Ctrl+C` signal to gracefully shut down the Axum server.
async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_err() {
        std::future::pending::<()>().await;
    }
}
