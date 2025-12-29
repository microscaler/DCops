//! Unit tests for SecretFetcher trait and implementations

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_fetcher::{SecretFetcher, RealSecretFetcher, mock::MockSecretFetcher};
    use k8s_openapi::api::core::v1::Secret;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_mock_secret_fetcher_get_secret_success() {
        let mut secrets = HashMap::new();
        secrets.insert("default/test-secret".to_string(), "test-token-value".to_string());
        let fetcher = MockSecretFetcher::new(Arc::new(Mutex::new(secrets)));

        let secret = fetcher.get_secret("default", "test-secret").await.unwrap();

        assert_eq!(secret.metadata.name, Some("test-secret".to_string()));
        assert_eq!(secret.metadata.namespace, Some("default".to_string()));
        assert!(secret.data.is_some());
        let data = secret.data.unwrap();
        assert!(data.contains_key("token"));
        let token_bytes = data.get("token").unwrap();
        let token = String::from_utf8(token_bytes.0.clone()).unwrap();
        assert_eq!(token, "test-token-value");
    }

    #[tokio::test]
    async fn test_mock_secret_fetcher_get_secret_not_found() {
        let secrets = HashMap::new();
        let fetcher = MockSecretFetcher::new(Arc::new(Mutex::new(secrets)));

        let result = fetcher.get_secret("default", "nonexistent-secret").await;

        assert!(result.is_err());
        if let Err(kube::Error::Api(err)) = result {
            assert_eq!(err.code, 404);
            assert!(err.message.contains("not found"));
        } else {
            panic!("Expected Api error with 404 code");
        }
    }

    #[tokio::test]
    async fn test_mock_secret_fetcher_multiple_secrets() {
        let mut secrets = HashMap::new();
        secrets.insert("default/secret1".to_string(), "token1".to_string());
        secrets.insert("default/secret2".to_string(), "token2".to_string());
        secrets.insert("namespace1/secret1".to_string(), "token3".to_string());
        let fetcher = MockSecretFetcher::new(Arc::new(Mutex::new(secrets)));

        let secret1 = fetcher.get_secret("default", "secret1").await.unwrap();
        let secret2 = fetcher.get_secret("default", "secret2").await.unwrap();
        let secret3 = fetcher.get_secret("namespace1", "secret1").await.unwrap();

        let data1 = secret1.data.unwrap();
        let token1 = String::from_utf8(data1.get("token").unwrap().0.clone()).unwrap();
        assert_eq!(token1, "token1");

        let data2 = secret2.data.unwrap();
        let token2 = String::from_utf8(data2.get("token").unwrap().0.clone()).unwrap();
        assert_eq!(token2, "token2");

        let data3 = secret3.data.unwrap();
        let token3 = String::from_utf8(data3.get("token").unwrap().0.clone()).unwrap();
        assert_eq!(token3, "token3");
    }

    #[tokio::test]
    async fn test_mock_secret_fetcher_namespace_isolation() {
        let mut secrets = HashMap::new();
        secrets.insert("default/secret1".to_string(), "token-default".to_string());
        secrets.insert("namespace1/secret1".to_string(), "token-namespace1".to_string());
        let fetcher = MockSecretFetcher::new(Arc::new(Mutex::new(secrets)));

        let default_secret = fetcher.get_secret("default", "secret1").await.unwrap();
        let ns1_secret = fetcher.get_secret("namespace1", "secret1").await.unwrap();

        let default_data = default_secret.data.unwrap();
        let default_token = String::from_utf8(default_data.get("token").unwrap().0.clone()).unwrap();
        assert_eq!(default_token, "token-default");

        let ns1_data = ns1_secret.data.unwrap();
        let ns1_token = String::from_utf8(ns1_data.get("token").unwrap().0.clone()).unwrap();
        assert_eq!(ns1_token, "token-namespace1");
    }
}

