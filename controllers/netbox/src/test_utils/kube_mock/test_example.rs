//! Example test showing how to use kube-rs mocks with tower-test
//!
//! This demonstrates the kube-rs recommended pattern for testing.

#[cfg(test)]
mod tests {
    use super::super::client::create_mock_kube_client;
    use tower_test::mock;
    use hyper::{Body, Request, Response};
    use kube::Client;

    #[tokio::test]
    #[ignore] // Ignored - direct kube::Client mocking blocked by Body type mismatch
    async fn test_mock_client_creation() {
        // NOTE: This test is ignored because direct kube::Client mocking with tower-test
        // is blocked by Body type mismatch in kube 2.0 (hyper::Body vs kube::client::Body).
        // Use MockTokenResolver instead for testing without a real kube::Client.
        
        // kube-rs recommended pattern from https://kube.rs/controllers/testing/
        // let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
        
        // Create mock client - blocked by Body type mismatch
        // let client = create_mock_kube_client(mock_service, "default");
        
        // Use MockTokenResolver instead - see test_utils/mock_token_resolver.rs
        unimplemented!("Use MockTokenResolver for testing instead")
    }
}

