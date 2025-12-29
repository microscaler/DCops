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
    async fn test_mock_client_creation() {
        // kube-rs recommended pattern from https://kube.rs/controllers/testing/
        let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
        
        // Create mock client - this will fail at compile time if kube 2.0 API changed
        let client_result = create_mock_kube_client(mock_service, "default").await;
        
        // Assert client was created successfully
        assert!(client_result.is_ok(), "Failed to create mock client: {:?}", client_result.err());
        let _client = client_result.unwrap();
        
        // If we get here, kube 2.0 supports Client::new with tower-test mocks!
        // We can now use this in our reconciler tests
    }
}

