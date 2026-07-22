// ============================================================================
// K8s API Client
// ============================================================================
// Reads from `kubernetes.default.svc:443` when called from inside the cluster,
// or from a local `kubectl proxy` (default `http://localhost:8001`) when
// running in Tilt/dev mode.  The client is **read-only** — it only performs
// `get` and `list` operations.

import type {
  DashboardData,
  FetchOptions,
  K8sError,
  K8sResource,
} from './types';
import {
  CrdCategory,
  type CrdMeta,
} from './types';

// ---------------------------------------------------------------------------
// Category catalogue — mirrors the 6 logical groups in the design doc.
// ---------------------------------------------------------------------------
const CRD_CATEGORY_MAP: Record<string, CrdCategory> = {
  // IPAM (9)
  netboxprefixes: 'ipam',
  netboxipaddresses: 'ipam',
  netboxipranges: 'ipam',
  netboxaggregates: 'ipam',
  netboxvlans: 'ipam',
  netboxrirs: 'ipam',
  netboxvrfs: 'ipam',
  netboxroutetargets: 'ipam',
  netboxroles: 'ipam',
  // DCIM (11)
  netboxdevices: 'dcim',
  netboxdeviceroles: 'dcim',
  netboxdevicetypes: 'dcim',
  netboxinterfaces: 'dcim',
  netboxmacaddresses: 'dcim',
  netboxlocations: 'dcim',
  netboxsites: 'dcim',
  netboxsitegroups: 'dcim',
  netboxregions: 'dcim',
  netboxmanufacturers: 'dcim',
  netboxplatforms: 'dcim',
  // Tenancy (2)
  netboxtenants: 'tenancy',
  netboxtenantgroups: 'tenancy',
  // Extras (1)
  netboxtags: 'extras',
  // Boot (2)
  bootprofiles: 'boot',
  bootintents: 'boot',
  // IP Pool (2)
  ippools: 'ippool',
  ipclaims: 'ippool',
};

// Infer the human-readable kind from the plural name (e.g. "netboxprefixes" → "NetBoxPrefix").
function pluralToKind(plural: string): string {
  const singular = plural.endsWith('s') ? plural.slice(0, -1) : plural;
  // Split on camel-case boundaries: "netboxprefix" → "Netboxprefix", then Capitalize
  const words = singular.replace(/([A-Z])/g, ' $1').trim().split(/\s+/);
  return words.map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join('');
}

// ---------------------------------------------------------------------------
// K8sClient
// ---------------------------------------------------------------------------

// Auto-detect runtime: in Tilt dev mode the KUBE_PROXY env var is set by the
// Tiltfile's environment() call; otherwise we try the in-cluster cert.
function detectApiBaseUrl(): string {
  // In-cluster: use the Kubernetes discovery URL with in-cluster auth.
  if (typeof window !== 'undefined') {
    // Running in browser: if we're in-cluster, the in-cluster cert is
    // mounted at /var/run/secrets/kubernetes.io/serviceaccount.
    // We can't check that here, so we default to the in-cluster URL and
    // let the browser handle auth via the mounted SA token.
    return 'https://kubernetes.default.svc:443';
  }
  // Server-side default — shouldn't reach here.
  return '';
}

export class K8sClient {
  private apiUrlBase: string;
  private isDev: boolean;

  constructor(proxyUrl?: string) {
    this.isDev = !!proxyUrl;
    this.apiUrlBase = proxyUrl ?? detectApiBaseUrl();
  }

  // Build the base URL for the API group.
  private apiUrl(path: string): string {
    if (this.isDev) {
      return `${this.apiUrlBase}${path}`;
    }
    // In-cluster: use the Kubernetes in-cluster discovery URL.
    return `https://kubernetes.default.svc:443${path}`;
  }

  // Fetch JSON from the API.  Returns a parsed object or throws K8sError.
  private async fetchJson<T>(url: string, options?: FetchOptions): Promise<T> {
    const res = await fetch(url, {
      headers: { Accept: 'application/json' },
      signal: options?.signal,
    });

    if (!res.ok) {
      let message = `HTTP ${res.status}`;
      try {
        const body = (await res.json()) as K8sError;
        message = body.message || message;
      } catch {
        // Not JSON — leave the default message.
      }
      throw {
        kind: 'Status',
        status: 'Failure',
        code: res.status,
        message,
        reason: res.statusText,
      } as K8sError;
    }

    return res.json();
  }

  // ---------------------------------------------------------------------------
  // Discover ALL dcops.* microgroups under dcops.microscaler.io.
  // ---------------------------------------------------------------------------
  async listCrdPlurals(): Promise<CrdMeta[]> {
    // GET /apis → the key is the API group name, the value contains versions.
    // dcops subgroups appear as keys like 'netbox.dcops.microscaler.io',
    // not 'dcops.microscaler.io'.
    const apiGroups = await this.fetchJson<Record<string, unknown>>(
      this.apiUrl('/apis'),
    );

    // Find all dcops.* subgroups and extract versions to determine the API version.
    const subgroupNames = Object.keys(apiGroups)
      .filter((key) => key.endsWith('.dcops.microscaler.io'));

    if (subgroupNames.length === 0) {
      throw {
        kind: 'Status',
        status: 'Failure',
        code: 404,
        message:
          'No dcops API groups found — are CRDs applied?',
      } as K8sError;
    }

    // Get the version from the first subgroup entry.
    const firstGroup = apiGroups[subgroupNames[0]] as
      | {
          versions?: { name: string }[];
        }
      | undefined;

    const version =
      firstGroup?.versions?.[0]?.name ?? 'v1alpha1';

    const allResources: CrdMeta[] = [];

    // For each subgroup, fetch its resources.
    for (const subgroup of subgroupNames) {
      const resourceList = await this.fetchJson<{
        kind: string;
        resources: { plural: string; name: string }[];
      }>(
        this.apiUrl(`/apis/${subgroup}/${version}`),
      );

      for (const r of resourceList.resources) {
        allResources.push({
          plural: r.plural,
          kind: pluralToKind(r.plural),
          category:
            CRD_CATEGORY_MAP[r.plural] ??
            ('dcim' as CrdCategory), // fallback — unlikely to happen
          subgroup,
        });
      }
    }

    return allResources;
  }

  // ---------------------------------------------------------------------------
  // List CRs of one type in one namespace (dynamic subgroup support).
  // ---------------------------------------------------------------------------
  async listCrds(
    plural: string,
    namespace: string,
    subgroup: string,
  ): Promise<K8sResource[]> {
    const data = await this.fetchJson<{ items: K8sResource[] }>(
      this.apiUrl(
        `/apis/${subgroup}/v1alpha1/namespaces/${encodeURIComponent(namespace)}/${encodeURIComponent(plural)}`,
      ),
    );
    return data.items;
  }

  // ---------------------------------------------------------------------------
  // Get a single CR by name (dynamic subgroup support).
  // ---------------------------------------------------------------------------
  async getCrds(
    plural: string,
    namespace: string,
    name: string,
    subgroup: string,
  ): Promise<K8sResource> {
    const data = await this.fetchJson<K8sResource>(
      this.apiUrl(
        `/apis/${subgroup}/v1alpha1/namespaces/${encodeURIComponent(namespace)}/${encodeURIComponent(plural)}/${encodeURIComponent(name)}`,
      ),
    );
    return data;
  }

  // ---------------------------------------------------------------------------
  // List all namespaces.
  // ---------------------------------------------------------------------------
  async listNamespaces(): Promise<string[]> {
    const data = await this.fetchJson<{ items: { metadata: { name: string } }[] }>(
      this.apiUrl('/api/v1/namespaces'),
    );
    return data.items.map((n) => n.metadata.name);
  }

  // ---------------------------------------------------------------------------
  // Fetch all CRs for the dashboard (one call per namespace).
  // ---------------------------------------------------------------------------
  async fetchAllDashboardData(
    namespaces: string[],
    crds: CrdMeta[],
  ): Promise<DashboardData> {
    const instances = new Map<string, K8sResource[]>();
    const errors: K8sError[] = [];

    // For each namespace, fetch every CRD type.
    for (const ns of namespaces) {
      for (const crd of crds) {
        try {
          const items = await this.listCrds(crd.plural, ns, crd.subgroup);
          const existing = instances.get(crd.plural) ?? [];
          instances.set(crd.plural, [...existing, ...items]);
        } catch (err) {
          // Per-call errors (e.g. no CRs for this type in this namespace)
          // are expected — just skip.
          if (
            typeof err === 'object' &&
            err !== null &&
            'code' in err &&
            (err as K8sError).code !== 403
          ) {
            errors.push(err as K8sError);
          }
        }
      }
    }

    // Build summary.
    const allCrInstances = Array.from(instances.values()).flat();
    const byCategory = {} as Record<string, number>;
    const byState = {} as Record<string, number>;
    let withErrors = 0;

    for (const inst of allCrInstances) {
      const crd = crds.find((c) => c.plural === inst.kind.toLowerCase() + 's');
      if (crd) {
        byCategory[crd.category] = (byCategory[crd.category] ?? 0) + 1;
      }
      const state = (inst.status as Record<string, unknown>)?.state as
        | string
        | undefined;
      if (state) {
        byState[state] = (byState[state] ?? 0) + 1;
      }
      if ((inst.status as Record<string, unknown>)?.error) {
        withErrors++;
      }
    }

    return {
      crds,
      instances,
      namespaces,
      summary: {
        total: allCrInstances.length,
        byCategory,
        byState,
        withErrors,
      },
    };
  }
}
