# EPIC-02: Dashboard Page & State Management

> **Parent:** [Dashboard Design](../01_dashboard_design.md)
> **Status:** Draft — awaiting implementation
> **Priority:** P0
> **Estimated effort:** 3–4 days
> **Dependencies:** [EPIC-001](./EPIC-001-api-client.md)

---

## Overview

Build the Dashboard page that serves as the landing view for the Admin section. It presents summary statistics (total CRs, by state, by category), a namespace filter, and a refresh mechanism. It acts as the parent component for the CR table and error summary.

## User Stories

| ID | Story | Persona |
|----|-------|---------|
| **US-04** | As a platform operator, I see summary cards showing total CR count, failures, and namespaces on the dashboard so I can assess platform health at a glance | Operator |
| **US-05** | As a developer, I can filter CRs by namespace so I can isolate resources to one namespace | Developer |
| **US-06** | As a platform operator, I can manually refresh data when I suspect stale state | Operator |
| **US-07** | As a platform operator, I see a visual chart of CR distribution by type so I can quickly see which categories dominate | Operator |

## Functional Requirements

### FR-4: Dashboard Page

| ID | Requirement |
|----|-------------|
| **FR-4-01** | Dashboard page must display 4 summary cards: Total CRs, Failed CRs, Namespaces, Last Refreshed |
| **FR-4-02** | Dashboard must display a CR distribution chart (bar or pie) grouped by category |
| **FR-4-03** | Dashboard must include a namespace filter dropdown |
| **FR-4-04** | Dashboard must include a manual "Refresh" button |
| **FR-4-05** | Dashboard must add an "Admin" tab to the header navigation alongside "User Docs" and "Contributor Docs" |

### FR-5: State Management

| ID | Requirement |
|----|-------------|
| **FR-5-01** | Use Solid.js `createSignal` for state (no Redux, no Zustand) |
| **FR-5-02** | Dashboard state must include: `crds`, `instances`, `summary`, `loading`, `error`, `namespace` |
| **FR-5-03** | Auto-refresh every 30 seconds (configurable) |

## Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-05** | UX | Dashboard must load and display data within 3s of page mount (proxy mode) |
| **NFR-06** | UX | Summary cards must update immediately on namespace filter change |
| **NFR-07** | Performance | Chart rendering must not block the main thread; use requestIdleCallback for large datasets |
| **NFR-08** | Accessibility | Summary cards and chart must be screen-reader accessible (ARIA labels) |

## Acceptance Criteria

| ID | Criterion | Status |
|----|-----------|--------|
| **AC-02-01** | [ ] Dashboard page is accessible via new "Admin" tab in header | Pending |
| **AC-02-02** | [ ] Summary cards display correct counts: total, failed, namespaces, last refresh time | Pending |
| **AC-02-03** | [ ] CR distribution chart renders with correct category counts | Pending |
| **AC-02-04** | [ ] Namespace filter updates all dashboard components (table, summary, chart) | Pending |
| **AC-02-05** | [ ] Manual refresh button re-fetches all data and updates UI | Pending |
| **AC-02-06** | [ ] Auto-refresh fires every 30 seconds and updates UI without errors | Pending |
| **AC-02-07** | [ ] Loading spinner appears during data fetch | Pending |
| **AC-02-08** | [ ] Error banner displays if API client fails with retry option | Pending |

## Definition of Done

- [ ] `pages/Dashboard.tsx` implemented with summary cards, chart, namespace filter, refresh button
- [ ] Dashboard integrates with EPIC-001 API client
- [ ] State management uses Solid.js signals
- [ ] Chart component renders category distribution
- [ ] Loading and error states handled
- [ ] Tests: component renders with mock data
- [ ] Code reviewed and merged

## Dependencies

| ID | Depends on |
|----|------------|
| FR-4 | EPIC-001 (API client must be ready) |
| FR-5 | None (Solid.js built-in) |

## Open Questions

| ID | Question | Default if unresolved |
|----|----------|----------------------|
| **Q1** | What chart library to use? | No external chart library — build lightweight bar chart with SVG/CSS to avoid dependency |
