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
    #[ignore] // Example test - not meant to run yet
    async fn example_mock_client_usage() {
        // kube-rs recommended pattern from https://kube.rs/controllers/testing/
        let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
        
        // Create mock client
        let client_result = create_mock_kube_client(mock_service, "default").await;
        
        // This should work if kube 2.0 supports Client::new with a service
        match client_result {
            Ok(client) => {
                // Client created successfully - can now use it in tests
                println!("Mock client created successfully!");
                // Use client to create Api instances and test reconcilers
            }
            Err(e) => {
                // If this fails, kube 2.0 may have changed the API
                // We'll need to use an alternative approach
                eprintln!("Failed to create mock client: {:?}", e);
            }
        }
    }
}

