# PXE server implementation plan

**Status:** M0–M2 implemented (HTTP server on shared `kind-kind`; DHCP/TFTP deferred)

Canonical design for the custom Rust **Option B** stack (`dhcproto` + `async-tftp` + `axum`). Older docs that recommend Pixiecore-only ([`background_coversation/04_IPAM.md`](background_coversation/04_IPAM.md)) are background; the codebase and this doc supersede them for implementation.

Related (not this doc):

- [`PXE_CLUSTER_IMPLEMENTATION.md`](PXE_CLUSTER_IMPLEMENTATION.md) — NetBox CRDs for Pi/Talos inventory
- [`cylon-regenesis/docs/plan/07-ipxe-dcops-spec.md`](../../cylon-regenesis/docs/plan/07-ipxe-dcops-spec.md) — HTTP URL namespace and acceptance tests

---

## Architecture (Phase 2 lab)

```
Git (BootProfile + BootIntent)
        ↓
pxe-server HTTP (axum) — reads BootIntent/BootProfile from API server
        ↑ static artifacts under PXE_ROOT
Kea DHCP (dcops-system) → options 66/67 → pxe-server Service
        ↓
Bare metal iPXE client
```

**Decisions (locked for v1):**

| Topic | Choice |
|-------|--------|
| Boot config source | pxe-server lists `BootIntent` / `BootProfile` via in-cluster kube client |
| DHCP | Kea only; **no** ProxyDHCP in pxe-server for lab |
| TFTP | Deferred; iPXE over HTTP |
| IPv6 / dual-stack | Deferred |
| Lab artifacts | ConfigMap (iPXE scripts) + `emptyDir` / hostPath for large files |

---

## Crate layout

| Path | Role |
|------|------|
| `crates/pxe-server/src/bin/pxe-server.rs` | Process entrypoint |
| `config.rs` | `PXE_ROOT`, `HTTP_LISTEN`, env parsing |
| `http.rs` | axum router: health, API, static files |
| `boot.rs` | MAC normalize, lifecycle, iPXE script render |
| `store.rs` | Kubernetes `BootStore` |
| `api.rs` | Pixiecore-compatible `BootConfig` JSON |
| `dhcp.rs`, `tftp.rs` | Deferred (stubs) |

---

## HTTP surface

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/healthz` | Liveness |
| GET | `/v1/boot/:mac` | Pixiecore-style JSON (`BootConfig`) |
| GET | `/ipxe/boot/:mac.ipxe` | Dynamic iPXE script |
| GET | `/*` | Static files under `PXE_ROOT` (see cylon-regenesis spec) |

Static layout under `PXE_ROOT`:

```
ipxe/cylon-resurrection.ipxe
ipxe/localboot.ipxe
cylon-regenesis/profiles/{profile}/vmlinuz
cylon-regenesis/profiles/{profile}/initrd.img
cylon-regenesis/profiles/{profile}/autoinstall/...
cylon-regenesis/regenesis-agent/{version}/regenesis-agent
```

---

## BootIntent lookup

1. Normalize client MAC (`aa:bb:cc:dd:ee:ff`).
2. List `BootIntent` cluster-wide; match `spec.macAddress`.
3. No match → HTTP 404.
4. `lifecycle == locked` → serve `ipxe/localboot.ipxe` content.
5. Resolve `BootProfile` from `profileRef` (namespace defaults to intent namespace).
6. Render iPXE script or return JSON kernel/initrd/cmdline.

---

## Kubernetes

| Resource | Namespace |
|----------|-----------|
| Deployment `pxe-server` | `dcops-system` |
| Service `pxe-server` :8080 | `dcops-system` |
| ServiceAccount + ClusterRole | list/get/watch `bootintents`, `bootprofiles` |
| ConfigMap `pxe-ipxe-scripts` | entry + localboot scripts |

Tilt: port forward host **8088** → pod **8080** (avoids shared-kind **8080**).

---

## Milestones

| Milestone | Deliverable | REG-DCOPS |
|-----------|-------------|-----------|
| M0 | Binary, Deployment, Tilt, `/healthz` | — |
| M1 | Static `ServeDir`, tests | REG-DCOPS-01 |
| M2 | BootIntent lookup, locked → localboot | REG-DCOPS-02, 03 |
| M3 | pxe-intent controller | REG-DCOPS-04 |
| M4 | Kea next-server options | REG-DCOPS-05 |
| M5 | Phase 2 E2E with cylon-regenesis | joint |

---

## Deferred

- ProxyDHCP (`dhcp.rs`), TFTP (`tftp.rs`), full dual-stack
- Pixiecore push API (controller writes config); v1 uses direct CRD read
- HTTPS / production VLAN isolation
