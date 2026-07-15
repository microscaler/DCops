#!/usr/bin/env bash
# Provision the NetBox role + database on the shared-k8s Postgres (data namespace).
#
# Modeled on hauliage/scripts/setup-db.sh. Idempotent; run by Tilt (netbox-db-init)
# before NetBox migrates. NetBox creates its own SCHEMA via `manage.py migrate`, but
# the DATABASE and login ROLE must exist first.
#
# NETBOX_DB_PASSWORD must match config/netbox/netbox-db-credentials.yaml (DB_PASSWORD).
set -euo pipefail

NS=data
DEPLOY=postgres-primary
WAIT_TIMEOUT="${NETBOX_DB_INIT_TIMEOUT:-600s}"
NETBOX_DB_PASSWORD="${NETBOX_DB_PASSWORD:-dcops_dev_password_change_in_prod}"

sql_escape() { printf '%s' "$1" | sed "s/'/''/g"; }
PW_SQL=$(sql_escape "${NETBOX_DB_PASSWORD}")

echo "⏳ Waiting for ${DEPLOY} rollout in ${NS} (${WAIT_TIMEOUT})..."
kubectl rollout status "deployment/${DEPLOY}" -n "${NS}" --timeout="${WAIT_TIMEOUT}"
kubectl wait --for=condition=ready pod -l 'app in (postgres, postgres-primary)' \
  -n "${NS}" --timeout="${WAIT_TIMEOUT}" >/dev/null

echo "⏳ Creating role 'netbox' and database 'netbox' (if missing)..."
# Main container is named "postgres"; $POSTGRESQL_PASSWORD is the in-pod superuser password.
kubectl exec -i -n "${NS}" "deployment/${DEPLOY}" -c postgres -- \
  sh -c 'env PGPASSWORD="$POSTGRESQL_PASSWORD" psql -h 127.0.0.1 -p 5432 -U postgres -d postgres -v ON_ERROR_STOP=1' <<EOF
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'netbox') THEN
    EXECUTE format('CREATE ROLE netbox LOGIN PASSWORD %L', '${PW_SQL}');
  ELSE
    EXECUTE format('ALTER ROLE netbox PASSWORD %L', '${PW_SQL}');
  END IF;
END \$\$;

SELECT 'CREATE DATABASE netbox OWNER netbox'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'netbox')\gexec

\c netbox
GRANT ALL PRIVILEGES ON DATABASE netbox TO netbox;
GRANT ALL ON SCHEMA public TO netbox;
ALTER SCHEMA public OWNER TO netbox;
EOF

echo "✅ NetBox role + database ready on the shared Postgres."
