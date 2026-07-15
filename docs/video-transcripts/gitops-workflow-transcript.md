# Video Transcript: GitOps Workflow with DCops

**Duration:** ~8 minutes  
**Target Audience:** SREs, DevOps Engineers, Platform Engineers  
**Prerequisites:** Basic Git and Kubernetes knowledge

---

## Introduction (0:00 - 0:30)

[Screen: DCops logo]

**Narrator:** "Welcome to GitOps Workflow with DCops. In this video, we'll explore how DCops brings GitOps principles to physical infrastructure management, giving you version control, review processes, and automatic reconciliation."

[Screen: GitOps concept]

**Narrator:** "GitOps isn't just for cloud resources anymore. DCops extends GitOps to your datacenter infrastructure - IP addresses, sites, devices, and network topology. All managed through Git, just like your application code."

---

## What You'll Learn (0:30 - 1:00)

[Screen: Learning objectives]

**Narrator:** "In this walkthrough, we'll:
1. Understand the GitOps workflow
2. See how changes flow from Git to NetBox
3. Learn about drift detection
4. See how to rollback changes
5. Understand the benefits

Let's dive in!"

---

## The GitOps Workflow (1:00 - 2:30)

[Screen: Workflow diagram]

**Narrator:** "Here's how DCops' GitOps workflow works:

You start by defining infrastructure in YAML files - sites, devices, IP pools, everything. You commit these to Git, just like any other code.

Your GitOps tool - FluxCD, ArgoCD, or similar - syncs these YAML files to your Kubernetes cluster.

The DCops controllers watch for changes and reconcile them with NetBox.

NetBox serves as the authoritative database, and your physical infrastructure is managed through NetBox.

The key is that Git is always the source of truth. If something changes in NetBox, the controller detects it and corrects it."

[Screen: Animated workflow]

**Narrator:** "Let me show you this in action..."

---

## Example: Adding a Site (2:30 - 4:30)

[Screen: Git repository]

**Narrator:** "Let's say I want to add a new datacenter site. I'll create a YAML file in my Git repository..."

[Screen: Creating YAML file]

```yaml
apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxSite
metadata:
  name: datacenter-2
  namespace: default
spec:
  name: "Data Center 2"
  tenant:
    apiGroup: "dcops.microscaler.io"
    kind: "NetBoxTenant"
    name: "datacenter-tenant"
```

**Narrator:** "I commit this to Git..."

[Screen: Git commit]

```bash
git add config/sites/datacenter-2.yaml
git commit -m "feat: add datacenter-2 site"
git push
```

**Narrator:** "Now my GitOps tool - in this case, FluxCD - detects the change and syncs it to Kubernetes..."

[Screen: Kubernetes API]

**Narrator:** "The CRD appears in the Kubernetes cluster. The DCops controller detects the new resource..."

[Screen: Controller logs]

**Narrator:** "The controller reconciles it - checking if the site exists in NetBox, and if not, creating it..."

[Screen: NetBox UI]

**Narrator:** "And there it is! The site was automatically created in NetBox. All from a Git commit - no manual NetBox configuration needed."

---

## Drift Detection (4:30 - 6:00)

[Screen: Drift scenario]

**Narrator:** "One of the most powerful features of DCops is drift detection. Let me show you what happens when someone manually changes something in NetBox..."

[Screen: NetBox UI - manual change]

**Narrator:** "Let's say someone goes into the NetBox UI and changes the site name from 'Data Center 1' to 'DC1 - Updated'..."

[Screen: Controller reconciliation]

**Narrator:** "The controller periodically reconciles all resources. When it checks this site, it notices the name in NetBox doesn't match what's in Git..."

[Screen: Controller correcting drift]

**Narrator:** "The controller automatically updates NetBox to match the Git state. The site name is changed back to 'Data Center 1'."

[Screen: Kubernetes event]

**Narrator:** "A Kubernetes event is emitted, so you can see that drift was detected and corrected. This ensures your Git repository always reflects reality."

[Screen: Deletion scenario]

**Narrator:** "Even if someone deletes a resource in NetBox, the controller will detect it's missing and recreate it from Git. Your infrastructure stays in sync automatically."

---

## Rollback (6:00 - 7:00)

[Screen: Rollback scenario]

**Narrator:** "Another powerful feature is easy rollback. Let's say I made a change that caused problems..."

[Screen: Git history]

**Narrator:** "I can simply revert the commit in Git..."

```bash
git revert HEAD
git push
```

[Screen: Automatic rollback]

**Narrator:** "The GitOps tool syncs the reverted state. The controller reconciles, and NetBox is automatically updated to the previous state. No manual cleanup needed - it's all automatic."

---

## Benefits (7:00 - 8:00)

[Screen: Benefits slide]

**Narrator:** "Let's talk about the benefits of this GitOps approach..."

### Version Control

**Narrator:** "Every change is tracked in Git. You have complete history - who changed what, when, and why. This is invaluable for auditing and troubleshooting."

### Review Process

**Narrator:** "All changes go through pull requests. You get code review, discussion, and approval before changes are applied. This prevents mistakes and ensures quality."

### Audit Trail

**Narrator:** "Complete audit trail - Git commits, Kubernetes events, NetBox change logs. Everything is tracked and searchable."

### Easy Rollback

**Narrator:** "Rollback is as simple as reverting a Git commit. No manual cleanup, no risk of missing something - it's all automatic."

### No Spreadsheets

**Narrator:** "And of course, no more spreadsheets! Everything is in Git, version controlled, and automatically synced."

---

## Summary (8:00 - 8:30)

[Screen: Summary slide]

**Narrator:** "In this video, we learned:
- How the GitOps workflow works
- How changes flow from Git to NetBox
- How drift detection keeps everything in sync
- How easy rollback is
- The benefits of GitOps for infrastructure

DCops brings the same GitOps workflows you use for applications to your physical infrastructure. No more spreadsheets, no more manual tracking - just Git, Kubernetes, and automatic reconciliation."

[Screen: End screen]

**Narrator:** "Thanks for watching! Check out the documentation for more examples and advanced topics."

---

## Production Notes

### Visual Elements:
1. Git repository view (animated commits)
2. GitOps tool dashboard (FluxCD/ArgoCD)
3. Kubernetes dashboard
4. Controller logs (animated)
5. NetBox UI (before/after changes)
6. Workflow diagram animations

### Key Moments:
- First Git commit (show the workflow)
- Drift detection (exciting moment)
- Automatic correction (show the power)
- Rollback (show how easy it is)

### Voiceover Tips:
- Use enthusiasm for powerful features
- Emphasize "automatic" and "no manual work"
- Pause before benefits
- Speak clearly when showing Git commands

