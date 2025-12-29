//! Mock Kubernetes client creation
//!
//! Creates a kube::Client from a mock HTTP service using tower-test.
//!
//! This follows the kube-rs recommended pattern for testing:
//! https://kube.rs/controllers/testing/

#[cfg(test)]
use kube::Client;
#[cfg(test)]
use tower_test::mock;
#[cfg(test)]
use hyper::http::{Request, Response};
// Note: tower-test 0.4 uses hyper 0.14, which has hyper::Body
#[cfg(test)]
use hyper::Body;

/// Create a mock Kubernetes client for testing
/// 
/// This creates a mock client using tower-test, following kube-rs's recommended pattern.
/// The mock service handle can be used to set up expected request/response pairs.
/// 
/// ## Example
/// 
/// ```rust,no_run
/// use tower_test::mock;
/// use hyper::Body;
/// 
/// let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
/// let client = create_mock_kube_client(mock_service, "default").await?;
/// 
/// // Set up expected API interactions using handle
/// // Then use client to create Api instances
/// ```
/// 
/// ## kube-rs Support
/// 
/// kube-rs recommends using `tower-test` for mocking. According to kube-rs documentation:
/// ```rust,ignore
/// let (mock_service, handle) = tower_test::mock::pair::<Request<Body>, Response<Body>>();
/// let client = Client::new(mock_service, "default");
/// ```
/// 
/// kube 2.0's Client API may have changed. This function uses the recommended pattern.
pub async fn create_mock_kube_client(
    mock_service: impl tower::Service<Request<Body>, Response = Response<Body>> + Send + Clone + 'static,
    default_namespace: &str,
) -> Result<Client, kube::Error> {
    // kube-rs recommended pattern: Client::new(mock_service, default_namespace)
    // According to kube-rs docs, this should work with tower-test mocks
    Client::new(mock_service, default_namespace)
}

