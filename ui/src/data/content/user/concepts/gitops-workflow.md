# GitOps Workflow

DCops follows a GitOps workflow where all infrastructure is declared in Git and automatically reconciled.

## The Workflow

```
Git Repository (YAML)
    ↓
Kubernetes API
    ↓
DCops Controllers
    ↓
NetBox API
    ↓
Physical Infrastructure
```

## Benefits

- **Version Control** - All changes tracked in Git
- **Review Process** - Pull requests for all infrastructure changes
- **Audit Trail** - Complete history of all changes
- **Rollback** - Revert changes by reverting Git commits

## How It Works

1. Define infrastructure in YAML files
2. Commit to Git
3. Controllers automatically reconcile
4. NetBox is updated to match Git state

