//! HTTP-only PXE server orchestration (DHCP/TFTP deferred).

use crate::config::ServerConfig;
use crate::error::PxeError;
use crate::http;
use crate::store::SharedBootStore;

/// Run the HTTP server until it exits.
pub async fn run_http(config: ServerConfig, boot_store: SharedBootStore) -> Result<(), PxeError> {
    http::serve(config, boot_store).await
}
