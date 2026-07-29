//! Static HTTP route tests (M1).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use std::fs;
use tempfile::TempDir;
use tower::ServiceExt;
use tower_http::services::ServeDir;

#[tokio::test]
async fn serves_static_file_under_pxe_root() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("ipxe")).unwrap();
    fs::write(tmp.path().join("ipxe/hello.txt"), "hello-pxe").unwrap();

    let app = Router::new().fallback_service(ServeDir::new(tmp.path()));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/ipxe/hello.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"hello-pxe");
}

#[tokio::test]
async fn health_route_returns_ok() {
    use axum::routing::get;

    let app = Router::new().route("/healthz", get(|| async { "ok" }));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
