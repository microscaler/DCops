//! Process entrypoint for the DCops PXE HTTP server.

use tracing::info;
use tracing_subscriber::EnvFilter;

use pxe_server::{serve_http, K8sBootStore, PxeError, ServerConfig};

#[tokio::main]
async fn main() -> Result<(), PxeError> {
    // rustls 0.23 refuses to auto-select a CryptoProvider when more than one is
    // present in the dependency graph (kube pulls in aws-lc-rs; this crate enables
    // the `ring` feature). Install ring explicitly before any TLS is used — the
    // kube client built in K8sBootStore below constructs a rustls ClientConfig and
    // panics without a default provider installed.
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("pxe-server starting");
    let config = ServerConfig::from_env()?;
    let boot_store = std::sync::Arc::new(K8sBootStore::new().await?);
    serve_http(config, boot_store).await
}
