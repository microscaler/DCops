# Multi-Tenant NetBox Client Architecture

## Overview

This document describes the multi-tenant architecture for the NetBox controller, where each tenant has its own NetBox API token stored in a Kubernetes Secret.

## Design Principles

1. **Single Point of Dependency Injection**: All token resolution happens in one place
2. **Per-Request Client Creation**: NetBoxClient is created per reconciliation with tenant-specific token
3. **Tenant Isolation**: Each tenant's resources use their own token
4. **Secret Security**: Tokens are stored in Kubernetes Secrets, referenced by Tenant CRD

## Architecture Components

### 1. Tenant CRD Enhancement

```rust
pub struct NetBoxTenantSpec {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub comments: Option<String>,
    pub group: Option<NetBoxResourceReference>,
    
    // NEW: Secret reference for tenant's NetBox API token
    pub token_secret: SecretReference,
}

pub struct SecretReference {
    pub name: String,           // Secret name
    pub namespace: Option<String>, // Optional namespace (defaults to CR namespace)
    pub key: Option<String>,    // Optional key (defaults to "token")
}
```

### 2. Token Resolver Service

**Single Point of Dependency Injection**

```rust
pub struct TokenResolver {
    kube_client: Client,
    netbox_url: String,
}

impl TokenResolver {
    /// Resolves token for a tenant reference
    /// This is the SINGLE POINT of token resolution
    pub async fn resolve_token(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<String, TokenResolutionError> {
        // 1. Fetch Tenant CRD
        // 2. Extract secret reference
        // 3. Fetch Secret
        // 4. Extract token
        // 5. Return token
    }
    
    /// Creates a NetBoxClient with resolved token
    pub async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<NetBoxClient, TokenResolutionError> {
        let token = self.resolve_token(namespace, tenant_ref).await?;
        NetBoxClient::new(self.netbox_url.clone(), token)
            .map_err(|e| TokenResolutionError::ClientCreation(e))
    }
}
```

### 3. Reconciler Changes

Reconcilers will:
- Extract tenant reference from resource
- Use `TokenResolver` to get tenant-specific client
- Perform operations with tenant-specific client

## Flow Diagrams

### Sequence Diagram: Token Resolution and Client Creation

```mermaid
sequenceDiagram
    participant Reconciler
    participant TokenResolver
    participant KubeAPI as Kubernetes API
    participant Secret as Kubernetes Secret
    participant NetBoxClient

    Reconciler->>Reconciler: Extract tenant reference<br/>from resource spec
    Reconciler->>TokenResolver: resolve_token(namespace, tenant_ref)
    
    TokenResolver->>KubeAPI: Get Tenant CRD<br/>(namespace, tenant_ref.name)
    KubeAPI-->>TokenResolver: Tenant CRD with<br/>token_secret reference
    
    TokenResolver->>TokenResolver: Extract secret reference<br/>(name, namespace, key)
    
    alt Secret namespace not specified
        TokenResolver->>TokenResolver: Use resource namespace
    end
    
    TokenResolver->>KubeAPI: Get Secret<br/>(secret_namespace, secret_name)
    KubeAPI-->>TokenResolver: Secret object
    
    TokenResolver->>TokenResolver: Extract token from Secret<br/>(key defaults to "token")
    TokenResolver-->>Reconciler: Token string
    
    Reconciler->>TokenResolver: create_client_for_tenant(namespace, tenant_ref)
    TokenResolver->>TokenResolver: resolve_token() (cached or fresh)
    TokenResolver->>NetBoxClient: new(netbox_url, token)
    NetBoxClient-->>TokenResolver: NetBoxClient instance
    TokenResolver-->>Reconciler: NetBoxClient with tenant token
    
    Reconciler->>NetBoxClient: API operations<br/>(create, update, query)
    NetBoxClient->>NetBoxClient: All requests use tenant token
```

### Flowchart: Multi-Tenant Reconciliation Flow

```mermaid
flowchart TD
    Start([Reconciler receives resource]) --> ExtractTenant[Extract tenant reference<br/>from resource.spec.tenant]
    
    ExtractTenant --> HasTenant{Has tenant<br/>reference?}
    HasTenant -->|No| Error1[Error: Tenant required]
    HasTenant -->|Yes| ResolveToken[TokenResolver.resolve_token<br/>namespace, tenant_ref]
    
    ResolveToken --> FetchTenant[Fetch Tenant CRD<br/>from Kubernetes]
    FetchTenant --> TenantExists{Tenant CRD<br/>exists?}
    TenantExists -->|No| Error2[Error: Tenant not found]
    TenantExists -->|Yes| ExtractSecret[Extract token_secret<br/>from Tenant.spec]
    
    ExtractSecret --> HasSecretRef{Has secret<br/>reference?}
    HasSecretRef -->|No| Error3[Error: Secret reference required]
    HasSecretRef -->|Yes| DetermineNS{Secret namespace<br/>specified?}
    
    DetermineNS -->|Yes| UseSecretNS[Use secret namespace]
    DetermineNS -->|No| UseResourceNS[Use resource namespace]
    
    UseSecretNS --> FetchSecret[Fetch Secret<br/>from Kubernetes]
    UseResourceNS --> FetchSecret
    
    FetchSecret --> SecretExists{Secret<br/>exists?}
    SecretExists -->|No| Error4[Error: Secret not found]
    SecretExists -->|Yes| ExtractKey[Extract token from Secret<br/>key defaults to 'token']
    
    ExtractKey --> HasToken{Token key<br/>exists?}
    HasToken -->|No| Error5[Error: Token key not found]
    HasToken -->|Yes| CreateClient[NetBoxClient::new<br/>netbox_url, token]
    
    CreateClient --> ClientValid{Client<br/>valid?}
    ClientValid -->|No| Error6[Error: Client creation failed]
    ClientValid -->|Yes| PerformOps[Perform NetBox operations<br/>with tenant-specific client]
    
    PerformOps --> End([Reconciliation complete])
    
    Error1 --> End
    Error2 --> End
    Error3 --> End
    Error4 --> End
    Error5 --> End
    Error6 --> End
```

### Component Architecture Diagram

```mermaid
graph TB
    subgraph "Controller Layer"
        Main[main.rs<br/>Reads NETBOX_URL env]
        Controller[Controller<br/>Creates TokenResolver]
    end
    
    subgraph "Token Resolution Layer - SINGLE POINT OF INJECTION"
        TokenResolver[TokenResolver<br/>resolve_token<br/>create_client_for_tenant]
    end
    
    subgraph "Reconciler Layer"
        SiteReconciler[Site Reconciler]
        PrefixReconciler[Prefix Reconciler]
        DeviceReconciler[Device Reconciler]
        OtherReconcilers[Other Reconcilers...]
    end
    
    subgraph "Kubernetes Resources"
        TenantCRD[NetBoxTenant CRD<br/>spec.token_secret]
        Secret[Kubernetes Secret<br/>data.token]
        ResourceCRDs[NetBoxSite CRD<br/>NetBoxPrefix CRD<br/>etc.]
    end
    
    subgraph "NetBox Client Layer"
        NetBoxClient[NetBoxClient<br/>Created per-request<br/>with tenant token]
    end
    
    subgraph "NetBox API"
        NetBoxAPI[NetBox REST API<br/>Authenticated per tenant]
    end
    
    Main --> Controller
    Controller --> TokenResolver
    Controller --> SiteReconciler
    Controller --> PrefixReconciler
    Controller --> DeviceReconciler
    Controller --> OtherReconcilers
    
    SiteReconciler -->|Uses| TokenResolver
    PrefixReconciler -->|Uses| TokenResolver
    DeviceReconciler -->|Uses| TokenResolver
    OtherReconcilers -->|Uses| TokenResolver
    
    TokenResolver -->|Fetches| TenantCRD
    TokenResolver -->|Fetches| Secret
    TokenResolver -->|Creates| NetBoxClient
    
    ResourceCRDs -->|References| TenantCRD
    
    NetBoxClient -->|API Calls| NetBoxAPI
    
    style TokenResolver fill:#ff9999,stroke:#333,stroke-width:4px
    style NetBoxClient fill:#99ff99,stroke:#333,stroke-width:2px
```

### Data Flow: Token Resolution Details

```mermaid
flowchart LR
    subgraph "Input"
        Resource[Resource CRD<br/>spec.tenant]
        Namespace[Namespace]
    end
    
    subgraph "TokenResolver - Single Point"
        Step1[1. Fetch Tenant CRD]
        Step2[2. Extract secret ref]
        Step3[3. Resolve namespace]
        Step4[4. Fetch Secret]
        Step5[5. Extract token]
    end
    
    subgraph "Output"
        Token[Token String]
        Client[NetBoxClient]
    end
    
    Resource --> Step1
    Namespace --> Step1
    Step1 --> Step2
    Step2 --> Step3
    Step3 --> Step4
    Step4 --> Step5
    Step5 --> Token
    Token --> Client
    
    style Step1 fill:#ffcccc
    style Step2 fill:#ffcccc
    style Step3 fill:#ffcccc
    style Step4 fill:#ffcccc
    style Step5 fill:#ffcccc
```

## Implementation Details

### TokenResolver Interface

```rust
pub struct TokenResolver {
    kube_client: Client,
    netbox_url: String,
    // Optional: Token cache per tenant to avoid repeated Secret fetches
    // token_cache: Arc<Mutex<HashMap<String, CachedToken>>>,
}

impl TokenResolver {
    /// SINGLE POINT OF TOKEN RESOLUTION
    /// All token resolution flows through this method
    pub async fn resolve_token(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<String, TokenResolutionError> {
        // Implementation details...
    }
    
    /// SINGLE POINT OF CLIENT CREATION
    /// All NetBoxClient creation with tenant tokens flows through this
    pub async fn create_client_for_tenant(
        &self,
        namespace: &str,
        tenant_ref: &NetBoxResourceReference,
    ) -> Result<NetBoxClient, TokenResolutionError> {
        let token = self.resolve_token(namespace, tenant_ref).await?;
        NetBoxClient::new(self.netbox_url.clone(), token)
            .map_err(TokenResolutionError::ClientCreation)
    }
}
```

### Reconciler Changes

```rust
impl Reconciler {
    pub async fn reconcile_site(&self, site: &NetBoxSite) -> Result<(), ControllerError> {
        // Extract tenant reference
        let tenant_ref = &site.spec.tenant;
        let namespace = site.metadata.namespace.as_deref().unwrap_or("default");
        
        // SINGLE POINT: Get tenant-specific client
        let netbox_client = self.token_resolver
            .create_client_for_tenant(namespace, tenant_ref)
            .await?;
        
        // Use tenant-specific client for all operations
        // ... reconciliation logic ...
    }
}
```

## Benefits

1. **Single Point of Injection**: All token resolution in `TokenResolver`
2. **Security**: Tokens stored in Kubernetes Secrets, not in CRDs
3. **Multi-Tenancy**: Each tenant isolated with own token
4. **Flexibility**: Secret namespace can differ from resource namespace
5. **Testability**: TokenResolver can be mocked for unit tests
6. **Caching Opportunity**: Can cache tokens per tenant to reduce Secret API calls

## Migration Path

1. Add `token_secret` field to `NetBoxTenantSpec`
2. Create `TokenResolver` service
3. Update `Controller` to create `TokenResolver` instead of single `NetBoxClient`
4. Update all reconcilers to use `TokenResolver.create_client_for_tenant()`
5. Remove `NETBOX_TOKEN` environment variable requirement
6. Update deployment to remove token from environment

