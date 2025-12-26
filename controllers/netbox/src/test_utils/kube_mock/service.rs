//! Mock Kubernetes HTTP service
//!
//! Uses tower-test to create a mock HTTP service that emulates the Kubernetes API server.

use tower_test::mock::{self, Handle};
use http::{Request, Response};
use hyper::Body;
use std::sync::Arc;
use crate::test_utils::kube_mock::store::MockKubeStore;

/// Mock Kubernetes HTTP service
/// 
/// This wraps tower-test's mock service and provides a handle
/// for setting up expected request/response pairs.
#[cfg(test)]
pub struct MockKubeService {
    /// Handle for setting up expected interactions
    pub handle: Handle<Request<Body>, Response<Body>>,
    /// In-memory store for resources
    pub store: Arc<MockKubeStore>,
}

#[cfg(test)]
impl MockKubeService {
    /// Create a new mock service
    pub fn new() -> (Self, mock::Mock<Request<Body>, Response<Body>>) {
        let (mock, handle) = mock::pair::<Request<Body>, Response<Body>>();
        let service = Self {
            handle,
            store: Arc::new(MockKubeStore::new()),
        };
        (service, mock)
    }

    /// Get the store for direct resource manipulation
    pub fn store(&self) -> Arc<MockKubeStore> {
        self.store.clone()
    }
}

#[cfg(test)]
impl Default for MockKubeService {
    fn default() -> Self {
        let (service, _mock) = Self::new();
        service
    }
}

