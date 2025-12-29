//! Trait for fetching Kubernetes Secrets
//!
//! This trait abstracts secret fetching to enable mocking in tests.

use k8s_openapi::api::core::v1::Secret;
use kube::Error as KubeError;

/// Trait for fetching Kubernetes Secrets
#[async_trait::async_trait]
pub trait SecretFetcher: Send + Sync {
    /// Get a secret by name and namespace
    async fn get_secret(&self, namespace: &str, name: &str) -> Result<Secret, KubeError>;
}

/// Real implementation using kube::Api<Secret>
pub struct RealSecretFetcher {
    kube_client: kube::Client,
}

impl RealSecretFetcher {
    pub fn new(kube_client: kube::Client) -> Self {
        Self { kube_client }
    }
}

#[async_trait::async_trait]
impl SecretFetcher for RealSecretFetcher {
    async fn get_secret(&self, namespace: &str, name: &str) -> Result<Secret, KubeError> {
        use kube::Api;
        let secret_api: Api<Secret> = Api::namespaced(self.kube_client.clone(), namespace);
        secret_api.get(name).await
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, Mutex};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

    /// Mock implementation using in-memory storage
    pub struct MockSecretFetcher {
        secrets: Arc<Mutex<HashMap<String, String>>>, // namespace/secret_name -> token
    }

    impl MockSecretFetcher {
        pub fn new(secrets: Arc<Mutex<HashMap<String, String>>>) -> Self {
            Self { secrets }
        }
    }

    #[async_trait::async_trait]
    impl SecretFetcher for MockSecretFetcher {
        async fn get_secret(&self, namespace: &str, name: &str) -> Result<Secret, KubeError> {
            let key = format!("{}/{}", namespace, name);
            let secrets = self.secrets.lock().unwrap();
            secrets.get(&key).map(|token| {
                // Create a Secret object from the stored token
                let mut data = BTreeMap::new();
                let token_bytes = token.as_bytes().to_vec();
                data.insert("token".to_string(), k8s_openapi::ByteString(token_bytes));
                
                Secret {
                    metadata: ObjectMeta {
                        name: Some(name.to_string()),
                        namespace: Some(namespace.to_string()),
                        ..Default::default()
                    },
                    data: Some(data),
                    ..Default::default()
                }
            }).ok_or_else(|| {
                KubeError::Api(kube::error::ErrorResponse {
                    code: 404,
                    message: format!("Secret {} not found in namespace {}", name, namespace),
                    reason: "NotFound".to_string(),
                    status: "Failure".to_string(),
                })
            })
        }
    }
}

