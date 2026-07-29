//! Runtime configuration from environment variables.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use crate::error::PxeError;

/// PXE HTTP server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Root directory for static boot artifacts (`ipxe/`, `cylon-regenesis/`, …).
    pub pxe_root: PathBuf,
    /// HTTP listen address (e.g. `0.0.0.0:8080`).
    pub http_listen: SocketAddr,
}

impl ServerConfig {
    /// Load configuration from the process environment.
    pub fn from_env() -> Result<Self, PxeError> {
        let pxe_root = std::env::var("PXE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/var/lib/pxe"));

        if !pxe_root.is_dir() {
            return Err(PxeError::Configuration(format!(
                "PXE_ROOT is not a directory: {}",
                pxe_root.display()
            )));
        }

        let listen_raw =
            std::env::var("HTTP_LISTEN").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let http_listen = listen_raw.parse::<SocketAddr>().map_err(|e| {
            PxeError::Configuration(format!("invalid HTTP_LISTEN '{listen_raw}': {e}"))
        })?;

        Ok(Self {
            pxe_root,
            http_listen,
        })
    }

    /// Path to a file under [`Self::pxe_root`].
    pub fn path_under_root(&self, relative: &str) -> PathBuf {
        self.pxe_root.join(relative)
    }

    /// Whether `path` is contained within [`Self::pxe_root`] (prevents traversal).
    pub fn is_under_root(&self, path: &Path) -> bool {
        path.starts_with(&self.pxe_root)
    }
}
