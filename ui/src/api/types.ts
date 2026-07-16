// ============================================================================
// K8s Resource Types
// ============================================================================

// Generic K8s Resource — matches K8s API response shape for any CRD
export interface K8sResource {
  apiVersion: string;
  kind: string;
  metadata: {
    name: string;
    namespace: string;
    creationTimestamp: string;
    labels?: Record<string, string>;
    annotations?: Record<string, string>;
    uid?: string;
    resourceVersion?: string;
  };
  spec: Record<string, unknown>;
  status?: Record<string, unknown>;
}

// CRD metadata from K8s API discovery
export interface CrdMeta {
  plural: string;
  kind: string;
  category: CrdCategory;
  subgroup: string; // e.g. 'netbox.dcops.microscaler.io', 'ippool.dcops.microscaler.io', 'boot.dcops.microscaler.io'
}

// Categories for CRD grouping (mirrors the 6 logical groups in 01_dashboard_design.md)
export type CrdCategory =
  | 'ipam'
  | 'dcim'
  | 'tenancy'
  | 'extras'
  | 'boot'
  | 'ippool';

// Resource state from CRD status subresource
export type ResourceState = 'Pending' | 'Created' | 'Updated' | 'Failed';

// Dashboard data — aggregated result of fetching all CRs
export interface DashboardData {
  crds: CrdMeta[];
  instances: Map<string, K8sResource[]>; // key = plural
  namespaces: string[];
  summary: {
    total: number;
    byCategory: Record<string, number>;
    byState: Record<string, number>;
    withErrors: number;
  };
}

// Error response from K8s API
export interface K8sError {
  kind: string;
  status: string;
  code: number;
  message: string;
  reason?: string;
}

// Fetch options for K8s API calls
export interface FetchOptions {
  timeout?: number;
  signal?: AbortSignal;
}
