# Controller Reconciler Architecture - End-to-End Guide

This document provides a comprehensive overview of what a complete controller reconciler looks like in the NetBox Controller, from CRD definition through to tests.

## Table of Contents

1. [CRD Definition](#crd-definition)
2. [CR Structure](#cr-structure)
3. [Reconciler Implementation](#reconciler-implementation)
4. [Watcher Setup](#watcher-setup)
5. [Event Emission](#event-emission)
6. [Error Handling & Backoff](#error-handling--backoff)
7. [Testing Strategy](#testing-strategy)
8. [Integration Points](#integration-points)

---

## 1. CRD Definition

The Custom Resource Definition (CRD) is defined in the `crds` crate, typically in a file like `crates/crds/src/dcim/site.rs`.

### Example: NetBoxSite CRD

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSite {
    #[serde(flatten)]
    pub metadata: ObjectMeta,
    
    pub spec: NetBoxSiteSpec,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<NetBoxSiteStatus>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteSpec {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub tenant: NetBoxResourceReference,  // Required dependency
    pub region: Option<NetBoxResourceReference>,  // Optional dependency
    pub status: SiteStatus,
    // ... other fields
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetBoxSiteStatus {
    pub netbox_id: Option<u64>,
    pub netbox_url: Option<String>,
    pub state: ResourceState,  // Pending, Created, Updated, Failed
    pub error: Option<String>,
    pub last_reconciled: Option<String>,
}
```

**Key Points:**
- Uses `schemars` for JSON schema generation (required for Kubernetes CRD validation)
- Uses `serde` for serialization/deserialization
- Status is optional (None = resource not yet created)
- Spec contains desired state
- Status contains actual state (netbox_id, netbox_url, state)

---

## 2. CR Structure

A Custom Resource (CR) is an instance of the CRD, created by users via YAML or kubectl.

### Example: NetBoxSite CR (YAML)

```yaml
apiVersion: dcops.microscaler.io/v1
kind: NetBoxSite
metadata:
  name: datacenter-1
  namespace: default
spec:
  name: "Data Center 1"
  slug: "datacenter-1"
  description: "Primary datacenter"
  tenant:
    apiGroup: dcops.microscaler.io
    kind: NetBoxTenant
    name: datacenter-tenant
    namespace: default
  status: Active
status:
  netboxId: 42
  netboxUrl: "http://netbox/api/dcim/sites/42/"
  state: Created
  error: null
```

**Key Points:**
- `metadata.name` and `metadata.namespace` identify the resource
- `spec` contains desired state (what user wants)
- `status` contains actual state (what controller has created)
- Dependencies are referenced via `NetBoxResourceReference` (tenant, region, etc.)

---

## 3. Reconciler Implementation

The reconciler is the core logic that reconciles desired state (CR) with actual state (NetBox).

### File Structure

```
controllers/netbox/src/reconciler/
├── mod.rs              # Main Reconciler struct, backoff logic
├── dcim/
│   ├── mod.rs
│   └── site.rs         # NetBoxSite reconciler
└── dcim/
    └── site_test.rs     # Tests
```

### Example: NetBoxSite Reconciler

```rust
// controllers/netbox/src/reconciler/dcim/site.rs

impl Reconciler {
    /// Reconciles a NetBoxSite CR with NetBox
    pub async fn reconcile_netbox_site(
        &self,
        site_crd: &NetBoxSite,
    ) -> Result<(), ControllerError> {
        // 1. Extract name and namespace
        let (name, namespace) = extract_name_and_namespace(site_crd, "NetBoxSite")?;
        
        // 2. Resolve tenant dependency (required)
        let tenant_id = resolve_required_dependency_id(
            &self.netbox_tenant_api,
            &site_crd.spec.tenant.name,
            "NetBoxTenant",
            name,
            |crd| crd.status.as_ref(),
        ).await?;
        
        // 3. Validate status and check for drift
        let result = validate_status_and_drift(
            site_crd.status.as_ref(),
            "NetBoxSite",
            namespace,
            name,
            |id| async move {
                self.token_resolver
                    .resolve_token_for_tenant(tenant_id)
                    .await?;
                let client = self.token_resolver.get_client_for_tenant(tenant_id)?;
                client.get_site(SiteId(id)).await
            },
        ).await?;
        
        match result {
            DriftCheckResult::UseExisting(site) => {
                // 4a. Resource exists - check if update needed
                if needs_update(&site, site_crd) {
                    // Update in NetBox
                    let updated = update_site(&client, SiteId(site.id), site_crd).await?;
                    // Update status
                    self.update_site_status(name, namespace, &updated, ResourceState::Updated).await?;
                } else {
                    // Already up-to-date
                }
            }
            DriftCheckResult::Recreate | DriftCheckResult::StatusCleared { .. } => {
                // 4b. Resource doesn't exist or status cleared - create it
                let token = self.token_resolver
                    .resolve_token_for_tenant(tenant_id)
                    .await?;
                let client = self.token_resolver.get_client_for_tenant(tenant_id)?;
                
                match client.create_site(create_site_request(site_crd, tenant_id)).await {
                    Ok(site) => {
                        // Success - update status
                        self.update_site_status(name, namespace, &site, ResourceState::Created).await?;
                        // Emit event
                        self.record_event_normal(
                            reasons::CREATED,
                            &format!("Created site '{}' in NetBox (ID: {})", name, site.id),
                            site_crd,
                        ).await;
                    }
                    Err(NetBoxError::Conflict(_)) => {
                        // GitOps compliance: query for existing and use it
                        if let Ok(existing) = client.get_site_by_name(&site_crd.spec.name).await {
                            self.update_site_status(name, namespace, &existing, ResourceState::Created).await?;
                        } else {
                            return Err(ControllerError::NetBox(e));
                        }
                    }
                    Err(e) => {
                        // Other error - update status with error
                        self.update_site_status_error(name, namespace, &format!("{}", e)).await?;
                        return Err(ControllerError::NetBox(e));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Helper to update site status
    async fn update_site_status(
        &self,
        name: &str,
        namespace: &str,
        site: &netbox_client::Site,
        state: ResourceState,
    ) -> Result<(), ControllerError> {
        let status_patch = Self::create_resource_status_patch(
            site.id,
            site.url.clone(),
            state,
            None,
        );
        update_resource_status(
            &self.netbox_site_api,
            name,
            namespace,
            &status_patch,
            "NetBoxSite",
            site.id,
        ).await
    }
}
```

**Key Patterns:**
1. **Dependency Resolution**: Use `resolve_required_dependency_id` or `resolve_optional_dependency_id`
2. **Drift Detection**: Use `validate_status_and_drift` to check if resource exists
3. **Status Updates**: Always update CR status after operations
4. **Error Handling**: Update status with error message on failure
5. **GitOps Compliance**: Handle conflicts by querying for existing resources

---

## 4. Watcher Setup

The watcher monitors Kubernetes for CR changes and triggers reconciliation.

### File: `controllers/netbox/src/watcher.rs`

```rust
impl Watcher {
    /// Watches NetBoxSite resources
    pub async fn watch_netbox_sites(&self) -> Result<(), ControllerError> {
        watch_resource(
            self.netbox_site_api.clone(),
            self.reconciler.clone(),
            |reconciler: Arc<Reconciler>, site: Arc<NetBoxSite>| {
                let reconciler = reconciler.clone();
                let site = site.clone();
                async move {
                    reconciler.reconcile_netbox_site(&site).await
                }
            },
            "NetBoxSite",
        ).await
    }
}
```

### Generic `watch_resource` Helper

```rust
async fn watch_resource<K, F>(
    api: Api<K>,
    reconciler: Arc<Reconciler>,
    reconcile_fn: F,
    resource_name: &str,
) -> Result<(), ControllerError>
where
    K: kube::Resource + Clone + Send + Sync + 'static,
    F: Fn(Arc<Reconciler>, Arc<K>) -> Pin<Box<dyn Future<Output = Result<Action, ControllerError>> + Send>> + Send + Sync + Clone + 'static,
{
    // Error policy: Fibonacci backoff on errors
    let error_policy = move |obj: Arc<K>, error: &ControllerError, ctx: Arc<Reconciler>| {
        let resource_key = format!("{}/{}", namespace, name);
        ctx.increment_error(&resource_key);
        let (backoff, error_count) = ctx.get_backoff_for_resource(&resource_key);
        
        // Emit retry event
        tokio::spawn(async move {
            ctx.record_event_retry_attempt_str(&error.to_string(), error_count, backoff, &*obj).await;
        });
        
        Action::requeue(Duration::from_secs(backoff))
    };
    
    // Reconcile function
    let reconcile = move |obj: Arc<K>, ctx: Arc<Reconciler>| {
        async move {
            match reconcile_fn(ctx.clone(), obj.clone()).await {
                Ok(_) => {
                    // Reset error count on success
                    ctx.reset_error(&resource_key);
                    Ok(Action::requeue(Duration::from_secs(10)))  // Periodic reconciliation
                }
                Err(e) => Err(e),  // Error policy handles it
            }
        }
    };
    
    // Configure controller
    let controller_config = ControllerConfig::default()
        .debounce(Duration::from_secs(5))  // Batch status updates
        .concurrency(3);  // Max 3 concurrent reconciliations
    
    Controller::new(api, watcher::Config::default())
        .with_config(controller_config)
        .run(reconcile, error_policy, reconciler)
        .for_each(|res| async move {
            if let Err(e) = res {
                error!("Controller error: {}", e);
            }
        })
        .await;
    
    Ok(())
}
```

**Key Points:**
- Uses `kube_runtime::Controller` for automatic reconnection
- Error policy implements Fibonacci backoff
- Debounce batches status updates (reduces API load)
- Concurrency limits prevent resource exhaustion
- Always requeues on success (enables periodic reconciliation for drift detection)

---

## 5. Event Emission

Events provide visibility into reconciliation operations for SREs.

### Event Reasons (Defined in `events.rs`)

```rust
pub mod reasons {
    pub const CREATED: &str = "Created";
    pub const UPDATED: &str = "Updated";
    pub const RECONCILIATION_FAILED: &str = "ReconciliationFailed";
    pub const DEPENDENCY_NOT_FOUND: &str = "DependencyNotFound";
    pub const DRIFT_DETECTED: &str = "DriftDetected";
    pub const RETRY_ATTEMPT: &str = "RetryAttempt";
    pub const TOKEN_RESOLUTION_FAILED: &str = "TokenResolutionFailed";
}
```

### Event Recording in Reconciler

```rust
impl Reconciler {
    /// Record a Normal event
    pub async fn record_event_normal<K>(
        &self,
        reason: &str,
        message: &str,
        resource: &K,
    ) where
        K: kube::Resource + Send + Sync,
    {
        if let Some(recorder) = &self.event_recorder {
            recorder.record_normal(reason, message, resource).await;
        }
    }
    
    /// Record a Warning event
    pub async fn record_event_warning<K>(
        &self,
        reason: &str,
        message: &str,
        resource: &K,
    ) where
        K: kube::Resource + Send + Sync,
    {
        if let Some(recorder) = &self.event_recorder {
            recorder.record_warning(reason, message, resource).await;
        }
    }
}
```

### Usage in Reconciler

```rust
// On successful creation
self.record_event_normal(
    reasons::CREATED,
    &format!("Created site '{}' in NetBox (ID: {})", name, site.id),
    site_crd,
).await;

// On dependency not found
self.record_event_warning(
    reasons::DEPENDENCY_NOT_FOUND,
    &format!("Tenant '{}' not found or not created yet", tenant_name),
    site_crd,
).await;

// On retry attempt (in error policy)
self.record_event_retry_attempt_str(
    &error.to_string(),
    error_count,
    backoff,
    site_crd,
).await;
```

**Key Points:**
- Events are optional (can be None for testing)
- Normal events for successful operations
- Warning events for errors that will be retried
- Events are visible via `kubectl get events` in the resource's namespace

---

## 6. Error Handling & Backoff

The reconciler implements sophisticated error handling with exponential backoff.

### Backoff State Management

```rust
// In Reconciler struct
backoff_states: Arc<Mutex<HashMap<String, BackoffState>>>,

struct BackoffState {
    backoff: FibonacciBackoff,  // 1 min min, 10 min max
    error_count: u32,
}

impl Reconciler {
    /// Get backoff duration for a resource
    pub fn get_backoff_for_resource(&self, resource_key: &str) -> (u64, u32) {
        let mut states = self.backoff_states.lock().unwrap();
        let state = states.entry(resource_key.to_string())
            .or_insert_with(BackoffState::new);
        (state.backoff.next_backoff_seconds(), state.error_count)
    }
    
    /// Increment error count (called by error policy)
    pub fn increment_error(&self, resource_key: &str) {
        let mut states = self.backoff_states.lock().unwrap();
        let state = states.entry(resource_key.to_string())
            .or_insert_with(BackoffState::new);
        state.increment_error();
    }
    
    /// Reset error count (called on successful reconciliation)
    pub fn reset_error(&self, resource_key: &str) {
        let mut states = self.backoff_states.lock().unwrap();
        if let Some(state) = states.get_mut(resource_key) {
            state.reset();
        }
    }
}
```

### Fibonacci Backoff Sequence

```
Error 1: 60s  (1 minute)
Error 2: 60s  (1 minute)
Error 3: 120s (2 minutes)
Error 4: 180s (3 minutes)
Error 5: 300s (5 minutes)
Error 6: 480s (8 minutes)
Error 7: 600s (10 minutes - max cap)
Error 8+: 600s (stays at max)
```

**Key Points:**
- Each resource has independent backoff state
- Backoff resets on successful reconciliation
- Max cap prevents excessive delays
- Error policy in watcher uses backoff for requeue delays

---

## 7. Testing Strategy

Comprehensive testing at multiple levels ensures reliability.

### Unit Tests: `site_test.rs`

```rust
#[cfg(test)]
mod tests {
    use crate::test_utils::mock_token_resolver::{MockTokenResolver, create_test_reconciler_with_mock_token_resolver};
    use crate::test_utils::create_test_netbox_site;
    use crate::kube_api_trait::KubeApiTrait;
    
    #[tokio::test]
    async fn test_reconcile_site_create() {
        // 1. Setup: Create mocks
        let mock_token_resolver = Arc::new(MockTokenResolver::new("http://test-netbox".to_string()));
        let (reconciler, apis, mock_event_recorder) = create_test_reconciler_with_mock_token_resolver(mock_token_resolver);
        
        // 2. Setup: Create test data
        let site = create_test_netbox_site("test-site", "default", None, None);
        let tenant = create_test_netbox_tenant("datacenter-tenant", "default", Some(1), Some("http://...".to_string()));
        apis.tenant_api.store("datacenter-tenant".to_string(), tenant);
        apis.site_api.store("test-site".to_string(), site.clone());
        
        // 3. Setup: Add NetBox resources to mock
        let mock_client = mock_token_resolver.mock_client();
        mock_client.add_tenant(/* ... */);
        
        // 4. Execute: Reconcile
        let result = reconciler.reconcile_netbox_site(&site).await;
        
        // 5. Assert: Verify success
        assert!(result.is_ok());
        
        // 6. Assert: Verify status updated
        let updated = apis.site_api.get("test-site").await.unwrap();
        assert_eq!(updated.status.unwrap().netbox_id, Some(42));
        
        // 7. Assert: Verify event emitted
        assert_normal_event_emitted(&mock_event_recorder, reasons::CREATED)
            .expect("CREATED event should be emitted");
    }
    
    #[tokio::test]
    async fn test_reconcile_site_dependency_not_found() {
        // Test error path: dependency not found
        // ...
    }
    
    #[tokio::test]
    async fn test_reconcile_site_drift_detection() {
        // Test drift detection: resource deleted in NetBox
        // ...
    }
}
```

### Event Integration Tests: `events_integration_test.rs`

```rust
#[tokio::test]
async fn test_site_created_event_emission() {
    // Verify that CREATED events are emitted with correct content
    // ...
    assert_event_message_contains(&event, "Created site");
    assert_event_for_resource(&event, &site);
}
```

### Test Utilities

```rust
// test_utils/mock_token_resolver.rs
pub fn create_test_reconciler_with_mock_token_resolver(
    mock_token_resolver: Arc<MockTokenResolver>,
) -> (Reconciler, TestReconcilerApis, MockEventRecorder, MockSecretFetcher) {
    // Creates a fully mocked reconciler for testing
}

// test_utils/event_test_helpers.rs
pub fn assert_normal_event_emitted(
    recorder: &MockEventRecorder,
    reason: &str,
) -> Option<CapturedEvent> {
    // Helper to assert events were emitted
}
```

**Key Testing Patterns:**
1. **Mock Everything**: TokenResolver, KubeApi, EventRecorder, SecretFetcher
2. **Test All Paths**: Create, Update, Delete, Drift, Errors
3. **Verify Status**: Always check status updates
4. **Verify Events**: Assert events are emitted correctly
5. **Test Error Handling**: Dependency not found, network errors, conflicts

---

## 8. Integration Points

### Controller Initialization

```rust
// controllers/netbox/src/controller.rs

impl Controller {
    pub async fn new(
        netbox_url: String,
        namespace: Option<String>,
    ) -> Result<Self, ControllerError> {
        // 1. Create Kubernetes client
        let kube_client = Client::try_default().await?;
        
        // 2. Create TokenResolver (multi-tenant support)
        let token_resolver = Arc::new(TokenResolver::new(kube_client.clone(), netbox_url));
        
        // 3. Create API clients for all CRD types
        let netbox_site_api: Api<NetBoxSite> = Api::namespaced(kube_client.clone(), ns);
        // ... 19 more APIs
        
        // 4. Create SecretFetcher and EventRecorder
        let secret_fetcher = Arc::new(RealSecretFetcher::new(kube_client.clone()));
        let event_recorder = Some(Arc::new(RecorderWrapper::new(Recorder::new(kube_client.clone(), reporter))));
        
        // 5. Create Reconciler
        let reconciler = Reconciler::new(
            token_resolver,
            Some(secret_fetcher),
            event_recorder,
            // ... all 19 API wrappers
        );
        
        // 6. Run startup reconciliation (map existing NetBox resources to CRs)
        reconciler.startup_reconciliation().await?;
        
        // 7. Create Watcher
        let watcher = Watcher::new(reconciler.clone(), /* ... all APIs ... */);
        
        // 8. Spawn watcher tasks (one per CRD type)
        let netbox_site_watcher = tokio::spawn(async move {
            watcher.watch_netbox_sites().await
        });
        // ... 19 more watchers
        
        // 9. Return Controller with all JoinHandles
        Ok(Controller {
            netbox_site_watcher,
            // ... 19 more watchers
        })
    }
    
    pub async fn run(self) -> Result<(), ControllerError> {
        // Wait for any watcher to exit (they should run forever)
        tokio::select! {
            result = self.netbox_site_watcher => {
                result??;
            }
            // ... 19 more watchers
        }
        Ok(())
    }
}
```

### Main Entry Point

```rust
// controllers/netbox/src/main.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize logging
    tracing_subscriber::fmt::init();
    
    // 2. Get configuration
    let netbox_url = env::var("NETBOX_URL")?;
    let namespace = env::var("NAMESPACE").ok();
    
    // 3. Create and run controller
    let controller = Controller::new(netbox_url, namespace).await?;
    controller.run().await?;
    
    Ok(())
}
```

---

## Summary: Complete Flow

1. **User creates CR** → Kubernetes API server stores it
2. **Watcher detects change** → `kube_runtime::Controller` triggers reconciliation
3. **Reconciler runs** → Resolves dependencies, checks drift, creates/updates NetBox resource
4. **Status updated** → CR status patched with netbox_id, netbox_url, state
5. **Event emitted** → Kubernetes event created for SRE visibility
6. **Success/Error** → Error policy handles retries with backoff, or resets on success
7. **Periodic reconciliation** → Watcher requeues every 10s to detect drift

**Key Architectural Principles:**
- **GitOps Compliance**: Handle conflicts by querying for existing resources
- **Multi-Tenant**: TokenResolver resolves tenant-specific NetBox tokens
- **Observability**: Events provide visibility into all operations
- **Resilience**: Fibonacci backoff prevents thundering herd
- **Testability**: All dependencies are trait-based for easy mocking

This architecture ensures reliable, observable, and maintainable reconciliation of Kubernetes CRs with NetBox resources.

