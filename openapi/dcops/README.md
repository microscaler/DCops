# DCops OpenAPI suite (`dcops`)

Consumer-driven contracts for the DCops web UI. **The browser talks only to the BFF**
(`openapi_bff.yaml`, served at `/api/v1/…`); the BFF proxies to one BRRTRouter service per
domain. Nothing in the SPA talks to the Kubernetes API, and no CRD discovery happens in the
browser — every screen in `docs/03_ui_design` is fed by an operation listed below.

```
openapi/dcops/
├── bff-suite-config.yaml       suite → service base paths (PriceWhisperer layout)
├── openapi_bff.yaml            GENERATED gateway spec (x-service / x-brrtrouter-downstream-path)
├── platform/openapi.yaml       dashboard, discovery, namespaces, events, /me
├── dcim/openapi.yaml           11 NetBox DCIM kinds
├── ipam/openapi.yaml           9 NetBox IPAM kinds
├── tenancy/openapi.yaml        tenants, tenant groups
├── extras/openapi.yaml         tags
├── boot/openapi.yaml           BootProfile, BootIntent (+ lifecycle counts)
├── ippools/openapi.yaml        IPPool, IPClaim  (PLACEHOLDER — see Gaps)
├── certificates/openapi.yaml   inventory, ownership, maturity, scan imports (EPIC-CM-002)
└── scripts/build_specs.py      the only writer of every file above
```

## Regenerating

```bash
python3 openapi/dcops/scripts/build_specs.py \
  --crds config/crd/all-crds.yaml --out openapi/dcops
for f in openapi/dcops/*/openapi.yaml openapi/dcops/openapi_bff.yaml; do
  python3 -m openapi_spec_validator "$f"; done
```

The CRD-backed services (dcim, ipam, tenancy, extras, boot, ippools) are derived from
`config/crd/all-crds.yaml`: each kind's `spec`/`status` `openAPIV3Schema` is converted to
OpenAPI 3.1 (`nullable: true` → `type: [T, "null"]`, `format: uint32` → `int32` with
`minimum: 0`) and wrapped in `<Kind>Row` (table columns: scalar spec fields, state, error,
lastReconciled, netboxId/Url), `<Kind>Detail` (full spec, status, conditions, `manifestYaml`,
`gitUrl`) and `<Kind>List`. Do not edit the YAML by hand; change the generator or the CRD.
CI should fail when a regenerate produces a diff (same rule as PriceWhisperer).

## Conventions

* OpenAPI **3.1.0** everywhere; stays 3.1 — BRRTRouter parses it natively.
* Every operation has `security: [BearerAuth]` (OIDC JWT validated with `JwksBearerProvider`;
  issuer/audience/JWKS URL are deployment config, Entra or sesame-idam — README delta 15).
  `/healthz` operations are the only unauthenticated ones.
* Every response and request body carries an `example` — BRRTRouter's generator needs them
  for the stub handlers and the TestApp fixtures.
* Paths inside a service spec are **service-relative**; `bff-suite-config.yaml` supplies the
  `/api/v1/<service>` prefix and `gateway_path_style: prefixed`. The BFF spec namespaces
  component schemas as `<Service><Name>` (e.g. `DcimDeviceRow`), keeps the seven shared
  schemas unprefixed, writes the full gateway path in `x-brrtrouter-downstream-path`, and marks
  each proxied operation `x-brrtrouter-impl: false` (service specs carry `true` — the service
  implements the operation); a BFF-orchestrated operation would be `true` and be
  listed in `metadata.bff_impl_operations` (none yet).
* List operations share the parameter set `namespace`, `state`, `search`, `limit`, `cursor`,
  `sort` plus per-kind filters (`KIND_FILTERS` in the generator: prefix→vrf/site/status,
  device→site/deviceRole/status, …). Paging is cursor-based; `page.total` is the filtered count.
* Errors are always `ErrorEnvelope { code, message, fields[], requestId }` where `code` is a
  stable machine token (`unknown_format`, `preview_changed`, `phase_mismatch`, `field_conflict`,
  …) and `message` is plain language safe to show to an Operations manager verbatim; the
  certificates service adds `mapping[]` on `unknown_format`.
* `stream_import_events` is `x-sse: true` (`text/event-stream`), the only streaming operation.
* Write operations exist **only** in `certificates` (ownership, acknowledge, imports). The
  NetBox/boot/ippool services are read-only by design: those resources are declared in Git and
  reconciled by the controllers (ADR-001); the UI shows the Git link, it does not edit.

## Screen → operation traceability

All 22 signed-off artboards in `docs/03_ui_design/artboards/`. Paths shown as the browser sees
them (BFF prefix included).

| Screen (artboard) | Operations (BFF path) |
|---|---|
| SignIn | none — OIDC Authorization Code + PKCE against the IdP; the SPA never posts credentials to DCops. After sign-in: `GET /api/v1/platform/me` |
| Main (dashboard) | `GET /api/v1/platform/summary`, `GET /api/v1/platform/attention`, `GET /api/v1/platform/namespaces` (header selector), `GET /api/v1/platform/me` (sidebar footer) |
| DcimOverview | `GET /api/v1/dcim/summary`, `GET /api/v1/dcim/{sites,locations,regions,site-groups,devices,device-types,device-roles,manufacturers,platforms,interfaces,mac-addresses}` (one per sub-tab) |
| DcimDeviceDetail | `GET /api/v1/dcim/devices/{ns}/{name}`, `GET /api/v1/dcim/devices/{ns}/{name}/interfaces`, `GET /api/v1/platform/events?kind=NetBoxDevice&namespace=&name=` (History tab) |
| IpamPrefixes | `GET /api/v1/ipam/summary`, `GET /api/v1/ipam/prefixes?vrf=&site=&status=` and sibling lists (`ip-addresses`, `ip-ranges`, `aggregates`, `vlans`, `rirs`, `vrfs`, `route-targets`, `roles`) |
| IpamPrefixDetail | `GET /api/v1/ipam/prefixes/{ns}/{name}`, `GET /api/v1/ipam/ip-addresses?…` (children), `GET /api/v1/platform/events?kind=NetBoxPrefix…` |
| Tenancy | `GET /api/v1/tenancy/summary`, `GET /api/v1/tenancy/tenants`, `GET /api/v1/tenancy/tenant-groups`, detail `GET …/{ns}/{name}` |
| Extras | `GET /api/v1/extras/summary`, `GET /api/v1/extras/tags`, `GET /api/v1/extras/tags/{ns}/{name}` |
| Boot | `GET /api/v1/boot/summary` (kinds + lifecycle counts), `GET /api/v1/boot/boot-intents?lifecycle=&profile=`, `GET /api/v1/boot/boot-profiles`, detail `GET …/{ns}/{name}` |
| IpPools | `GET /api/v1/ippools/summary`, `GET /api/v1/ippools/pools?vrf=` (`utilisationPercent` computed server-side), `GET /api/v1/ippools/claims?pool=`, detail `GET …/{ns}/{name}` — **placeholder CRDs** |
| CertificatesInventory | `GET /api/v1/certificates/summary`, `GET /api/v1/certificates/inventory?state=&bucket=&owner=&businessUnit=&severity=&expiringWithinDays=&search=` |
| CertificateDetail | `GET /api/v1/certificates/inventory/{ns}/{name}`, `PATCH …/ownership`, `POST …/acknowledge`, `GET /api/v1/platform/events?kind=CertificateInventory…` |
| BulkAssign | `GET /api/v1/certificates/inventory?owner=none…` (selection), `POST /api/v1/certificates/bulk/ownership` (dry run, then apply) |
| CertificatesMaturity | `GET /api/v1/certificates/maturity?environment=&groupBy=` (estate level, per-level counts, weakest critical certs, trend) |
| ImportUpload | `POST /api/v1/certificates/imports` → browser `PUT` to the returned one-time signed URL (bucket, not DCops) → `POST /api/v1/certificates/imports/{id}/uploaded`; `GET …/{id}/events` (SSE) for progress |
| ImportReview | `GET /api/v1/certificates/imports/{id}`, `GET …/{id}` (mapping, warnings, totals, preview expiry), `GET …/{id}/preview?outcome=&limit=&cursor=` (diff rows), `POST …/{id}/repreview` (corrected column mapping), `GET …/{id}/artifacts/preview` (302 to signed GET) |
| ImportConfirm | `POST /api/v1/certificates/imports/{id}/confirm` (idempotent; refused with 409 if the preview changed or expired), `POST …/{id}/cancel` |
| ImportSummary | `GET /api/v1/certificates/imports/{id}` (phase Completed/Failed, counters), `GET …/{id}/artifacts/result` (302), `GET …/{id}/events` until terminal |
| ImportHistory | `GET /api/v1/certificates/imports?namespace=&phase=&limit=&cursor=` |
| Docs | static content bundled with the SPA; `GET /api/v1/platform/kinds` renders the "what DCops manages" table |
| EmptyState | any list operation returning `items: []` with `page.total: 0`; `GET /api/v1/platform/kinds` supplies the kind description for the empty message |
| ErrorState | any operation returning `ErrorEnvelope` (401 → re-auth via OIDC; 403 → role message from `/me`; 5xx → `requestId` shown for support) |

Role gating (write buttons hidden, not just rejected): every `PATCH/POST` under
`/certificates` requires `dcops:certificates:import` (mapped from IdP claims — EPIC-CM-002.2);
`dcops:certificates:admin` is reserved for controller-side policy changes and is not used by
any UI operation yet. The SPA reads its roles from `GET /api/v1/platform/me`.

## Operations by service

| Service | Ops | Notes |
|---|---|---|
| platform | 7 | summary, attention, kinds, namespaces, events, me, healthz |
| dcim | 24 | 11 kinds × list/get, summary, device interfaces |
| ipam | 19 | 9 kinds × list/get, summary |
| tenancy | 5 | 2 kinds, summary |
| extras | 3 | tags, summary |
| boot | 5 | 2 kinds, summary with lifecycle counts |
| ippools | 5 | 2 kinds, summary |
| certificates | 18 | inventory 3, ownership 3, maturity 1, imports 10, healthz |
| **BFF** | **86** | 85 paths, 174 schemas (167 service-prefixed + 7 shared: ErrorEnvelope, ResourceMeta, Condition, Event, PageInfo, KindCount, State), all operationIds unique |

## Gaps and decisions recorded here

1. **IPPool / IPClaim CRDs do not exist in `config/crd/`** although the mockups (`IpPools`)
   and the old dashboard design list them. The generator carries `PLACEHOLDER_CRDS` with the
   minimal spec the mockup needs (`prefix`, `vrf`, `capacity`/`allocated`; claim → `pool`,
   `address`). When the real CRDs land, delete the placeholder and regenerate — the routes and
   schema names stay the same.
2. **`NetBoxInterface` `device` filter** is served from the interface's `spec.device` reference,
   not via a NetBox query; the DCIM device detail "Interfaces" tab is a filtered list of CRs.
3. **Events** come from a single platform operation (`/platform/events`) rather than one per
   service so the History tab is identical on every detail page. The service watches
   `Event` objects in the namespaces the caller may read (`/platform/namespaces` is the
   authoritative list; it replaces the browser-side namespace probe).
4. **Namespaced schema names in the BFF** are prefixed by service (`CertificatesImportAccepted`),
   which is what `brrtrouter-gen` emits for the suite client; the SPA imports the generated
   client from the BFF spec only.
5. Certificate import upload goes browser → bucket via the signed URL the API mints; the BFF
   never carries file bytes (`x-brrtrouter-body-size-bytes` stays at the default 1 MiB on
   `create_import`, whose body is metadata plus a base64 sample of the first 8 KiB, ≤ 12,288 chars).

Related: `docs/02_certificate_management/stories/EPIC-CM-002-scan-import-api.md` (service
implementation), `EPIC-CM-004` (UI consumes this suite), `docs/01_dashboard_design.md`
(architecture), README delta 16.
