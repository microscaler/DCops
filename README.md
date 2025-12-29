# DCops: GitOps Infrastructure Control for On-Premise and Datacenter Infrastructure

> **Stop managing infrastructure with spreadsheets. Start managing it with Git.**

DCops brings Kubernetes-native, GitOps-driven infrastructure management to your on-premise datacenter, colocation facility, or rented cabinet. Declare your infrastructure intent in Git, and let Kubernetes controllers automatically reconcile it to your physical hardware.

## The Problem You're Solving

If you're running infrastructure in a datacenter, colocation facility, or rented cabinet, you're likely managing:

- **IP address allocation** in spreadsheets or wikis
- **Network inventory** in disconnected tools or manual databases
- **PXE boot configurations** that require manual intervention
- **Device provisioning** that can't be safely automated
- **Infrastructure state** that drifts from your documentation

Every change requires manual coordination. Every rebuild risks human error. Every expansion means more spreadsheets to maintain.

**DCops eliminates this operational debt.**

## What DCops Does

DCops is a set of Kubernetes controllers that manage your physical infrastructure through a GitOps workflow. Think of it as "Infrastructure as Code" for your datacenter.

### Core Capabilities

**🎯 Deterministic IP Allocation**
- Declare IP pools and claims in Git
- Automatic allocation from NetBox IPAM
- No more spreadsheets, no more manual tracking
- Safe, idempotent operations that prevent conflicts

**🔧 Infrastructure Inventory Management**
- Manage sites, regions, devices, and network topology in Git
- Automatic reconciliation with NetBox
- Drift detection and correction
- Multi-tenant support for shared infrastructure

**🚀 Controlled PXE Boot**
- Define boot profiles and intents declaratively
- Prevent infinite boot loops and accidental reinstallations
- Safe cluster rebuilds without manual intervention

**📊 Single Source of Truth**
- Git is your source of truth
- NetBox is your authoritative database
- Controllers ensure they stay in sync
- Full audit trail and change history

## How It Works

DCops follows a simple, powerful pattern:

```
Your Git Repository (YAML)
    ↓
Kubernetes Controllers
    ↓
NetBox (IPAM & Inventory)
    ↓
Your Physical Infrastructure
```

**1. Declare Intent in Git**
Define your infrastructure as Kubernetes Custom Resources (CRDs) in YAML files. Commit to Git like any other code.

**2. Controllers Reconcile Automatically**
Kubernetes controllers watch your Git repository and continuously reconcile your declared intent with the actual state in NetBox.

**3. NetBox Manages State**
NetBox serves as your authoritative IPAM and inventory database. Controllers read from and write to NetBox, but you never configure NetBox manually.

**4. Infrastructure Stays in Sync**
If someone manually changes NetBox, controllers detect the drift and either correct it or alert you. Your Git repository always reflects reality.

## Why You Should Use DCops

### For SREs

**Eliminate Operational Toil**
- No more IP address spreadsheets
- No more manual NetBox configuration
- No more "who changed what" investigations
- Automated drift detection and correction

**Safe, Repeatable Operations**
- All changes go through Git (PRs, reviews, approvals)
- Idempotent operations prevent conflicts
- Rollback by reverting Git commits
- Full audit trail of every change

**Kubernetes-Native**
- Uses the same patterns you already know
- Integrates with your existing GitOps workflows
- Runs in your management cluster
- Standard Kubernetes RBAC and policies

### For Engineering Managers

**Reduce Infrastructure Risk**
- Eliminate human error in IP allocation
- Prevent configuration drift
- Safe, tested changes through Git workflows
- Faster incident recovery

**Scale Your Operations**
- Onboard new team members faster
- Support multiple tenants and environments
- Automate repetitive tasks
- Reduce time-to-provision for new infrastructure

**Lower Operational Costs**
- Less time spent on manual infrastructure management
- Fewer incidents from configuration errors
- Better resource utilization tracking
- Clear ROI on infrastructure investments

## Perfect For

**On-Premise Datacenters**
Manage your own infrastructure with the same GitOps workflows you use for cloud resources.

**Colocation Facilities**
Coordinate infrastructure across multiple tenants and environments with clear separation and audit trails.

**Rented Cabinets**
Start small with a single cabinet, scale as you grow. All infrastructure managed through Git.

**Hybrid Environments**
Bridge the gap between cloud-native workflows and physical infrastructure management.

## Technology Foundation

DCops is built on proven, production-ready technologies:

- **Kubernetes** - Industry-standard orchestration platform
- **NetBox** - The de facto standard for IPAM and datacenter inventory
- **Rust** - High-performance, memory-safe controllers
- **GitOps** - Declarative, version-controlled infrastructure management

## Getting Started

DCops integrates with your existing Kubernetes cluster and NetBox instance. No special hardware required—just a management Kubernetes cluster and NetBox.

## Learn More

- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)

---

**Ready to eliminate infrastructure spreadsheets?** Start managing your datacenter infrastructure the same way you manage your applications: with Git, Kubernetes, and declarative intent.
