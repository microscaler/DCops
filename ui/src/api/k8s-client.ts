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
export class K8sClient {
  private proxyUrl: string;

  constructor(proxyUrl = 'http://localhost:8001') {
    this.proxyUrl = proxyUrl;
  }

  // Build the base URL for the API group.
  private apiUrl(path: string): string {
    if (this.proxyUrl) {
      return `${this.proxyUrl}${path}`;
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
  // Discover all dcops CRD plurals + categories.
  // ---------------------------------------------------------------------------
  async listCrdPlurals(): Promise<CrdMeta[]> {
    // GET /apis → list API groups, then look for dcops.microscaler.io
    const apiGroups = await this.fetchJson<Record<string, unknown>>(
      this.apiUrl('/apis'),
    );

    const dcopsGroup = apiGroups['dcops.microscaler.io'] as
      | {
          versions?: { name: string }[];
        }
      | undefined;

    if (!dcopsGroup) {
      throw {
        kind: 'Status',
        status: 'Failure',
        code: 404,
        message:
          'API group dcops.microscaler.io not found — are CRDs applied?',
      } as K8sError;
    }

    const version =
      dcopsGroup.versions?.[0]?.name ?? 'v1alpha1';

    // GET /apis/dcops.microscaler.io/v1alpha1 → get available resources (plural + kind)
    const resourceList = await this.fetchJson<{
      kind: string;
      resources: { plural: string; name: string }[];
    }>(
      this.apiUrl(`/apis/dcops.microscaler.io/${version}`),
    );

    return resourceList.resources.map((r) => ({
      plural: r.plural,
      kind: pluralToKind(r.plural),
      category:
        CRD_CATEGORY_MAP[r.plural] ??
        ('dcim' as CrdCategory), // fallback — unlikely to happen
    }));
  }

  // ---------------------------------------------------------------------------
  // List CRs of one type in one namespace.
  // ---------------------------------------------------------------------------
  async listCrds(
    plural: string,
    namespace: string,
  ): Promise<K8sResource[]> {
    const data = await this.fetchJson<{ items: K8sResource[] }>(
      this.apiUrl(
        `/apis/dcops.microscaler.io/v1alpha1/namespaces/${encodeURIComponent(namespace)}/${encodeURIComponent(plural)}`,
      ),
    );
    return data.items;
  }

  // ---------------------------------------------------------------------------
  // Get a single CR by name.
  // ---------------------------------------------------------------------------
  async getCrds(
    plural: string,
    namespace: string,
    name: string,
  ): Promise<K8sResource> {
    const data = await this.fetchJson<K8sResource>(
      this.apiUrl(
        `/apis/dcops.microscaler.io/v1alpha1/namespaces/${encodeURIComponent(namespace)}/${encodeURIComponent(plural)}/${encodeURIComponent(name)}`,
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
          const items = await this.listCrds(crd.plural, ns);
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
