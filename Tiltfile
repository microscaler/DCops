# ====================
# Future Controllers
# ====================
# Additional controllers will be added here as they're implemented

# ====================
# DCops Documentation Site
# ====================

# Build docs Docker image
docker_build(
    'dcops-docs',
    '.',
    dockerfile='./dockerfiles/Dockerfile.dcops-docs',
    platform='linux/amd64',
    only=[
        './ui-docs',
        './dockerfiles/Dockerfile.dcops-docs',
        './dockerfiles/nginx.dcops-docs.conf',
    ],
    ignore=[
        'ui-docs/node_modules',
        'ui-docs/dist',
        'ui-docs/.git',
    ],
    live_update=[
        sync('./ui-docs', '/app/ui-docs'),
        run('cd /app && yarn build', trigger=['./ui-docs']),
    ],
)

k8s_yaml(kustomize('%s/config/dcops-docs' % DCops_DIR))

k8s_resource(
    'dcops-docs',
    port_forwards='8801:80',
    labels=['docs'],
)

# ====================
# DCops Dashboard SPA
# ====================

# Build dashboard Docker image
docker_build(
    'dcops-dashboard',
    '.',
    dockerfile='./dockerfiles/Dockerfile.dcops-dashboard',
    platform='linux/amd64',
    only=[
        './ui-dashboard',
        './dockerfiles/Dockerfile.dcops-dashboard',
        './dockerfiles/nginx.dcops-dashboard.conf',
    ],
    ignore=[
        'ui-dashboard/node_modules',
        'ui-dashboard/dist',
        'ui-dashboard/.git',
    ],
    live_update=[
        sync('./ui-dashboard', '/app/ui-dashboard'),
        run('cd /app && yarn build', trigger=['./ui-dashboard']),
    ],
)

# kubectl proxy for local dev (Dashboard SPA needs K8s API access)
local_resource(
    'kubectl-proxy',
    cmd='kubectl proxy --port=8001 --address=0.0.0.0',
    labels=['infrastructure'],
)

k8s_yaml(kustomize('%s/config/dcops-dashboard' % DCops_DIR))

k8s_resource(
    'dcops-dashboard',
    port_forwards='8802:80',
    labels=['docs'],
    resource_deps=['kubectl-proxy'],
)
