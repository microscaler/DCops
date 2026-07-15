//! HTTP server for iPXE boot file delivery and boot API routes.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::BootConfig;
use crate::boot::{render_ipxe_script, BootResolution};
use crate::config::ServerConfig;
use crate::error::PxeError;
use crate::store::SharedBootStore;

/// Shared axum state.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub boot_store: SharedBootStore,
}

/// Build the HTTP router.
pub fn router(state: AppState) -> Router {
    let static_root = state.config.pxe_root.clone();
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/boot/{mac}", get(get_boot_json))
        .route("/ipxe/boot/{mac}", get(get_boot_script))
        .fallback_service(ServeDir::new(static_root))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn get_boot_json(
    State(state): State<AppState>,
    Path(mac): Path<String>,
) -> Result<axum::Json<BootConfig>, ApiError> {
    match state.boot_store.resolve_mac(&mac).await {
        Ok(BootResolution::Profile(cfg)) => Ok(axum::Json(cfg)),
        Ok(BootResolution::LocalBootOnly) => Err(ApiError::locked(&mac)),
        Err(e) => Err(ApiError::from_pxe(e)),
    }
}

async fn get_boot_script(
    State(state): State<AppState>,
    Path(mac): Path<String>,
) -> Result<Response, ApiError> {
    let resolution = state.boot_store.resolve_mac(&mac).await.map_err(ApiError::from_pxe)?;

    let body = match resolution {
        BootResolution::LocalBootOnly => read_localboot_script(&state)?,
        BootResolution::Profile(cfg) => render_ipxe_script(&cfg),
    };

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        body,
    )
        .into_response())
}

fn read_localboot_script(state: &AppState) -> Result<String, ApiError> {
    let path = state.config.path_under_root("ipxe/localboot.ipxe");
    std::fs::read_to_string(&path).map_err(|e| {
        ApiError::internal(format!("read {}: {e}", path.display()))
    })
}

/// Start the HTTP server (runs until shutdown).
pub async fn serve(config: ServerConfig, boot_store: SharedBootStore) -> Result<(), PxeError> {
    let listen = config.http_listen;
    info!(%listen, root = %config.pxe_root.display(), "starting pxe-server HTTP");

    let state = AppState { config: Arc::new(config), boot_store };
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(listen)
        .await
        .map_err(|e| PxeError::Http(format!("bind {listen}: {e}")))?;

    axum::serve(listener, app).await.map_err(|e| PxeError::Http(format!("serve: {e}")))?;

    Ok(())
}

/// HTTP API errors mapped to status codes.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn locked(mac: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: format!("BootIntent for {mac} is locked — local disk boot only"),
        }
    }

    fn internal(message: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message,
        }
    }

    fn from_pxe(err: PxeError) -> Self {
        let status = match &err {
            PxeError::NotFound(_) => StatusCode::NOT_FOUND,
            PxeError::Configuration(msg) if msg.contains("invalid MAC") => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, self.message).into_response()
    }
}
