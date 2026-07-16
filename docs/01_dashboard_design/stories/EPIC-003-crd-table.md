# EPIC-03: CRD Table & Detail View

> **Parent:** [Dashboard Design](../01_dashboard_design.md)
> **Status:** Draft — awaiting implementation
> **Priority:** P1
> **Estimated effort:** 4–5 days
> **Dependencies:** [EPIC-001](./EPIC-001-api-client.md), [EPIC-002](./EPIC-002-dashboard-page.md)

---

## Overview

Build the generic CR table component that lists instances of any CRD type with sortable columns, error highlighting, and clickable rows that open the detail view. The detail view displays full spec and status JSON with syntax highlighting.

## User Stories

| ID | Story | Persona |
|----|-------|---------|
| **US-08** | As a platform operator, I can browse all CR instances for any type in a tabular view sorted by name, namespace, state, and age | Operator |
| **US-09** | As a platform operator, I see Failed CRs highlighted in red so I can quickly identify problems | Operator |
| **US-10** | As a developer, I can click a row to see the full YAML/JSON of a CR's spec and status | Developer |
| **US-11** | As a platform operator, I see a link to the NetBox resource (if netboxId exists) so I can cross-reference | Operator |

## Functional Requirements

### FR-6: Generic CRD Table

| ID | Requirement |
|----|-------------|
| **FR-6-01** | Table must show columns: Name, Namespace, State, Age, Error (if any) |
| **FR-6-02** | Table must be sortable by any column (click header toggles asc/desc) |
| **FR-6-03** | Rows for Failed CRs must be highlighted in red |
| **FR-6-04** | Rows must be clickable and open the CrdDetail panel |
| **FR-6-05** | Table must paginate at 50 rows per page |
| **FR-6-06** | Table must support filtering by state (Created/Failed/Pending) via a dropdown |

### FR-7: CR Detail Panel

| ID | Requirement |
|----|-------------|
| **FR-7-01** | Detail panel must display: metadata (name, namespace, labels, annotations), spec (JSON), status (JSON) |
| **FR-7-02** | Spec and status must be displayed in a syntax-highlighted JSON viewer |
| **FR-7-03** | If the CR has a `netboxId`, display a link to the NetBox resource (URL from `status.netboxUrl`) |
| **FR-7-04** | Detail panel must have a "Back to table" button |

### FR-8: Supporting Components

| ID | Requirement |
|----|-------------|
| **FR-8-01** | `StateBadge` component renders state as a colored badge: Created=green, Failed=red, Pending=yellow |
| **FR-8-02** | `JsonViewer` component renders JSON with syntax highlighting, collapse/expand, and copy button |
| **FR-8-03** | `ErrorSummary` component shows an accordion list of CRs with errors |

## Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-09** | Performance | Table with 500+ rows must remain responsive (virtualization or pagination) |
| **NFR-10** | UX | Sort and filter operations must complete in <200ms |
| **NFR-11** | Accessibility | Table must be keyboard navigable and screen-reader accessible |
| **NFR-12** | Compatibility | Must render correctly for all 27 CRD types regardless of spec shape |

## Acceptance Criteria

| ID | Criterion | Status |
|----|-----------|--------|
| **AC-03-01** | [ ] Table displays all CRs for a given type with Name, Namespace, State, Age, Error columns | Pending |
| **AC-03-02** | [ ] Clicking a column header sorts ascending; clicking again sorts descending | Pending |
| **AC-03-03** | [ ] Failed CR rows have a red background highlight | Pending |
| **AC-03-04** | [ ] Clicking a row opens the CrdDetail panel for that CR | Pending |
| **AC-03-05** | [ ] Detail panel shows metadata, spec, status, and NetBox link (if present) | Pending |
| **AC-03-06** | [ ] StateBadge renders correct color for each state value | Pending |
| **AC-03-07** | [ ] JsonViewer displays JSON with syntax highlighting and copy button | Pending |
| **AC-03-08** | [ ] Table pagination shows 50 rows per page with page navigation | Pending |
| **AC-03-09** | [ ] ErrorSummary shows CRs with errors in an accordion | Pending |
| **AC-03-10** | [ ] Table works with empty data (no CRs deployed) — shows empty state message | Pending |

## Definition of Done

- [ ] `components/dashboard/CrdTable.tsx` implemented with sorting, filtering, pagination
- [ ] `components/dashboard/CrdDetail.tsx` implemented with spec/status display
- [ ] `StateBadge` component renders correctly for all states
- [ ] `JsonViewer` component renders with syntax highlighting and copy
- [ ] `ErrorSummary` accordion component implemented
- [ ] All components handle empty/loading/error states
- [ ] Tests: table renders, sorts, filters, paginates with mock data
- [ ] Code reviewed and merged

## Dependencies

| ID | Depends on |
|----|------------|
| FR-6 | EPIC-001 (API client), EPIC-002 (Dashboard state) |
| FR-7 | FR-6 (detail opens from table row) |
| FR-8 | FR-7 (supporting components for table and detail) |

## Open Questions

| ID | Question | Default if unresolved |
|----|----------|----------------------|
| **Q1** | Should the detail panel be a side panel or a full page? | Side panel (slides in from right) for quick inspection; standalone route `#/admin/crd-detail` for deep linking |
| **Q2** | How to handle CRs with very large status payloads (e.g. 10KB+ JSON)? | Collapse by default, expandable with "Expand all" button |
