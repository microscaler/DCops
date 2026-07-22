# Aether ↔ DCops IPAM / DHCP Contract

How Project Aether (the KVM/Firecracker hypervisor orchestration plane) and DCops
(NetBox IPAM + the Kea DHCP controller) divide responsibility for a VM's network
identity, and the concrete CRD/DHCP mechanics that make it work.

> **Division of ownership:** **Aether owns the MAC address** (and the disk).
> **DCops owns the IP** (via NetBox IPAM + Kea DHCP). Aether never allocates IPs;
> it asserts a stable MAC and a target DHCP pool, and DCops turns that into a
> lease.

This is the DCops-side companion to Aether's
`docs/architecture/impl_network_identity.md`.

---

## Why this split

When an Aether VM is recovered onto a new blade after a node failure, it must come
back with the **same** IP, DNS name, and L2 identity — otherwise it isn't really
the same workload. Aether guarantees this by preserving the VM's **MAC** across
recovery (the MAC is derived deterministically from the workload uuid, so recovery
replays it with no state). If the IP is a DHCP lease keyed on that MAC, the
recovered VM gets its address back for free. DCops is the system of record for
addresses, so it owns that half.

---

## The claim: what Aether declares

For each VM, Aether declares a DCops **`NetBoxIPAddress`** custom resource keyed on
the MAC:

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxIPAddress
metadata:
  name: aether-<workload-uuid>      # deterministic, DNS-1123 safe
spec:
  status: dhcp                       # DHCP-managed, not a static NetBox IP
  macAddress: "52:54:00:ab:cd:ef"    # Aether-owned, stable across recovery
  ipRange:                           # the DHCP pool to lease from
    apiGroup: dcops.microscaler.io
    kind: NetBoxIPRange
    name: dhcp-pool-range
  tenant: { ... }
  # NOTE: no `address:` — DCops/Kea assign the IP
```

The critical properties:

- **`status: dhcp`** — this is a DHCP reservation, not a statically-assigned IP.
- **`macAddress`** set, **`address` absent** — Aether asserts the MAC and the pool;
  DCops picks the actual address.
- **`ipRange`** points at a pool that is **populated** (see below).
- The resource name is deterministic, so re-declaring the same workload (e.g. on
  recovery) converges on the same object — idempotent.

Aether publishes this claim GitOps-style (a manifest reconciled by Flux/Argo) or,
in future, via a live apply. Either way it lands as a CR in the DCops cluster.

---

## Populated IP ranges: the NetBox constraint (and the fix)

The DHCP pool is a `NetBoxIPRange` with **`markPopulated: true`**:

```yaml
kind: NetBoxIPRange
metadata: { name: dhcp-pool-range }
spec:
  startAddress: 192.168.1.100/24
  endAddress:   192.168.1.200/24
  markPopulated: true      # externally managed (by Kea) — see below
```

`markPopulated` tells NetBox the range is managed by an external system (a DHCP
server). NetBox then **prohibits creating individual `IPAddress` objects inside the
range** — by design (see [`NETBOX_IP_RANGE_ANALYSIS.md`](./NETBOX_IP_RANGE_ANALYSIS.md)).

The `NetBoxIPAddress` reconciler honours this
(`controllers/netbox/src/reconciler/ipam/ip_address.rs`):

1. When it resolves the `ipRange`, it reads the range's `mark_populated` flag.
2. If the range is populated, it **does not** create an `IPAddress` in NetBox.
   Instead it records the address in the CR status as terminally `Created` **with
   no NetBox id** (`create_populated_range_ip_status_patch`), emits an
   `ExternallyManaged` event, and returns — short-circuiting before the drift
   check so there is no create→fail→recreate loop.

The address in the pool belongs to Kea; NetBox merely documents the pool. This is
the repo's own recommended "Option 1," now implemented and unit-tested. See the
[resolution section of the analysis doc](./NETBOX_IP_RANGE_ANALYSIS.md#resolution-implemented).

---

## The DHCP half: Kea reservation keyed by MAC

The DHCP controller (`controllers/dhcp/`) watches `NetBoxIPAddress` (and
`NetBoxIPRange`, `NetBoxPrefix`) CRDs and translates them into Kea configuration:

- The **range** becomes a Kea subnet/pool.
- A `NetBoxIPAddress` with `status: dhcp` and a `macAddress` becomes a Kea **host
  reservation** keyed on that MAC (via the Kea Control Agent `reservation-add` /
  subnet reservation config — see [`controllers/dhcp/KEA_COMMANDS.md`](../controllers/dhcp/KEA_COMMANDS.md)).

So when the VM boots and its NIC (the MAC Aether assigned) sends a DHCP DISCOVER,
Kea matches the reservation and hands back a **stable** lease — the same address
every time, because the reservation is keyed on the MAC, not on lease timing.

---

## End-to-end

```text
Aether: reserve_mac(uuid) ─► 52:54:00:…            (deterministic)
        └─► publish NetBoxIPAddress { mac, status: dhcp, ipRange: <populated>, no address }
                                   │
DCops NetBox controller ──────────┤ range is populated → track in CR status
                                   │                      (no NetBox IPAddress object)
DCops DHCP controller ────────────┘ → Kea host reservation keyed by MAC
                                   │
VM boots with that MAC ───────────► DHCP DISCOVER ─► Kea matches reservation ─► stable IP
                                   │
Aether node failure ──► recover VM ─► replay SAME MAC ─► SAME Kea reservation ─► SAME IP
```

Identity (MAC, Aether), address (IP, DCops/Kea), and disk (replicated ZVOL,
Aether storage node) all follow the VM to its new home — the three legs of a
genuine recovery.

---

## Boundaries & responsibilities

| Concern | Owner | Mechanism |
| :--- | :--- | :--- |
| Stable MAC | **Aether** | Deterministic derivation from workload uuid; replayed on recovery |
| Declaring the reservation | **Aether** | `NetBoxIPAddress` CR (MAC + populated `ipRange`, no address) |
| Accepting the claim without allocating | **DCops NetBox controller** | Populated-range check → track in CR status, no NetBox IP object |
| Assigning & serving the IP | **DCops / Kea** | Host reservation keyed by MAC → DHCP lease |
| Reading the leased IP back | *Not yet wired* | Aether EPIC-09.3 (DHCP snooping / guest agent) |

The one open loop is **read-back**: DCops/Kea assign the IP, but Aether does not
yet discover the guest's actual leased address to report it. Everything up to the
lease is wired.

## References

- Aether side: `Aether/docs/architecture/impl_network_identity.md`
- NetBox populated-range analysis + fix: [`NETBOX_IP_RANGE_ANALYSIS.md`](./NETBOX_IP_RANGE_ANALYSIS.md)
- DHCP controller: [`../controllers/dhcp/README.md`](../controllers/dhcp/README.md),
  [`../controllers/dhcp/KEA_COMMANDS.md`](../controllers/dhcp/KEA_COMMANDS.md)
- Reconciler code: `controllers/netbox/src/reconciler/ipam/ip_address.rs`
