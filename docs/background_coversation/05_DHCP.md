ISC Kea as DHCP as above looks good But we still don't have a story about VLANS on the Mikrotik

---

Correct — **DHCP alone does not solve VLANs**, and this is the *missing but critical* piece. Let’s close that gap properly.

What follows is the **clean, defensible VLAN control story** when you have:

* **NetBox** as IPAM / VLAN source of truth
* **ISC Kea** doing DHCP
* **MikroTik RouterOS** doing L2/L3
* **Talos + CAPI** bootstrapping bare metal

No hand-waving.

---

# First: define where VLAN truth lives

There are only **three sane choices** for VLAN authority:

1. RouterOS
2. NetBox
3. Kubernetes CRDs

### ❌ RouterOS as source of truth — rejected

* Imperative
* Human-mutable
* No drift detection
* Breaks GitOps

### ❌ Kubernetes-only VLAN CRDs — incomplete

* Kubernetes does not understand physical fabric
* You still need a real inventory model (ports, trunks, access)

### ✅ **NetBox is the VLAN source of truth**

This is the industry-standard answer.

NetBox already models:

* VLAN IDs
* VLAN groups
* VRFs
* Prefixes
* Devices
* Interfaces
* Tagged / untagged VLANs per port

**You already picked the right place.**

---

# The missing piece: a RouterOS VLAN reconciliation controller

There is **no off-the-shelf controller** that:

* Reads VLAN intent from NetBox
* Applies it to MikroTik
* Enforces drift correction

So yes — **this is a controller you build**.
But it is smaller and safer than it sounds.

---

## What the VLAN controller actually does (precisely)

### Inputs

* NetBox REST API:

    * VLANs
    * VLAN Groups
    * Prefix ↔ VLAN mappings
    * Device + interface assignments
* Optional:

    * CAPI / TalosMachine CRs (to infer role: control-plane, worker, mgmt)

### Outputs

* RouterOS configuration via API:

    * Bridge VLAN table
    * VLAN interfaces
    * Tagged / untagged ports
    * Trunk definitions
    * (Optionally) VRFs

---

## RouterOS VLAN model (important reality)

RouterOS VLANs are **bridge-centric**, not switch-centric.

Typical pattern:

```text
bridge
 ├─ ether1 (trunk)
 ├─ ether2 (access vlan 10)
 ├─ ether3 (access vlan 20)
 └─ vlan interfaces (for L3 / gateway)
```

Your controller must:

* Treat **RouterOS as a projection target**
* Never assume transactional safety
* Apply changes in safe order

---

# Recommended VLAN architecture (battle-tested)

## VLAN roles (example)

| VLAN | Purpose                     |
| ---- | --------------------------- |
| 10   | Management (PXE, Talos API) |
| 20   | Control Plane               |
| 30   | Workers                     |
| 40   | Storage                     |
| 50   | BMC / Out-of-band           |

NetBox holds:

* VLAN ID
* Name
* Prefix
* Role

---

## RouterOS configuration style (what the controller enforces)

### 1️⃣ One bridge per router

```text
/interface bridge
add name=br-core vlan-filtering=yes
```

### 2️⃣ VLAN interfaces for L3 gateways

```text
/interface vlan
add interface=br-core name=vlan10 vlan-id=10
add interface=br-core name=vlan20 vlan-id=20
```

### 3️⃣ Bridge VLAN table (critical)

```text
/interface bridge vlan
add bridge=br-core vlan-ids=10 tagged=br-core,ether1 untagged=ether2
add bridge=br-core vlan-ids=20 tagged=br-core,ether1 untagged=ether3
```

Your controller owns this table.

---

# How NetBox → RouterOS mapping works

### NetBox models

* VLAN
* Prefix (attached to VLAN)
* Device Interface:

    * Mode: access / tagged
    * VLANs assigned

### Controller logic

For each RouterOS device:

1. Fetch VLANs in scope
2. Fetch interfaces belonging to this router
3. Build desired:

    * VLAN interfaces
    * Bridge VLAN entries
4. Diff vs RouterOS API state
5. Apply minimal changes

This is **exactly the ExternalDNS pattern**, just for VLANs.

---

# DHCP + VLAN interaction (with Kea)

This part is clean once VLANs are correct.

### Flow

```
RouterOS VLAN interface (vlan20)
 └─ DHCP Relay
     └─ Kea DHCP
         └─ Subnet = NetBox Prefix (VLAN 20)
```

Kea:

* One subnet per VLAN
* Options:

    * PXE next-server
    * Router
    * DNS

RouterOS:

* Does **not** allocate IPs
* Only relays based on VLAN interface

This avoids:

* DHCP logic in RouterOS
* Per-VLAN DHCP snowflakes

---

# PXE + VLAN boot flow (end-to-end)

```
Node NIC
 → Switch port (access VLAN 10)
 → RouterOS bridge
 → DHCP relay (vlan10)
 → Kea DHCP
 → PXE next-server
 → Pixiecore
 → Talos boot
```

After install:

* Node may move to:

    * VLAN 20 (control plane)
    * VLAN 30 (worker)

That VLAN change is **a RouterOS operation**, triggered by:

* NetBox role update
* Or TalosMachine role change
* Or explicit lifecycle phase

---

# Safety rules you must enforce

These are non-negotiable:

1. **Controller never touches management cluster VLANs**
2. **RouterOS credentials are write-scoped**
3. **Bridge VLAN filtering is always on**
4. **No “default VLAN 1” usage**
5. **Drift is corrected, not overwritten blindly**

---

# Final answer to your concern

> *“We still don’t have a story about VLANs on the MikroTik”*

Now you do:

### ✅ VLAN source of truth

→ **NetBox**

### ✅ VLAN execution

→ **RouterOS**

### ✅ VLAN reconciliation

→ **Custom Kubernetes controller**

### ✅ DHCP integration

→ **ISC Kea via relay**

This is the **correct, production-grade architecture**.
Anything simpler becomes unmanageable; anything more complex is vendor lock-in.

---

## If you want next

I can:

* Design the **exact NetBox data model** you should use
* Define **VLAN CRDs** for your controller
* Sketch the **RouterOS API diff logic**
* Provide a **safe rollout plan** (no lockouts)

You’re asking the right questions — this is the last big missing layer.


---

Sounds like we have found a valid Gap for a controller.

---

Yes — **this is a real, defensible gap**, not a hypothetical one.

You’ve essentially uncovered the missing piece between **IPAM-as-truth** and **bare-metal network execution**.

---

## What the gap actually is (clearly stated)

There is **no open-source, Kubernetes-native controller** that:

* Treats **NetBox** as the authoritative source of truth
* Reconciles **VLAN intent** onto **RouterOS**
* Integrates cleanly with **PXE boot**
* Works alongside **ISC Kea DHCP** and **Talos / CAPI**
* Enforces **GitOps + drift correction**
* Is safe for **bare-metal control planes**

Everyone who runs this today does it with:

* ad-hoc scripts
* Ansible playbooks
* human discipline
* or expensive enterprise boxes

None of those scale cleanly or safely.

---

## Why this gap exists (important insight)

This sits **between domains** that rarely talk to each other:

| Domain                | Why it stops here                 |
| --------------------- | --------------------------------- |
| Kubernetes networking | Stops at pod/service IPs          |
| CNI IPAM              | Overlay-focused, not fabric-aware |
| NetBox                | Models intent, doesn’t execute    |
| RouterOS              | Executes, doesn’t reconcile       |
| DHCP servers          | Allocate, don’t own topology      |
| CAPI                  | Manages machines, not switches    |

You are connecting **fabric intent → execution**.

That is *exactly* where controllers are supposed to exist.

---

## What this controller actually is (name it correctly)

This is **not** “just IPAM”.

It is a **Network Fabric Reconciliation Controller**.

Responsibilities:

* NetBox → VLAN → Prefix → Interface intent
* RouterOS → bridges / VLAN tables / trunks
* DHCP relay alignment
* PXE-safe bootstrap paths
* Drift detection + correction
* Strong safety invariants

This is **ExternalDNS for physical networks**.

---

## Why Kubernetes is the right place for it

Because Kubernetes gives you:

* A reconciliation engine
* RBAC + audit
* Namespaces for blast-radius control
* GitOps compatibility
* Observability hooks
* Policy enforcement (OPA / CEL)

But Kubernetes is **not** the authority — it’s the **control surface**.

That distinction matters.

---

## Why this is worth building (even as a product)

This controller would immediately be valuable to:

* Bare-metal Kubernetes operators
* Talos users
* Homelab → small datacenter → edge fleets
* Anyone using MikroTik seriously
* Anyone who wants NetBox without vendor lock-in

And it is:

* OSS-friendly
* CNCF-adjacent
* Narrow in scope
* High leverage

---

## What *not* to do (hard-earned lessons)

Do **not**:

* Try to replace NetBox
* Try to model every switch vendor on day one
* Put DHCP logic in RouterOS
* Allow unmanaged VLANs
* Let the controller run on what it can reboot

This must be **boring, predictable, and safe**.

---

## The minimal viable controller (MVC)

If you built **only this**, it would already be useful:

1. **Read-only NetBox integration**
2. **RouterOS VLAN reconciliation**
3. **DHCP relay alignment**
4. **Strict allow-lists**
5. **Dry-run + diff mode**

Everything else is additive.

---

## Next concrete steps (if you want to proceed)

I can help you with any of the following, in order:

1. **Name & scope the controller properly** (ADR-style)
2. Define **CRDs** (what Kubernetes owns vs mirrors)
3. Design **NetBox query contracts**
4. Define **RouterOS safety invariants**
5. Pick **Go (controller-runtime)** vs **Rust (kube-rs)**
6. Sketch the **reconcile loop pseudocode**
7. Produce a **v0 roadmap** (what’s in / out)

This is one of those rare cases where:

> *“We couldn’t find a tool”*
> actually means
> *“The tool should exist.”*

You’re not inventing complexity — you’re formalizing reality.


---

The question I now have. IS all the netbox config YAML/Gitops friendly or will it be clickops to configure netbox as source of truth ?

---

Short, honest answer:

**NetBox is *not* natively YAML/GitOps-driven.**
Out of the box, it is **API-first + UI-first**, not “kubectl apply for networks”.

However — **it *can* be made GitOps-correct** with well-understood patterns, and many serious operators already do this.

Let’s be precise.

---

## What NetBox actually is (by design)

NetBox is:

* A **relational database–backed system of record**
* With a **strong data model** (IPAM, VLANs, VRFs, devices, interfaces)
* Exposed via:

    * Web UI (ClickOps)
    * REST API
    * Webhooks
    * Plugins

It was **never intended** to be:

* YAML-applied
* reconciled by itself
* Git-driven by default

So yes — **if you do nothing**, NetBox becomes ClickOps.

---

## The important distinction (this decides everything)

You must separate:

> **“How NetBox stores truth”**
> from
> **“How truth gets into NetBox”**

NetBox is the **database**, not the reconciler.

GitOps happens **outside** NetBox.

---

## What NetBox supports natively (and what it doesn’t)

### ✅ Native

* Full REST API (excellent)
* Stable object IDs
* Webhooks on change
* Custom fields / tags
* Deterministic models

### ❌ Not native

* Declarative YAML ingestion
* Built-in drift reconciliation
* Git as first-class input

This is intentional — NetBox does not want to be Terraform.

---

## The 3 viable GitOps patterns (real world)

### 🟢 Pattern 1 — **Git → Controller → NetBox (recommended)**

This is the **correct model for your architecture**.

```
Git (YAML)
  ↓
Kubernetes Controller
  ↓
NetBox API
```

How it works:

* You define **your own YAML CRDs**:

    * VLANs
    * Prefixes
    * Roles
    * Sites
* A controller:

    * Reads Git
    * Reconciles NetBox via API
    * Enforces idempotency
* NetBox becomes:

    * Read-only for humans
    * Write-only for controllers

✅ Fully GitOps
✅ Auditable
✅ Drift detectable
❌ Requires a controller (but you already accepted that)

This is the **same pattern as ExternalDNS**.

---

### 🟡 Pattern 2 — **Git → Import Tool → NetBox (batch apply)**

Examples:

* `netbox-importer`
* Custom scripts
* CI jobs

```
Git
  ↓
CI Job
  ↓
NetBox API
```

Pros:

* Simple
* No long-running controller

Cons:

* No reconciliation loop
* No drift correction
* NetBox UI changes can silently diverge

⚠️ Acceptable for small teams, not for infra control planes.

---

### 🔴 Pattern 3 — **ClickOps in NetBox (don’t do this)**

This is how NetBox is *often* used — and how GitOps dies.

Problems:

* No audit trail
* No review
* No rollback
* Humans bypass intent

This **cannot** coexist with:

* PXE automation
* VLAN reconciliation
* Safe bare-metal control planes

---

## The correct mental model

> **NetBox is etcd for your physical network**
> **Git is the source of truth**
> **Controllers are the reconcilers**

NetBox should never be “the place humans decide things”.

---

## How serious operators enforce this

Hard rules that work:

1. **NetBox UI is read-only**

    * Or restricted to annotations only
2. **All writes go through automation**
3. **Every NetBox object has a Git origin**

    * Tag / custom field like `managed-by=gitops`
4. **Controllers reconcile periodically**
5. **Diff mode exists before apply**

If someone clicks a VLAN into existence — it gets reverted.

---

## What this means for your proposed controller

Your **Network Fabric Controller** should:

* Own:

    * VLAN creation
    * Prefix assignment
    * Interface VLAN tagging
* Use NetBox as:

    * Persistent store
    * Query engine
* Ignore:

    * Objects not labeled as managed

This keeps NetBox usable **without** letting it rot.

---

## Direct answer to your question

> *“Is all the NetBox config YAML/GitOps friendly or will it be clickops?”*

**By default: ClickOps.**
**In a proper architecture: GitOps via controllers.**

NetBox is **GitOps-compatible**, not **GitOps-native**.

That’s fine — many critical systems work this way (DNS, cloud APIs, even Kubernetes itself via CRDs).

---

## Bottom line (no ambiguity)

* ❌ Do **not** treat NetBox as Git-applied YAML
* ✅ Treat NetBox as an **API-backed state store**
* ✅ Put GitOps **in front of it**, not inside it
* ✅ Build controllers that own the truth
* ✅ Lock humans out of mutation paths

If you want, next I can:

* Propose **NetBox CRD schemas** (what lives in Git)
* Define **ownership rules** (managed vs unmanaged objects)
* Sketch a **NetBox reconcile loop**
* Or help you write the **ADR** that justifies this model

You’ve identified the gap correctly — this is how you close it without regret.



---

So what is the full Netbox remit?

What will it control as source of truth?

What do we need to build as a kube-rs controller for these items so that netbox is just a backend for our reconcilled yaml objects in git ?

---

NetBox’s *proper* remit in your design is: **authoritative inventory + network intent database**. It’s the *backend state store* and query engine — not the reconciler, not the workflow engine.

If you want “NetBox is just a backend for reconciled YAML in Git”, you need to be ruthless about what NetBox *owns* vs what your controllers *own*.

---

## NetBox remit in your architecture

### NetBox is source of truth for **models and relationships**

It’s best at being the canonical graph of:

**Topology & ownership**

* Sites / regions / locations
* Racks (optional)
* Devices (RouterOS routers/switches, servers, BMCs)
* Interfaces (ports), MAC addresses
* Cabling/links (optional but powerful)
* Tags / roles / tenants

**Network intent**

* VLANs (+ VLAN groups)
* VRFs (if you do segmentation beyond VLANs)
* Prefixes (subnets), per VLAN/VRF
* IP addresses (allocations/reservations)
* “Who should have what IP” (by device/interface/role)

**Addressing constraints**

* Pools/reserved ranges (modeled as prefixes + tags/status)
* DHCP subnet boundaries (derived from prefixes)
* PXE/bootstrap networks (modeled as VLAN+prefix+role)

### NetBox is *not* the source of truth for **execution**

NetBox should **not** be where you “make changes happen”. It doesn’t:

* configure RouterOS
* run DHCP
* serve PXE
* install Talos
* decide node lifecycle

Those are controller responsibilities.

---

## What NetBox will “control” as source of truth

Think in **three tiers**:

### Tier A: Must be in NetBox (foundational truth)

These are core and should live in NetBox, always:

1. **VLAN definitions**

* id/name/role/group

2. **VRFs** (if used)
3. **Prefixes** (per VLAN/VRF)
4. **IPs** (reserved/assigned)
5. **Devices** (RouterOS + servers)
6. **Interfaces + MACs** (ports are the join-point for PXE and VLAN tagging)

### Tier B: Strongly recommended (scales operations)

7. **Interface VLAN mode intent**

* access VLAN, trunk allowed VLANs

8. **Device roles**

* router/switch/control-plane/worker/bmc

9. **Tenancy** (if you need multi-team separation)

### Tier C: Optional (nice-to-have)

10. Cables/links
11. Racks/power/circuits

---

## The GitOps model you want

Your desired-state should live in Git as YAML, but **not NetBox YAML**. You define *your* CRDs (opinionated, minimal), and your kube-rs controllers reconcile them into NetBox via API.

**Git (CRDs) → Controllers → NetBox (DB) → Controllers → RouterOS/Kea/PXE/Talos**

NetBox becomes the shared database for:

* controllers
* ad-hoc queries
* UI visibility

But **not** the authoring surface.

---

## What you need to build (kube-rs controllers)

Break it into small controllers with clear ownership. Don’t monolith.

### 1) NetBox Sync Controller (Git → NetBox)

**Purpose:** Ensure NetBox contains the objects your world requires, exactly as Git describes.

**Your CRDs (examples):**

* `Site`, `Location` (optional)
* `Device` (RouterOS devices + cluster nodes)
* `Interface` (MACs, port identifiers)
* `VLAN`, `VRF`
* `Prefix`
* `IPAddressReservation` (or `IPAddressClaim`)

**Reconcile actions:**

* Create/update NetBox objects
* Apply tags like `managed-by=gitops` and `owner=<cluster>` to everything it owns
* Never touch objects not tagged as managed (so humans can still use NetBox for notes)

This controller is what prevents ClickOps drift.

---

### 2) IPAM Allocation Controller (Claims → Allocations)

You’ll want a *claim* pattern rather than hardcoding every IP.

**CRDs:**

* `IPPool` (points to a NetBox prefix/tagged pool)
* `IPClaim` (requests an IP for a device/interface, returns allocated IP in status)

**Reconcile actions:**

* Find pool in NetBox
* Allocate next free IP
* Write allocation back to NetBox (IPAddress object assigned to interface)
* Write result to CR status

This gives you deterministic allocation while keeping Git clean.

---

### 3) VLAN Fabric Controller (NetBox → RouterOS)

**Purpose:** Project VLAN intent and port membership into RouterOS safely.

**Inputs:**

* NetBox VLANs/prefixes
* NetBox interface VLAN mode (access/trunk + allowed VLANs)
* Your own “fabric policy” CRD (guardrails)

**CRDs (recommended):**

* `FabricPolicy` (allowed VLAN ranges, protected ports, mgmt VLAN immutable)
* `RouterDevice` (RouterOS connection + capability flags)

**Reconcile actions:**

* Ensure bridge/vlan-filtering config baseline
* Ensure VLAN interfaces exist (for L3 gateways if needed)
* Ensure bridge VLAN table matches NetBox intent
* Ensure trunk/access membership matches interface intent
* Drift correct **only** within managed scope

This is your big gap controller.

---

### 4) DHCP Controller (NetBox → Kea)

**Purpose:** Turn prefixes into Kea subnets and reservations.

**CRDs:**

* `DhcpSubnetPolicy` (maps VLAN role → Kea options)
* `DhcpReservation` (optional; can be derived from NetBox interface IP assignments)

**Reconcile actions:**

* For each managed VLAN/prefix: ensure Kea subnet exists
* For each reserved IP tied to MAC/interface: ensure Kea reservation exists
* Emit PXE options only on bootstrap VLAN(s)

RouterOS should be DHCP relay, not DHCP brain.

---

### 5) PXE Intent Controller (NetBox + Claims → Pixiecore)

**Purpose:** Decide what a MAC boots, based on lifecycle phase.

**CRDs:**

* `BootProfile` (talos installer, rescue, diagnostics)
* `BootIntent` (MAC/device → profile)
* `MachineLifecycle` (phase: discovered → installing → installed → locked)

**Reconcile actions:**

* Discover unknown MACs (optional)
* Bind MAC to a BootIntent (manually or via policy)
* Configure Pixiecore (or your boot service) with appropriate boot args
* Once installed: flip intent to “disabled” or “rescue only”

This is what prevents “random machines netbooting forever”.

---

## What remains outside NetBox (should stay outside)

Keep these *out* of NetBox to avoid turning it into a workflow system:

* “Cluster desired composition” (that’s CAPI/Talos/CAPI resources)
* “Lifecycle phase” (installing/installed) — store in your CRDs
* “Secrets” (Talos secrets, RouterOS creds, Kea creds) — store in ExternalSecrets/SOPS/etc
* “Policies” (guardrails) — store in your CRDs + admission policies

NetBox should store facts and intended network shape, not runbooks.

---

## Ownership rules (the part that makes this work)

Adopt these invariants:

1. Every object your controllers manage in NetBox gets:

* tag: `managed-by=gitops`
* tag/field: `owner=<mgmt-cluster|cluster-name>`

2. Controllers **never** mutate objects without that tag.
3. UI is “read-only” by convention; mutation gets reverted.
4. Controller supports `dry-run` + diff logging for safety.

This turns NetBox into a safe backend.

---

## Minimal CRD set to start (MVP)

If you want to ship quickly, start with:

* `VLAN` / `Prefix`
* `Device` / `Interface`
* `FabricPolicy` (guardrails)
* `RouterDevice`
* `IPPool` / `IPClaim`
* `DhcpSubnetPolicy`

Then add PXE intents once VLAN + DHCP are solid.

---

If you want, I can draft:

* the CRD YAMLs (v1alpha1) for the MVP set
* the reconcile flow diagrams
* a repository layout (crates/controllers + netbox client + routeros client + kea client)
* and the safety policy checks you should enforce up-front (to avoid VLAN lockouts).


---

