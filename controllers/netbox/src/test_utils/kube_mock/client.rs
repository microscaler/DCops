//! Mock Kubernetes client creation
//!
//! Creates a kube::Client from a mock HTTP service using tower-test.
//!
//! This follows the kube-rs recommended pattern for testing:
//! https://kube.rs/controllers/testing/
//!
//! Note: kube 2.0 uses its own Body type, so we need to adapt the tower-test mock.

#[cfg(test)]
use kube::Client;
#[cfg(test)]
use tower_test::mock;
#[cfg(test)]
use http::Request;
#[cfg(test)]
use kube::client::Body as KubeBody;

/// Create a mock Kubernetes client for testing
/// 
/// This creates a mock client using tower-test, following kube-rs's recommended pattern.
/// The mock service handle can be used to set up expected request/response pairs.
/// 
/// ## Example
/// 
/// ```rust,no_run
/// use tower_test::mock;
/// use kube::client::Body;
/// 
/// let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
/// let client = create_mock_kube_client(mock_service, "default");
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
/// kube 2.0's Client::new returns Client directly (not Result) and uses kube::client::Body.
pub fn create_mock_kube_client<S>(
    mock_service: S,
    default_namespace: &str,
) -> Client
where
    S: tower::Service<Request<KubeBody>> + Send + Clone + 'static,
    S::Response: tower::Service<Request<KubeBody>> + Send,
    S::Error: Into<kube::Error>,
{
    // kube-rs recommended pattern: Client::new(mock_service, default_namespace)
    // Client::new returns Client directly (not Result) in kube 2.0
    Client::new(mock_service, default_namespace)
}

