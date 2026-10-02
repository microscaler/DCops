# Certificate Management — Epic & Story Authoring Rules

These rules govern every file in `docs/02_certificate_management/stories/`. They exist so that twelve epics written by different hands read as one backlog.

## File and identifiers

- One file per epic: `EPIC-CM-0NN-<slug>.md`. Epics are numbered 001–012 per `README.md`.
- Story IDs: `CM-0NN.S` where `NN` is the epic number and `S` is 1-based (e.g. `CM-002.3`).
- Requirement IDs inside a story: `FR-0NN.S-k`, `NFR-0NN.S-k`, `AC-0NN.S-k`, `UT-0NN.S-k`, `IT-0NN.S-k`. `k` is 1-based and zero-padded to two digits.
- Cross-reference other stories by ID in backticks: `CM-001.2`.
- Parent design document: `../../CERTIFICATE_MANAGEMENT_DESIGN.md` — cite its section numbers (§5.2 etc.) rather than restating it.

## Epic header (verbatim structure)

```markdown
# EPIC-CM-0NN: <Title>

> **Parent:** [Certificate Management Design](../../CERTIFICATE_MANAGEMENT_DESIGN.md) §<n>
> **Phase:** <0–4> — <phase name from design §11>
> **CMMI level advanced:** <Level N · Name, or "—">
> **Status:** Draft — awaiting implementation
> **Priority:** P0 | P1 | P2
> **Estimated effort:** <n–m days>
> **Depends on:** EPIC-CM-0NN (<why>), … or None

---

## Overview
<2–4 paragraphs: what the epic delivers, why it exists, what is explicitly out of scope.>

## Epic-level diagram
<One mermaid diagram showing how this epic's stories fit together or where the epic sits in the system. Choose the type that fits.>

## Stories

| ID | Story | Persona | Effort |
|----|-------|---------|--------|
| **CM-0NN.1** | As a …, I … so that … | … | n d |

## Definition of Done (epic)
- [ ] every story's ACs pass
- [ ] `just verify` (fmt, clippy with workspace lints, nextest) is green
- [ ] docs updated (README capability list, implementation checklist where applicable)
- [ ] <epic-specific items>

## Open questions
| ID | Question | Default if unresolved |
```

## Story section (verbatim structure — every heading is mandatory; write "None." under a heading rather than omitting it)

```markdown
---

## CM-0NN.S — <Story title>

**As a** <persona>, **I want** <capability> **so that** <outcome>.

### Description
<What is built, where in the repo it lives (paths), which existing traits/modules it reuses, what it deliberately does not do. Reference the design doc by section.>

### Diagrams
<At least one mermaid diagram. Pick the best fit and say why in one line above it:
- flowchart — decision logic, reconcile paths
- sequenceDiagram — API calls, UI ↔ API ↔ K8s ↔ controller
- stateDiagram-v2 — condition/lifecycle transitions
- erDiagram — CRD field relationships, data model
- classDiagram — Rust traits/structs
Diagrams must parse with mermaid 11 (validate with `mmdc`). Every block starts with the shared `%%{init: {'theme':'base', …}}%%` directive (copy it from any existing epic) so colours are explicit and readable on both light and dark viewers — never rely on the viewer's default theme, never add `classDef`/`linkStyle` lines. Every `subgraph` MUST be followed by `style <id> fill:transparent,stroke:#8A94A6,stroke-width:1.5px,stroke-dasharray:4 3` — viewers that ignore the init directive otherwise paint clusters with an opaque theme colour that hides labels. No HTML tags inside node labels except `<br/>`. Quote labels containing punctuation.>

### Functional requirements
| ID | Requirement |
|----|-------------|
| **FR-0NN.S-01** | Must … (testable, single behaviour, uses MUST/SHOULD/MAY) |

### Non-functional requirements
| ID | Category | Requirement |
|----|----------|-------------|
| **NFR-0NN.S-01** | Security / Performance / Reliability / Observability / Compatibility / Accessibility / Idempotency | … with a measurable threshold where possible |

### Acceptance criteria
| ID | Given / When / Then |
|----|---------------------|
| **AC-0NN.S-01** | **Given** … **When** … **Then** … |

### Unit test criteria
| ID | Test | Covers |
|----|------|--------|
| **UT-0NN.S-01** | `module::test_name` — what it asserts, with mocks named (e.g. `MockKubeApi`, `MockCertificateAdapter`) | FR-…, AC-… |

### Integration test criteria
| ID | Test | Environment | Covers |
|----|------|-------------|--------|
| **IT-0NN.S-01** | … | kind/k3s via Tilt, `cargo nextest --features integration`, or "None." | … |

### QA process
<Numbered steps a QA engineer follows on the shared-k8s dev cluster (see README "Getting started"): what to deploy (`just dev-up`, `just apply-examples` etc.), what to do, what to observe (`kubectl get … -o yaml`, Events, UI screens, Datadog), and the exit criteria. Include the negative path. End with the artefacts QA attaches to the PR (screenshots, `kubectl` output, test report).>
```

## Style

- Use the repository's conventions: Rust workspace, edition 2024, `kube` 2.0, `clippy::unwrap_used = deny`, `missing_docs = warn`; controllers reuse `KubeApiTrait`, `EventRecorderTrait`, `SecretFetcher`, `BackoffCalculator` from `controllers/netbox`; tests via `cargo nextest`; dev via Tilt on the `shared-k8s` context; UI is SolidJS + Vite + Tailwind.
- CRD group `dcops.microscaler.io`, version `v1alpha1`. Kinds: `CertificateInventory` (plural `certificateinventories`), `CertificateScanImport` (`certificatescanimports`), `CertificateBinding` (`certificatebindings`).
- Condition types and status fields exactly as design §5.2. Metric names exactly as design §8.1.
- HTTP APIs are spec-first: operations are added to `openapi/dcops/certificates/openapi.yaml` and served by the BRRTRouter service (`services/certificates-api`); never add hand-written routes to the controller. Heavy or long-running work belongs in a Job or the controller, never in a request handler.
- Prefer prose in Overview/Description; tables for requirements/criteria; no marketing language.
- Every FR must be traceable to at least one AC and at least one UT or IT. State "None." explicitly for integration tests only when there is truly nothing to integrate.
- Keys and secrets: any story touching Secrets must carry an NFR that key material never appears in status, Events, logs or metrics.
- Length: 250–600 lines per epic. Completeness beats brevity, padding is still padding.
