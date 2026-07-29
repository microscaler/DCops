# EPIC-01: K8s API Client & CRD Discovery

> **Parent:** [Dashboard Design](../01_dashboard_design.md)
> **Status:** Draft — awaiting implementation
> **Priority:** P0
> **Estimated effort:** 3–4 days
> **Dependencies:** None

---

## Overview

Build the foundational K8s API client that the entire dashboard depends on. It must auto-discover all `dcops.microscaler.io` CRD types from the cluster API, list CR instances by type and namespace, and fetch individual CR details. Supports both `kubectl proxy` mode (Tilt dev) and in-cluster mode (production SA token).

## User Stories

| ID | Story | Persona |
|----|-------|---------|
| **US-01** | As a platform operator, I see an auto-discovered list of all CRD types so I don't need to maintain a hardcoded registry | Operator |
| **US-02** | As a developer, I can run `tilt up` and see live CR data without configuring any auth tokens | Developer |
| **US-03** | As a platform operator, I can see CRs across all namespaces in one view | Operator |

## Functional Requirements

### FR-1: K8s API Client

| ID | Requirement |
|----|-------------|
| **FR-1-01** | Client must support two modes: proxy (`localhost:8001`) and in-cluster (SA token) |
| **FR-1-02** | Must discover all CRDs in `dcops.microscaler.io` group via `GET /apis` endpoint |
| **FR-1-03** | Must list CR instances of a given type in a given namespace via `GET /apis/dcops.microscaler.io/v1alpha1/namespaces/{ns}/{plural}` |
| **FR-1-04** | Must fetch a single CR by name via `GET /apis/dcops.microscaler.io/v1alpha1/namespaces/{ns}/{plural}/{name}` |
| **FR-1-05** | Must list available namespaces via `GET /api/v1/namespaces` |

### FR-2: CRD Discovery

| ID | Requirement |
|----|-------------|
| **FR-2-01** | Discovery must filter API groups by `dcops.microscaler.io` |
| **FR-2-02** | Must return a list of `{plural, kind, category}` objects |
| **FR-2-03** | Categories must be assigned: `ipam`, `dcim`, `tenancy`, `extras`, `boot`, `ippool` |

### FR-3: Types

| ID | Requirement |
|----|-------------|
| **FR-3-01** | Define `K8sResource` TypeScript interface matching K8s API response shape |
| **FR-3-02** | Define `CrdMeta` TypeScript interface with plural, kind, category |

## Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-01** | Security | In-cluster mode must use only `get` and `list` verbs (no create/update/delete) |
| **NFR-02** | Reliability | Client must handle HTTP errors (403, 500, timeout) gracefully with clear error messages |
| **NFR-03** | Performance | CRD discovery must complete in <2s; list of CRs per type must complete in <3s |
| **NFR-04** | Compatibility | Must work with `kubectl proxy` on localhost:8001 and in-cluster SA token auth |

## Acceptance Criteria

| ID | Criterion | Status |
|----|-----------|--------|
| **AC-01-01** | [ ] `listCrdPlurals()` returns all 27 CRDs grouped by category when connected to a cluster with all CRDs deployed | Pending |
| **AC-01-02** | [ ] `listCrds('netboxprefixes', 'default')` returns all NetBoxPrefix CRs in the default namespace | Pending |
| **AC-01-03** | [ ] `getCrds('netboxprefixes', 'default', 'control-plane-prefix')` returns a single CR matching the name | Pending |
| **AC-01-04** | [ ] Proxy mode works: `new K8sClient('http://localhost:8001')` can fetch CRD list and CR instances | Pending |
| **AC-01-05** | [ ] Client returns a structured error (not crash) when the API is unreachable | Pending |
| **AC-01-06** | [ ] All TypeScript types compile with no errors under strict mode | Pending |

## Definition of Done

- [ ] `api/k8s-client.ts` implemented with all methods
- [ ] `api/crd-discovery.ts` implemented with category assignment
- [ ] `api/types.ts` implements all shared types
- [ ] Tests: mock client can verify list/get operations
- [ ] TypeScript compiles with `--strict`
- [ ] Works in both proxy and in-cluster modes
- [ ] Code reviewed and merged

## Dependencies

| ID | Depends on |
|----|------------|
| FR-1 | None (foundation layer) |
| FR-2 | FR-1 (uses client endpoints) |
| FR-3 | None (type definitions, used by other epics) |

## Open Questions

| ID | Question | Default if unresolved |
|----|----------|----------------------|
| **Q1** | Should discovery cache results for the session or re-fetch on every navigation? | Cache for session lifetime; manual refresh re-fetches |
