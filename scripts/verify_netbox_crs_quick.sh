#!/bin/bash
# Quick verification commands for NetBox CRs
# 
# This script provides quick bash commands for manual verification.
# For comprehensive automated verification, use scripts/verify_netbox_crs.py

set -euo pipefail

echo "============================================================"
echo "Quick NetBox CR Verification"
echo "============================================================"

echo ""
echo "1. Verifying all NetBox CRDs exist..."
kubectl get crd | grep netbox || echo "⚠️  No NetBox CRDs found"

echo ""
echo "2. Verifying all CRs have status..."
for crd in netboxprefixes netboxtenants netboxsites netboxroles netboxtags netboxaggregates netboxvlans; do
  echo "Checking $crd..."
  kubectl get $crd -A -o jsonpath='{range .items[*]}{.metadata.namespace}/{.metadata.name}: {.status.netboxId}{"\n"}{end}' || echo "  No CRs found"
done

echo ""
echo "3. Verifying resources in NetBox database..."
POSTGRES_POD=$(kubectl get pod -n netbox -l app=postgres -o jsonpath='{.items[0].metadata.name}' || echo "")
if [ -z "$POSTGRES_POD" ]; then
  echo "⚠️  Could not find PostgreSQL pod"
else
  echo "Using PostgreSQL pod: $POSTGRES_POD"
  kubectl exec -n netbox "$POSTGRES_POD" -- psql -U netbox -d netbox -c "SELECT id, name FROM tenancy_tenant LIMIT 5;" || echo "  Could not query database"
fi

echo ""
echo "============================================================"
echo "For comprehensive verification, run:"
echo "  python3 scripts/verify_netbox_crs.py --all"
echo "============================================================"

