// ============================================================================
// CRD Discovery
// ============================================================================
// Auto-discovers all `dcops.microscaler.io` CRD types from the K8s API and
// assigns them to logical categories (IPAM, DCIM, Tenancy, etc.).

import { K8sClient } from './k8s-client';
import type { CrdMeta, DashboardData } from './types';

/**
 * Discover and return all CRD metadata from the cluster.
 * Throws if the API group is not available.
 */
export async function discoverCrdMeta(
  client: K8sClient,
): Promise<CrdMeta[]> {
  return client.listCrdPlurals();
}

/**
 * Get the full dashboard data in one call.
 * Fetches namespaces and CRDs, then aggregates all CR instances.
 */
export async function fetchDashboardData(
  client: K8sClient,
): Promise<DashboardData> {
  const crds = await discoverCrdMeta(client);
  const namespaces = await client.listNamespaces();
  return client.fetchAllDashboardData(namespaces, crds);
}
