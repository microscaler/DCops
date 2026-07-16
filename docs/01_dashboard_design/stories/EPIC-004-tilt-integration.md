# EPIC-04: Tilt Integration & Production Deployment

> **Parent:** [Dashboard Design](../01_dashboard_design.md)
> **Status:** Draft — awaiting implementation
> **Priority:** P1
> **Estimated effort:** 2–3 days
> **Dependencies:** [EPIC-001](./EPIC-001-api-client.md), [EPIC-002](./EPIC-002-dashboard-page.md), [EPIC-003](./EPIC-003-crd-table.md)

---

## Overview

Wire up the Tiltfile to start `kubectl proxy` as a local resource dependency, and add the dashboard route to the UI navigation. Add RBAC manifests for in-cluster production deployment and update the deployment manifest to reference the dashboard page.

## User Stories

| ID | Story | Persona |
|----|-------|---------|
| **US-12** | As a developer, I can run `tilt up` and immediately see the dashboard with live CR data | Developer |
| **US-13** | As a platform operator, I can deploy the dashboard in-cluster and it will authenticate via ServiceAccount | Operator |

## Functional Requirements

### FR-9: Tiltfile Integration

| ID | Requirement |
|----|-------------|
| **FR-9-01** | Add `kubectl proxy` as a `local_resource` in Tiltfile on port 8001 |
| **FR-9-02** | `kubectl-proxy` must start before `dcops-ui` (resource dependency) |
| **FR-9-03** | `dcops-ui` Tiltfile resource must depend on `kubectl-proxy` |

### FR-10: In-Cluster Deployment

| ID | Requirement |
|----|-------------|
| **FR-10-01** | Add ClusterRole and ClusterRoleBinding for `dcops-ui-reader` with `get` and `list` on `dcops.microscaler.io/*` |
| **FR-10-02** | Create `config/dcops-ui-rbac/` directory with RBAC manifests |
| **FR-10-03** | Update `config/dcops-ui/kustomization.yaml` to include the RBAC resources |

### FR-11: UI Navigation

| ID | Requirement |
|----|-------------|
| **FR-11-01** | Add "Admin" tab to header navigation in `App.tsx` |
| **FR-11-02** | Admin tab routes to `#/admin/dashboard` |
| **FR-11-03** | Admin section must be separate from existing doc sections (User Docs, Contributor Docs) |

## Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-13** | Security | Production RBAC must only grant `get` and `list` (no write access) |
| **NFR-14** | Reliability | Tilt must start all dependencies before serving the UI |
| **NFR-15** | Deployability | Dashboard must work both with proxy (Tilt) and in-cluster (SA token) |

## Acceptance Criteria

| ID | Criterion | Status |
|----|-----------|--------|
| **AC-04-01** | [ ] `tilt up` starts `kubectl proxy` before `dcops-ui` | Pending |
| **AC-04-02** | [ ] Dashboard loads at `http://localhost:8800` with live CR data | Pending |
| **AC-04-03** | [ ] "Admin" tab appears in header navigation | Pending |
| **AC-04-04** | [ ] Clicking Admin tab shows the dashboard | Pending |
| **AC-04-05** | [ ] RBAC manifests exist in `config/dcops-ui-rbac/` with ClusterRole and ClusterRoleBinding | Pending |
| **AC-04-06** | [ ] RBAC only grants `get` and `list` on `dcops.microscaler.io/*` | Pending |
| **AC-04-07** | [ ] `kustomization.yaml` includes RBAC resources | Pending |
| **AC-04-08** | [ ] In-cluster mode: UI uses SA token, not proxy URL | Pending |

## Definition of Done

- [ ] Tiltfile has `kubectl-proxy` local_resource with correct deps
- [ ] RBAC manifests created and tested
- [ ] Admin tab integrated into header navigation
- [ ] Dashboard accessible at `#/admin/dashboard`
- [ ] Tilt deployment tested end-to-end
- [ ] In-cluster deployment tested (if cluster accessible)
- [ ] Code reviewed and merged

## Dependencies

| ID | Depends on |
|----|------------|
| FR-9 | None (Tiltfile changes) |
| FR-10 | None (standalone manifests) |
| FR-11 | EPIC-001, EPIC-002, EPIC-003 (all UI components must exist) |

## Open Questions

| ID | Question | Default if unresolved |
|----|----------|----------------------|
| **Q1** | Should the dashboard have its own namespace or share `dcops-system`? | Share `dcops-system` — minimal footprint, consistent with other DCops services |
