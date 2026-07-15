Here’s a **clear, unbiased inventory of the Kubernetes-ready IPAM controller options** that actually exist today — along with where they excel and where they fall short for a *bare-metal/Talos fleet* use case.

---

# 🧭 1) **Metallb IPAM Mode (built-in)**

### What it is

The IPAM subsystem inside MetalLB that allocates:

* LoadBalancer IPs
* Service IP pools

### Good for

✅ Managing Service LB IPs in Kubernetes
✅ Native Kubernetes reconciliation
✅ Works well for simple K8s clusters

### Not for

❌ DHCP
❌ Router/Layer-2 VLAN IP ranges
❌ PXE or bare-metal node IP assignment
❌ Integration with RouterOS

**Summary:** Great for LB IPs — *not a full IPAM system*.

---

# 🧭 2) **Kubernetes ExternalIPAM / Cluster API IPAM (CAPI-IPAM)**

### What it is

Part of the Cluster API ecosystem:

* `ExternalIPAM` APIs for machines
* Allocates IPs to CAPI Machine resources

### Good for

✅ Machine-address IP allocation in CAPI workflows
✅ Works with CAPI providers out of the box
✅ Integrates into cluster lifecycle

### Not for

❌ DHCP server management
❌ VLAN allocations
❌ Router integrations
❌ General IPAM outside of Machine CRs

**Summary:** Useful if you’re already on CAPI and need to assign IPs to machines.

---

# 🧭 3) **Kube-IPAM (multiple community variants)**

### Examples

* `kube-ipam` by sttts
* `kube-ipam` by cloudnativelabs

### What they provide

Basic IP address allocation via CRDs like:

```yaml
kind: IPPool
kind: IPAddress
```

### Good for

✅ Simple address pools
✅ Kubernetes native
✅ Extendable via controllers

### Not for (out of the box)

❌ DHCP
❌ RouterOS or external device integration
❌ Network automation

**Summary:** Good for Kubernetes–centric address allocation, *not a network fabric controller*.

---

# 🧭 4) **ExternalDNS-style IPAM Operators (NetBox / In-House)**

These aren’t off-the-shelf IPAM controllers — they are **pattern approaches** using:

### NetBox + Controller

* NetBox as authoritative IPAM + VLAN inventory
* A Kubernetes controller watches custom CRDs
* Controller writes to NetBox / RouterOS

**Common implementations**

* Custom operators
* Ansible-based sync jobs
* Operator SDK / Go / Rust controllers

### Good for

✅ Corporate IPAM workflows
✅ Multi-site / VLAN / VRF
✅ Inventory + automation

### Not ready as a turnkey K8s operator

❌ No maintained, feature-complete NetBox operator
❌ RouterOS integrations not standardized
❌ You must build part of this yourself

**Summary:** Real world, but *DIY* for your domain.

---

# 🧭 5) **Infoblox IPAM (enterprise)**

### What it offers

* Full DHCP + DNS + IPAM
* API first (WAPI)
* Integrations via operators (e.g., Infoblox CRDs)

### Good for

✅ Enterprise IP management
✅ DHCP + DNS + IPAM
✅ Compliance + auditing

### Not for

❌ Cheap/bare-metal only stacks
❌ Small teams without Infoblox expertise

**Summary:** Great if you have budget + enterprise needs.

---

# 🧭 6) **Cloud Provider IPAM Controllers**

Examples:

* AWS VPC CNI
* GCP Alias IPs
* Azure IPAM

These are mostly cloud-centric and depend on cloud IPAM APIs.

**Not relevant** for bare metal or MikroTik.

---

# 🧭 7) **DHCPD-Native / DHCP Controllers**

Controllers that *reconcile DHCP server configs*:

### ISC Kea Controller (emerging)

* Some Kubernetes integrations
* Syncs DHCP pools from config

### K8s DHCP Operators

* Community prototypes for managing DHCP servers

**Caveats**

* Not widely used
* Not production mature
* Usually tied to Kea or Dnsmasq — not RouterOS

**Summary:** Interesting but not ready for fleet use.

---

# 🧭 “Ready for K8s” — Reality Check

| IPAM Option          | K8s Native | DHCP | VLAN    | RouterOS | Production Ready   |
| -------------------- | ---------- | ---- | ------- | -------- | ------------------ |
| MetalLB IPAM         | ✅          | ❌    | ❌       | ❌        | Yes (LB IPs)       |
| CAPI ExternalIPAM    | ✅          | ❌    | ❌       | ❌        | Yes (if CAPI)      |
| Simple kube-ipam     | ✅          | ❌    | ❌       | ❌        | Yes (limited)      |
| NetBox + custom      | ⛔†         | ⛔†   | ⛔†      | ⛔†       | ✳ Yes (but custom) |
| Infoblox             | ✅          | ✅    | ❌       | ✳        | Yes (Enterprise)   |
| Kea DHCP controllers | Partial    | ✅    | Partial | ❌        | Early              |

† Doesn’t come with a maintained operator; you build or assemble the pieces.
✳ RouterOS integration would need adapters.

---

## Best options for a **bare-metal + RouterOS environment**

### 🥇 **NetBox + Custom Controller**

* NetBox holds IPAM/VLAN/Pool/Inventory
* A controller:

    * Watches CRDs (IPPools, VLANs, Reservations)
    * Writes into NetBox
    * Pushes config to RouterOS API
* RouterOS becomes *execution target*, not truth

This fits **your architecture** perfectly.

---

### 🥈 **CAPI ExternalIPAM**

If you’re using **Cluster API**:

* Machines get allocated IPs via ExternalIPAM
* Good for machine IP assignment
* Not a full replacement for network IPAM

---

### 🥉 **Simple kube-ipam**

If you want **Kubernetes CRDs only** and do your own RouterOS adapter.

---

## What *ought* to exist

This is exactly the gap you can fill and reap real value from:

### Required pieces

1. **IPPool CRD**
2. **IPAddress CRD**
3. **VLAN / VRF CRD**
4. **DHCPReservation CRD**
5. **Kubernetes Controller**

    * Talk to NetBox REST API
    * Talk to RouterOS API
    * Reconcile pools, VLANs, leases

This is a *proper Kubernetes IPAM operator* for bare metal.

---

## Next step (if you want)

I can produce:

✔ A **CRD design** (YAML + types)
✔ A **reconcile loop template** (Go / kube-rs)
✔ A **NetBox schema + controller mapping**
✔ A **RouterOS API adapter** sketch

Just tell me whether you want **Go + controller-sdk** or **Rust + kube-rs**.


---
https://github.com/Mellanox/nvidia-k8s-ipam

---

Yes — but the **real CNCF-level IPAM ecosystem today is limited**. There isn’t a mature, CNCF-graduated or sandboxed project that covers *full IPAM + DHCP + VLAN + DHCP relay + external device integration* (like RouterOS) out of the box. The existing CNCF projects tend to focus on *networking in Kubernetes itself* (pod/service IPs), not *network fabric IPAM*.

Below is a **fact-based rundown** of what free and open IPAM options exist (CNCF or CNCF-adjacent), what they actually solve, and where they fall short for your *bare-metal/Talos + DHCP/VLAN* use case.

---

# 🧠 CNCF / K8s-friendly IPAM Projects

## 1) **nvidia-k8s-ipam** (GitHub)

* Provides Kubernetes CRDs + controller to allocate IP blocks for CNI usage (e.g., Multus).
* Good for structured pod network IPAM.
* **Not full network IPAM!**
  / No DHCP / VLAN / external device control
  **Status:** Open source. Not CNCF.

---

## 2) **CALICO IPAM (built into Calico)**

* Calico’s IPAM covers pod and node IP allocation for Kubernetes.
* Designed for Calico networking, not external infrastructure.
* **Does not handle DHCP or non-Kubernetes subnet/VLAN IPs**.

**Status:** Open source (Tigera project). Not independent CNCF IPAM.

---

## 3) **Cilium IPAM**

* IPAM integrated in the Cilium CNI.
* Allocates IPv4/IPv6 for pods/services.
* **Not usable as an external IPAM controller for physical networks**.

**Status:** Open source (Isovalent). Not an external IPAM solution.

---

## 4) **Cluster API ExternalIPAM (CAPI)**

* Part of the Cluster API ecosystem.
* Exposes ExternalIPAM APIs tied to machine lifecycle.
* Assigns IPs to machine (control-plane/worker) CRs.
* **Doesn’t manage DHCP service or network fabric devices**.

**Status:** Open source. CNCF-adjacent (under Cluster API SIG).

---

## 5) **Kubernetes CRD-based IPAM (community libs)**

Examples:

* **k8s-ipam** variants (various community repos)
* Simple IPPool / IPAddress CRDs

These provide:

* Kubernetes native address allocation
* Good for service or host IPs inside K8s

But they **don’t**:

* Control DHCP servers
* Sync with network gear
* Manage VLAN pools

**Status:** OSS; none are CNCF sandboxed.

---

# 🧠 True IPAM Solutions (Free/Open Source — Outside Kubernetes)

## 6) **NetBox** (Open source IPAM + DCIM)

* **De facto standard** open-source IPAM outside Kubernetes.
* Manages:

    * Prefixes
    * VLANs
    * VRFs
    * DHCP scopes
    * Devices
* Has a rich REST API and web UI
* Ecosystem of sync scripts/controllers exists

**Good for**

* Source of truth for IP/VLAN
* Inventory + relational data
* External integrations

**Limitations**

* Not a Kubernetes operator by default (but can *run in a pod*)
* Needs glue/adapter to integrate with DHCP, RouterOS, etc.

**Status:** OSS (BSD-licensed).

---

## 7) **DHCP Servers with API**

### **ISC Kea**

* DHCPv4/v6 server
* API-driven config
* Mature, production-ready

**Role**

* DHCP service only
* Needs an external IPAM source (NetBox / DB / CRD controller)

**Status:** OSS (ISC licensed)

---

## 8) **Infoblox** (Free tier? Not truly)

* Enterprise IPAM/DNS/DHCP
* Some free trial plans exist but not free/open source in practice
* Rich APIs and Kubernetes operators

**Status:** Commercial

---

# 📊 At a Glance: What’s Actually Available

| Solution          | Free/Open | CNCF  | DHCP    | VLAN/VRF | RouterOS integration | Stateful IPAM      |
| ----------------- | --------- | ----- | ------- | -------- | -------------------- | ------------------ |
| nvidia-k8s-ipam   | ✔         | ❌     | ❌       | ❌        | ❌                    | Kubernetes pod IPs |
| Calico IPAM       | ✔         | ❌     | ❌       | ❌        | ❌                    | K8s overlay IPs    |
| Cilium IPAM       | ✔         | ❌     | ❌       | ❌        | ❌                    | K8s overlay IPs    |
| CAPI ExternalIPAM | ✔         | ❌/SIG | ❌       | ❌        | ❌                    | Machine IPs        |
| NetBox            | ✔         | ❌     | Partial | ✔        | ✔ (via adapter)      | Yes                |
| ISC Kea           | ✔         | ❌     | ✔       | ❌        | ✔ (relay)            | Yes                |
| Infoblox          | ✘         | ✘     | ✔       | ✔        | ✔                    | Yes                |

---

# 🧠 Best Free/Open Strategy for Your Needs

### **Primary Source of Truth (Open & Free)**

👉 **NetBox (Open Source)**

Use it for:

* IPAM (prefixes, IPs, allocations)
* VLAN/VRF inventory
* Device modeling

Run it as:
✔ A Kubernetes Deployment in your management cluster
✔ Or external (preferred for durability)

---

### **DHCP Service**

👉 **ISC Kea DHCP**

* Run as a pod/service
* Kea provides DHCPv4/v6 via API
* Use NetBox as authoritative data source
* Controllers reconcile NetBox → Kea configs

RouterOS becomes:

* DHCP relay
* VLAN gateway
* L2/L3 fabric

---

### **Controller Layers (Kubernetes)**

| Controller                          | Role                                                     |
| ----------------------------------- | -------------------------------------------------------- |
| **IPAM Controller (custom)**        | Watches IPPool/Vlan CRDs ↔ NetBox, maintains consistency |
| **DHCP Sync Controller**            | Reconciles DHCP scopes (NetBox ⇒ Kea)                    |
| **RouterOS Sync Controller**        | Syncs VLANs/IPs into network gear (RouterOS API)         |
| **PXE / Node Bootstrap Controller** | Renders PXE/Talos configs using IPAM info                |

These orchestrate the ecosystem in *GitOps style*.

---

# 🧠 How This Compares to CNCF Expectations

**CNCF IPAM status:**
There is **no graduated or sandbox CNCF IPAM project** that:
✔ Manages bare-metal IP pools
✔ Covers DHCP scopes
✔ Integrates with external devices
✔ Operates as a fully Kubernetes controller

All existing fit one of these categories:

1. **K8s CNI IPAM** (overlay networking) — CNCF ecosystem
2. **Machine IPAM (CAPI)** — infrastructure lifecycle, limited
3. **External IPAM servers** — non-CNCF, OSS

---

## 🧠 Summary — Best Free/Open Path

If your goal is:

**“Kubernetes-native IPAM with DHCP + VLAN + RouterOS integration”**

Then the *only practical, free/open path* is:

```
       Git
        ↓
   Kubernetes (MGMT)
   ├─ IPAM CRDs
   ├─ Custom Controllers
   │    ├─ NetBox Sync
   │    ├─ DHCP (Kea) Sync
   │    └─ RouterOS Sync
   └─ NetBox (source of truth)
        ↓
 DHCP (Kea) / RouterOS (relay / VLAN / routing)
        ↓
   PXE/Talos Boot
        ↓
   Bare-metal clusters
```

---

## 🔜 If you want the next step

I can provide:

✅ A **reference architecture** for this stack
✅ CRD designs (IPPool, VLAN, Lease, Reservation)
✅ Controller reconcile loop templates
✅ Helm charts for NetBox + Kea
✅ RouterOS API adapter code patterns

Just tell me whether you want **Go + controller-sdk** or **Rust + kube-rs** for the controllers.


---

is this it? https://github.com/netbox-community/netbox

---

Yes — the link you shared **is exactly NetBox**:
👉 **[https://github.com/netbox-community/netbox](https://github.com/netbox-community/netbox)** is the official open-source **IP Address Management (IPAM) and Data Center Infrastructure Management (DCIM)** project. ([GitHub][1])

### What NetBox *is*

✔ An open-source application for **IPAM** — managing IP address space, prefixes, VRFs, VLANs, and individual IPs
✔ A **DCIM** tool — tracking devices, racks, cables, circuits, and other infrastructure inventory
✔ A **source of truth** for network intent
✔ Rich REST API + extensible model (custom fields, plugins, webhooks)
✔ Web UI + programmable interface for automation and integrations ([GitHub][1])

NetBox was originally developed by engineers at DigitalOcean and is widely used by network teams as the **central database for IP addressing, VLAN planning, and network inventories** — not just a simple spreadsheet replacement. ([netboxlabs.com][2])

### What NetBox does *not* do by itself

❌ It **does not run DHCP**, CNI, BGP, or PXE services on the network
❌ It **does not automatically push configs** to devices like RouterOS
❌ It is **not a Kubernetes operator by default** — it’s an application you can run anywhere (including inside a Kubernetes pod) ([GitHub][3])

Instead, it **exposes APIs** that automation systems (Ansible, Terraform, or custom controllers) can consume to orchestrate real infrastructure.

---

### Typical uses for NetBox

📌 Authoritative IPAM (prefixes / IPs)
📌 VLAN and VRF inventory
📌 Device and rack documentation
📌 REST API for automation
📌 Integration source of truth for controllers that then push config to network gear or services ([netboxlabs.com][2])

So yes — that GitHub repo **is the open-source NetBox** you’d use as the backbone for IPAM in your architecture.

If you want, I can outline the **specific data models (prefix, IP address, VRF, VLAN)** you’d leverage and how a Kubernetes controller could sync that into RouterOS or DHCP servers.

[1]: https://github.com/netbox-community/netbox?utm_source=chatgpt.com "GitHub - netbox-community/netbox: The premier source of ..."
[2]: https://netboxlabs.com/blog/netbox-ipam/?utm_source=chatgpt.com "An In-Depth Guide to NetBox for IPAM"
[3]: https://github.com/netbox-community/netbox/discussions/13691?utm_source=chatgpt.com "How does Netbox as a DCIM & IPAM tool fit alongside DDI?"


---

We have identified:
- Netbox for IPAM

I need:
- PXE controller options
- DHCP controller or use Mikrotik / RouterOS device for this

Plans on how to bridge the above:
- Networking controller (Do we build this)


---

Good — you’ve locked in the **hard part** (IPAM source of truth). Everything else now fans out cleanly from that decision.

Below is a **clear, realistic menu of options** for each remaining layer, followed by **recommended bridging plans** (including where you *must* build something yourself).

---

# ✅ Fixed

## **NetBox**

Role:

* Authoritative IPAM
* VLANs / VRFs
* Device inventory
* MAC ↔ IP ↔ hostname truth

NetBox does **not**:

* Serve DHCP
* Do PXE
* Push config to routers

So everything else projects *from* NetBox.

---

# 1️⃣ PXE controller options (bare metal boot)

PXE is *always* a control-plane concern. There is no “magic Kubernetes-native PXE”.

## Option A — **Pixiecore / Pixieboot (recommended)**

**Best fit for Talos + Kubernetes control planes**

### What it gives you

* Stateless PXE / iPXE over HTTP
* No local TFTP state
* Simple API-driven boot
* Widely used in Talos/Sidero ecosystems

### How it fits

```
NetBox → PXE Controller → Pixiecore → Nodes
```

### Kubernetes story

* Run Pixiecore as a Deployment
* Expose HTTP + DHCP proxy port
* Your **PXE controller**:

    * Queries NetBox
    * Renders Talos kernel/initramfs args
    * Decides *what* a node should boot

✅ Clean
✅ Stateless
✅ Designed for ephemeral boot

---

## Option B — **Matchbox (older, heavier)**

* CoreOS-era PXE system
* GRPC + profiles
* Stateful config store

⚠️ Still works, but:

* More operational drag
* Less Talos-aligned
* Harder GitOps story

Use only if already deployed.

---

## Option C — **dnsmasq + iPXE (DIY)**

* Full control
* Maximum pain

Only do this if you enjoy writing glue forever.

---

### 🔥 PXE recommendation

> **Pixiecore + a small PXE intent controller**
> This controller is **yours** — and it should be.

---

# 2️⃣ DHCP options

You have **two sane paths**. Everything else is a trap.

---

## Option A — **RouterOS (MikroTik) as DHCP server**

Entity: **RouterOS**

### Pros

* Already on the wire
* VLAN-aware
* Fast
* Reliable

### Cons

* Not declarative
* No transactions
* Needs external reconciliation
* Humans *will* click things unless blocked

### Required bridge

You **must** build a controller that:

* Reads NetBox
* Writes DHCP pools / reservations into RouterOS via API
* Periodically reconciles drift

```
NetBox → DHCP Sync Controller → RouterOS
```

✅ Acceptable if RouterOS is your L3 boundary
❌ Dangerous without reconciliation + policy locks

---

## Option B — **ISC Kea DHCP (recommended for control-plane purity)**

Entity: **ISC Kea**

### Pros

* API-driven
* Database-backed
* Fully automatable
* GitOps-friendly
* Kubernetes-friendly

### Cons

* Another moving part
* RouterOS becomes relay only

### Architecture

```
NetBox → DHCP Controller → Kea DHCP
RouterOS → DHCP Relay → Kea
```

This is the **cleanest separation of concerns**.

---

### 🔥 DHCP recommendation

> **ISC Kea as DHCP + RouterOS as relay**
> Especially if you want long-term sanity.

---

# 3️⃣ Bridging layer — the Networking Controller (yes, you build this)

There is **no off-the-shelf controller** that:

* Speaks NetBox
* Speaks RouterOS
* Understands PXE intent
* Integrates with Talos / CAPI

This is your **glue plane**.

---

## 🧠 What the Networking Controller does

Think of it as **ExternalDNS, but for L2/L3 + PXE**.

### Inputs

* NetBox (IPAM, VLANs, devices)
* Git (cluster intent)
* CAPI / TalosMachine CRs

### Outputs

* RouterOS:

    * VLANs
    * DHCP pools OR relay config
    * Static reservations
* PXE system:

    * MAC → boot profile
    * Talos config selection
* DHCP system:

    * Subnets
    * Reservations
    * Options (next-server, bootfile)

---

## 🧩 Suggested internal split (do NOT monolith this)

| Sub-controller    | Responsibility          |
| ----------------- | ----------------------- |
| **IPAM Sync**     | NetBox ↔ internal CRDs  |
| **DHCP Sync**     | NetBox → Kea / RouterOS |
| **PXE Intent**    | MAC → boot profile      |
| **RouterOS Sync** | VLANs, helpers, pools   |
| **Safety Policy** | Prevent self-management |

Each can be small, testable, and replaceable.

---

## 🔐 Hard rules you must enforce

These are non-negotiable:

1. **RouterOS is never source of truth**
2. **Humans do not configure DHCP manually**
3. **PXE intent is explicit (no “unknown boot”)**
4. **Management cluster never PXE-boots itself**
5. **All writes are idempotent + reconciled**

---

# 4️⃣ Putting it all together (final recommended stack)

```
Git
 ↓
Management Kubernetes Cluster
 ├─ NetBox (IPAM / VLAN / Inventory)
 ├─ GitOps (Flux / Argo)
 ├─ PXE Intent Controller  ──┐
 ├─ DHCP Sync Controller   ──┼─→ RouterOS (VLANs / Relay)
 ├─ IPAM Sync Controller  ──┘
 ├─ Kea DHCP (optional)
 └─ CAPI + Talos providers

RouterOS
 ├─ VLAN gateway
 ├─ DHCP relay
 └─ L3 routing

Bare Metal
 └─ PXE → Talos → Cluster
```

---

# 🔚 Bottom line

* **NetBox**: correct choice
* **PXE**: Pixiecore + custom controller
* **DHCP**:

    * *Short term*: RouterOS + controller
    * *Correct long term*: Kea DHCP
* **Networking controller**:
  👉 **Yes, you build this — and you keep it small and boring**

If you want next, I can:

* Design **CRDs** for PXE/DHCP/IPAM
* Sketch a **RouterOS reconcile loop**
* Provide a **NetBox → Kea schema mapping**
* Or help you decide **Go vs Rust** for the controllers

You’re designing a real fleet control plane — this is the right direction.


---


