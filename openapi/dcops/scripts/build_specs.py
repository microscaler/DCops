#!/usr/bin/env python3
"""Build the DCops OpenAPI suite (openapi/dcops/<service>/openapi.yaml + openapi_bff.yaml).

Consumer-driven: every operation exists because a screen in docs/03_ui_design needs it.
CRD-backed schemas are extracted from config/crd/all-crds.yaml so the contracts never drift
from the resource definitions; bespoke operations (platform aggregates, certificates, imports)
are authored in this file. The emitted YAML is the reviewable artefact and is NEVER hand-edited
— this script is the only writer (same rule as PriceWhisperer's cut_trader_specs.py).

Usage: python3 openapi/dcops/scripts/build_specs.py [--crds config/crd/all-crds.yaml] [--out openapi/dcops]
"""
from __future__ import annotations
import argparse, copy, json, os, re, sys, textwrap
import yaml

# ----------------------------------------------------------------------------- CRD → schema
def to_31(s):
    """Convert a Kubernetes openAPIV3Schema fragment to OpenAPI 3.1 JSON Schema."""
    if isinstance(s, list):
        return [to_31(x) for x in s]
    if not isinstance(s, dict):
        return s
    s = dict(s)
    nullable = s.pop('nullable', False)
    for k in ('x-kubernetes-preserve-unknown-fields', 'x-kubernetes-int-or-string', 'x-kubernetes-list-type', 'x-kubernetes-list-map-keys', 'x-kubernetes-map-type'):
        s.pop(k, None)
    if s.get('format') in ('uint64', 'uint32', 'uint16', 'uint8'):
        s['format'] = 'int64' if s['format'] == 'uint64' else 'int32'
    if 'minimum' in s and isinstance(s['minimum'], float) and s['minimum'].is_integer() and s.get('type') == 'integer':
        s['minimum'] = int(s['minimum'])
    out = {k: to_31(v) for k, v in s.items()}
    if nullable and 'type' in out:
        t = out['type']
        out['type'] = [t, 'null'] if isinstance(t, str) else list(dict.fromkeys(list(t) + ['null']))
    return out

def load_crds(path):
    docs = [d for d in yaml.safe_load_all(open(path)) if d and d.get('kind') == 'CustomResourceDefinition']
    crds = {}
    for d in docs:
        names = d['spec']['names']; v = d['spec']['versions'][0]
        root = v['schema']['openAPIV3Schema']['properties']
        crds[names['kind']] = {
            'kind': names['kind'], 'plural': names['plural'], 'version': v['name'], 'group': d['spec']['group'],
            'spec': to_31(root['spec']), 'status': to_31(root.get('status', {'type': 'object', 'properties': {}})),
            'printer': v.get('additionalPrinterColumns') or [],
        }
    return crds

# ----------------------------------------------------------------------------- suite definition
GROUP = 'dcops.microscaler.io'
SERVICES = {
    # service: (title, description, [kinds])  — kinds are CRD Kind names
    'dcim':     ('Data centre inventory (NetBox DCIM)', ['NetBoxSite', 'NetBoxLocation', 'NetBoxRegion', 'NetBoxSiteGroup', 'NetBoxDevice', 'NetBoxDeviceType', 'NetBoxDeviceRole', 'NetBoxManufacturer', 'NetBoxPlatform', 'NetBoxInterface', 'NetBoxMACAddress']),
    'ipam':     ('IP address management (NetBox IPAM)', ['NetBoxPrefix', 'NetBoxIPAddress', 'NetBoxIPRange', 'NetBoxAggregate', 'NetBoxVLAN', 'NetBoxRIR', 'NetBoxVRF', 'NetBoxRouteTarget', 'NetBoxRole']),
    'tenancy':  ('Tenancy (NetBox tenants and groups)', ['NetBoxTenant', 'NetBoxTenantGroup']),
    'extras':   ('Extras (NetBox tags)', ['NetBoxTag']),
    'boot':     ('PXE boot control', ['BootProfile', 'BootIntent']),
    'ippools':  ('IP pools and claims', ['IPPool', 'IPClaim']),
}
# Screen-facing route segment per kind (matches the mockup sub-navigation)
SEGMENT = {
    'NetBoxSite': 'sites', 'NetBoxLocation': 'locations', 'NetBoxRegion': 'regions', 'NetBoxSiteGroup': 'site-groups',
    'NetBoxDevice': 'devices', 'NetBoxDeviceType': 'device-types', 'NetBoxDeviceRole': 'device-roles', 'NetBoxManufacturer': 'manufacturers',
    'NetBoxPlatform': 'platforms', 'NetBoxInterface': 'interfaces', 'NetBoxMACAddress': 'mac-addresses',
    'NetBoxPrefix': 'prefixes', 'NetBoxIPAddress': 'ip-addresses', 'NetBoxIPRange': 'ip-ranges', 'NetBoxAggregate': 'aggregates',
    'NetBoxVLAN': 'vlans', 'NetBoxRIR': 'rirs', 'NetBoxVRF': 'vrfs', 'NetBoxRouteTarget': 'route-targets', 'NetBoxRole': 'roles',
    'NetBoxTenant': 'tenants', 'NetBoxTenantGroup': 'tenant-groups', 'NetBoxTag': 'tags',
    'BootProfile': 'boot-profiles', 'BootIntent': 'boot-intents', 'IPPool': 'pools', 'IPClaim': 'claims',
}
# Extra list filters per kind (query params) beyond the common ones — consumer-driven from the mockups
KIND_FILTERS = {
    'NetBoxPrefix': ['vrf', 'site', 'status'], 'NetBoxDevice': ['site', 'deviceRole', 'status'], 'NetBoxInterface': ['device'],
    'NetBoxIPAddress': ['vrf', 'status'], 'NetBoxVLAN': ['site'], 'BootIntent': ['lifecycle', 'profile'], 'IPClaim': ['pool'], 'IPPool': ['vrf'],
}
# Placeholder CRDs for kinds the UI already lists but the repo has not defined yet (design gap, recorded in README)
PLACEHOLDER_CRDS = {
    'IPPool': {
        'kind': 'IPPool', 'plural': 'ippools', 'version': 'v1alpha1', 'group': GROUP, 'printer': [],
        'spec': {'type': 'object', 'required': ['prefix'], 'properties': {
            'prefix': {'type': 'string', 'description': 'CIDR the pool allocates from'},
            'vrf': {'type': ['string', 'null'], 'description': 'NetBoxVRF name'},
            'gateway': {'type': ['string', 'null']},
            'reserved': {'type': 'array', 'items': {'type': 'string'}, 'description': 'Addresses never allocated'},
            'description': {'type': ['string', 'null']}}},
        'status': {'type': 'object', 'properties': {
            'allocated': {'type': 'integer'}, 'capacity': {'type': 'integer'},
            'state': {'type': 'string', 'enum': ['Pending', 'Created', 'Updated', 'Failed']},
            'error': {'type': ['string', 'null']}, 'lastReconciled': {'type': ['string', 'null'], 'format': 'date-time'}}},
    },
    'IPClaim': {
        'kind': 'IPClaim', 'plural': 'ipclaims', 'version': 'v1alpha1', 'group': GROUP, 'printer': [],
        'spec': {'type': 'object', 'required': ['pool'], 'properties': {
            'pool': {'type': 'string', 'description': 'IPPool name'},
            'requestedBy': {'type': ['string', 'null'], 'description': 'Kind/name of the requester, e.g. NetBoxDevice/cp-01'},
            'macAddress': {'type': ['string', 'null']},
            'hostname': {'type': ['string', 'null']}}},
        'status': {'type': 'object', 'properties': {
            'address': {'type': ['string', 'null']}, 'netboxId': {'type': ['integer', 'null'], 'format': 'int64'},
            'state': {'type': 'string', 'enum': ['Pending', 'Created', 'Updated', 'Failed']},
            'error': {'type': ['string', 'null']}, 'lastReconciled': {'type': ['string', 'null'], 'format': 'date-time'}}},
    },
}

# ----------------------------------------------------------------------------- shared components
STATE = {'type': 'string', 'enum': ['Pending', 'Created', 'Updated', 'Failed'], 'description': 'Reconciliation state written by the controller'}
COMMON = {
    'ErrorEnvelope': {'type': 'object', 'required': ['code', 'message', 'requestId'], 'properties': {
        'code': {'type': 'string', 'description': 'Stable machine-readable error code'},
        'message': {'type': 'string', 'description': 'Human-readable, safe to show to an Operations manager'},
        'fields': {'type': 'array', 'items': {'type': 'object', 'required': ['name', 'reason'], 'properties': {'name': {'type': 'string'}, 'reason': {'type': 'string'}}}},
        'requestId': {'type': 'string'}},
        'examples': [{'code': 'not_found', 'message': 'No NetBoxPrefix named control-plane-prefix in namespace default', 'fields': [], 'requestId': '01J7QZ3M8V2K'}]},
    'ResourceMeta': {'type': 'object', 'required': ['name', 'namespace', 'uid', 'creationTimestamp'], 'properties': {
        'name': {'type': 'string'}, 'namespace': {'type': 'string'}, 'uid': {'type': 'string'},
        'creationTimestamp': {'type': 'string', 'format': 'date-time'}, 'resourceVersion': {'type': 'string'},
        'labels': {'type': 'object', 'additionalProperties': {'type': 'string'}},
        'annotations': {'type': 'object', 'additionalProperties': {'type': 'string'}}}},
    'Condition': {'type': 'object', 'required': ['type', 'status'], 'properties': {
        'type': {'type': 'string'}, 'status': {'type': 'string', 'enum': ['True', 'False', 'Unknown']},
        'reason': {'type': ['string', 'null']}, 'message': {'type': ['string', 'null']},
        'lastTransitionTime': {'type': ['string', 'null'], 'format': 'date-time'}, 'observedGeneration': {'type': ['integer', 'null']}}},
    'Event': {'type': 'object', 'required': ['type', 'reason', 'message', 'lastTimestamp'], 'properties': {
        'type': {'type': 'string', 'enum': ['Normal', 'Warning']}, 'reason': {'type': 'string'}, 'message': {'type': 'string'},
        'count': {'type': 'integer'}, 'firstTimestamp': {'type': ['string', 'null'], 'format': 'date-time'},
        'lastTimestamp': {'type': 'string', 'format': 'date-time'}, 'source': {'type': ['string', 'null']}}},
    'PageInfo': {'type': 'object', 'required': ['total'], 'properties': {
        'total': {'type': 'integer', 'description': 'Total matching items'},
        'nextCursor': {'type': ['string', 'null'], 'description': 'Opaque cursor for the next page; null on the last page'}}},
    'KindCount': {'type': 'object', 'required': ['kind', 'plural', 'segment', 'count'], 'properties': {
        'kind': {'type': 'string'}, 'plural': {'type': 'string'}, 'segment': {'type': 'string', 'description': 'Route segment under this service'},
        'count': {'type': 'integer'}, 'byState': {'type': 'object', 'additionalProperties': {'type': 'integer'}}}},
    'State': STATE,
}
COMMON_PARAMS = {
    'namespace': {'name': 'namespace', 'in': 'query', 'required': False, 'description': 'Restrict to one namespace; absent = all namespaces the caller may read', 'schema': {'type': 'string'}},
    'state': {'name': 'state', 'in': 'query', 'required': False, 'description': 'Filter by reconciliation state', 'schema': STATE},
    'search': {'name': 'search', 'in': 'query', 'required': False, 'description': 'Case-insensitive match on name and the kind\'s primary field', 'schema': {'type': 'string'}},
    'limit': {'name': 'limit', 'in': 'query', 'required': False, 'schema': {'type': 'integer', 'minimum': 1, 'maximum': 500, 'default': 50}},
    'cursor': {'name': 'cursor', 'in': 'query', 'required': False, 'description': 'Cursor from a previous page', 'schema': {'type': 'string'}},
    'sort': {'name': 'sort', 'in': 'query', 'required': False, 'description': 'Field name, prefix with - for descending', 'schema': {'type': 'string', 'default': 'name'}},
    'pathNamespace': {'name': 'namespace', 'in': 'path', 'required': True, 'schema': {'type': 'string'}},
    'pathName': {'name': 'name', 'in': 'path', 'required': True, 'schema': {'type': 'string'}},
}
SECURITY = {'BearerAuth': {'type': 'http', 'scheme': 'bearer', 'bearerFormat': 'JWT', 'description': 'OIDC access token (Microsoft Entra ID or Sesame-IDAM); validated against the configured JWKS'}}

def resp(desc, schema, example=None):
    r = {'description': desc, 'content': {'application/json': {'schema': schema}}}
    if example is not None: r['content']['application/json']['example'] = example
    return r
def err(code, desc, token=None):
    if token is None and ':' in desc and ' ' not in desc.split(':', 1)[0]:
        token, desc = desc.split(':', 1); desc = desc.strip()
    ex = {'code': token or {'401': 'unauthenticated', '403': 'forbidden', '404': 'not_found', '409': 'conflict', '400': 'invalid_request', '410': 'gone', '413': 'payload_too_large', '415': 'unsupported_media_type', '422': 'unprocessable', '503': 'unavailable', '500': 'internal'}.get(code, 'error'), 'message': desc, 'requestId': 'req-01J9Z3Q8K7'}
    return {code: {'description': desc, 'content': {'application/json': {'schema': {'$ref': '#/components/schemas/ErrorEnvelope'}, 'example': ex}}}}
ERR_COMMON = {**err('401', 'Missing or invalid bearer token'), **err('403', 'Token lacks the required role'), **err('500', 'Unexpected error')}
def ref(n): return {'$ref': f'#/components/schemas/{n}'}

# ----------------------------------------------------------------------------- example synthesis
def example_for(schema, name='', depth=0):
    """Deterministic example value for a JSON schema (used where a mockup value is not curated)."""
    if '$ref' in schema: return {}
    t = schema.get('type'); tl = t if isinstance(t, list) else [t]
    if 'enum' in schema: return schema['enum'][0]
    if 'default' in schema: return schema['default']
    if 'string' in tl:
        f = schema.get('format')
        if f == 'date-time': return '2026-09-08T08:41:12Z'
        if f == 'date': return '2026-09-08'
        n = name.lower()
        if 'name' == n: return 'example'
        if n.endswith('url'): return 'http://netbox.netbox/api/dcim/sites/42/'
        if 'mac' in n: return 'dc:a6:32:1f:04:9a'
        if 'prefix' in n or 'cidr' in n: return '10.10.0.0/24'
        if 'address' in n or n.endswith('ip4'): return '10.10.0.11'
        if 'slug' in n: return 'example'
        if 'description' in n or 'comments' in n: return 'Example description'
        if 'error' in n: return None if 'null' in tl else ''
        return 'example'
    if 'integer' in tl: return 42 if 'id' in name.lower() else 1
    if 'number' in tl: return 1.0
    if 'boolean' in tl: return False
    if 'array' in tl: return [example_for(schema.get('items', {}), name, depth + 1)] if depth < 3 else []
    if 'object' in tl or 'properties' in schema:
        return {k: example_for(v, k, depth + 1) for k, v in schema.get('properties', {}).items() if depth < 4}
    return None

def scalar_props(schema):
    """Top-level scalar properties (for list rows)."""
    out = {}
    for k, v in schema.get('properties', {}).items():
        t = v.get('type'); tl = t if isinstance(t, list) else [t]
        if any(x in tl for x in ('string', 'integer', 'number', 'boolean')): out[k] = v
        elif 'object' in tl and 'name' in v.get('properties', {}):  # reference objects → the referenced name
            out[k] = {'type': ['string', 'null'], 'description': (v.get('description') or '').split('\n')[0] + ' (name of the referenced resource)'}
    return out

# ----------------------------------------------------------------------------- CRD-backed service
def crd_service(service, title, kinds, crds):
    comps = copy.deepcopy(COMMON); paths = {}; tags = []
    kind_counts = []
    for kind in kinds:
        c = crds.get(kind) or PLACEHOLDER_CRDS.get(kind)
        if not c: raise SystemExit(f'no CRD or placeholder for {kind}')
        seg = SEGMENT[kind]; short = kind.replace('NetBox', '')
        plural_snake = seg.replace('-', '_')
        snake = re.sub(r'(sses|xes)$', lambda m: m.group(1)[:-2], plural_snake) if re.search(r'(sses|xes)$', plural_snake) else plural_snake[:-1]
        tags.append({'name': short, 'description': f'{kind} ({c["plural"]}.{c["group"]}/{c["version"]})'})
        comps[f'{short}Spec'] = c['spec']; comps[f'{short}Status'] = c['status']
        rowprops = {'metadata': ref('ResourceMeta'), 'state': ref('State'), 'error': {'type': ['string', 'null']},
                    'lastReconciled': {'type': ['string', 'null'], 'format': 'date-time'},
                    'netboxId': {'type': ['integer', 'null'], 'format': 'int64'}, 'netboxUrl': {'type': ['string', 'null']},
                    'spec': {'type': 'object', 'description': 'Scalar spec fields for table columns', 'properties': scalar_props(c['spec'])}}
        stat_scalars = {k: v for k, v in scalar_props(c['status']).items() if k not in ('state', 'error', 'lastReconciled', 'netboxId', 'netboxUrl')}
        if stat_scalars: rowprops['status'] = {'type': 'object', 'properties': stat_scalars}
        comps[f'{short}Row'] = {'type': 'object', 'required': ['metadata', 'state', 'spec'], 'properties': rowprops}
        comps[f'{short}Detail'] = {'type': 'object', 'required': ['apiVersion', 'kind', 'metadata', 'spec'], 'properties': {
            'apiVersion': {'type': 'string', 'const': f'{c["group"]}/{c["version"]}'}, 'kind': {'type': 'string', 'const': kind},
            'metadata': ref('ResourceMeta'), 'spec': ref(f'{short}Spec'), 'status': {'oneOf': [ref(f'{short}Status'), {'type': 'null'}]},
            'conditions': {'type': 'array', 'items': ref('Condition')},
            'manifestYaml': {'type': 'string', 'description': 'The declared resource as YAML (spec only), for the Manifest panel'},
            'gitUrl': {'type': ['string', 'null'], 'description': 'Link to the declaring file in the GitOps repository when known'}}}
        comps[f'{short}List'] = {'type': 'object', 'required': ['items', 'page'], 'properties': {'items': {'type': 'array', 'items': ref(f'{short}Row')}, 'page': ref('PageInfo')}}
        # examples
        spec_ex = example_for(c['spec'], 'spec'); status_ex = example_for(c['status'], 'status')
        row_ex = {'metadata': {'name': f'{seg[:-1] if seg.endswith("s") else seg}-1', 'namespace': 'default', 'uid': '2a1f0c7e-1', 'creationTimestamp': '2026-09-01T10:00:00Z'},
                  'state': 'Created', 'error': None, 'lastReconciled': '2026-09-08T08:41:12Z', 'netboxId': 42, 'netboxUrl': f'http://netbox.netbox/api/{c["plural"]}/42/',
                  'spec': {k: example_for(v, k) for k, v in rowprops['spec']['properties'].items()}}
        det_ex = {'apiVersion': f'{c["group"]}/{c["version"]}', 'kind': kind, 'metadata': row_ex['metadata'], 'spec': spec_ex, 'status': status_ex,
                  'conditions': [{'type': 'Ready', 'status': 'True', 'reason': 'Reconciled', 'message': 'present in NetBox', 'lastTransitionTime': '2026-09-08T08:41:12Z'}],
                  'manifestYaml': yaml.safe_dump({'apiVersion': f'{c["group"]}/{c["version"]}', 'kind': kind, 'metadata': {'name': row_ex['metadata']['name'], 'namespace': 'default'}, 'spec': spec_ex}, sort_keys=False),
                  'gitUrl': None}
        params = [ref_p for ref_p in ('namespace', 'state', 'search', 'limit', 'cursor', 'sort')]
        extra = [{'name': f, 'in': 'query', 'required': False, 'schema': {'type': 'string'}, 'description': f'Filter by {f}'} for f in KIND_FILTERS.get(kind, [])]
        paths[f'/{seg}'] = {'get': {
            'operationId': f'list_{plural_snake}', 'tags': [short], 'summary': f'List {kind} resources as table rows',
            'security': [{'BearerAuth': []}], 'x-cors': 'inherit',
            'parameters': [{'$ref': f'#/components/parameters/{p}'} for p in params] + extra,
            'responses': {'200': resp(f'{kind} rows', ref(f'{short}List'), {'items': [row_ex], 'page': {'total': 1, 'nextCursor': None}}), **ERR_COMMON}}}
        paths[f'/{seg}/{{namespace}}/{{name}}'] = {'get': {
            'operationId': f'get_{snake}', 'tags': [short], 'summary': f'Get one {kind} with status, conditions and manifest',
            'security': [{'BearerAuth': []}], 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/pathNamespace'}, {'$ref': '#/components/parameters/pathName'}],
            'responses': {'200': resp(f'{kind}', ref(f'{short}Detail'), det_ex), **err('404', 'No such resource'), **ERR_COMMON}}}
        kind_counts.append({'kind': kind, 'plural': c['plural'], 'segment': seg, 'count': 1, 'byState': {'Created': 1}})
    # summary
    paths['/summary'] = {'get': {
        'operationId': f'get_{service}_summary', 'tags': ['Summary'], 'summary': f'Counts per kind and state for the {service} section landing page',
        'security': [{'BearerAuth': []}], 'x-cors': 'inherit',
        'parameters': [{'$ref': '#/components/parameters/namespace'}],
        'responses': {'200': resp('Per-kind counts', {'type': 'object', 'required': ['kinds', 'total'], 'properties': {'total': {'type': 'integer'}, 'kinds': {'type': 'array', 'items': ref('KindCount')}}}, {'total': len(kind_counts), 'kinds': kind_counts}), **ERR_COMMON}}}
    # device interfaces (DCIM device detail tab)
    if service == 'dcim':
        paths['/devices/{namespace}/{name}/interfaces'] = {'get': {
            'operationId': 'list_device_interfaces', 'tags': ['Device'], 'summary': 'Interfaces belonging to one device (device detail → Interfaces tab)',
            'security': [{'BearerAuth': []}], 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/pathNamespace'}, {'$ref': '#/components/parameters/pathName'}],
            'responses': {'200': resp('Interface rows', ref('InterfaceList'), {'items': [], 'page': {'total': 0, 'nextCursor': None}}), **err('404', 'No such device'), **ERR_COMMON}}}
    # boot lifecycle counts
    if service == 'boot':
        comps['LifecycleCounts'] = {'type': 'object', 'properties': {k: {'type': 'integer'} for k in ('discovered', 'installing', 'installed', 'locked')}}
        paths['/summary']['get']['responses']['200'] = resp('Per-kind counts plus lifecycle distribution', {'type': 'object', 'required': ['kinds', 'total', 'lifecycle'], 'properties': {'total': {'type': 'integer'}, 'kinds': {'type': 'array', 'items': ref('KindCount')}, 'lifecycle': ref('LifecycleCounts')}}, {'total': 63, 'kinds': kind_counts, 'lifecycle': {'discovered': 1, 'installing': 2, 'installed': 54, 'locked': 3}})
    if service == 'ippools':
        comps['IPPoolRow']['properties']['utilisationPercent'] = {'type': 'number', 'description': 'allocated / capacity × 100, computed server-side'}
    return spec_doc(service, title, paths, comps, tags)

def spec_doc(service, title, paths, comps, tags, params=None, extra_security=None):
    for item in paths.values():
        for method, op in item.items():
            if method in ('get', 'post', 'put', 'patch', 'delete'): op.setdefault('x-brrtrouter-impl', True)
    return {
        'openapi': '3.1.0',
        'info': {'title': f'DCops — {title}', 'version': '0.1.0',
                 'description': textwrap.dedent(f'''
                    {title}. Paths are service-relative; the BFF adds /api/v1/{service} (bff-suite-config.yaml).
                    Generated by openapi/dcops/scripts/build_specs.py from config/crd/all-crds.yaml and the screen contracts in
                    docs/03_ui_design — do not edit by hand. Version policy: stay on 3.1.0.''').strip()},
        'tags': tags,
        'paths': paths,
        'components': {'schemas': comps, 'parameters': params or COMMON_PARAMS, 'securitySchemes': SECURITY},
    }

# ----------------------------------------------------------------------------- platform (dashboard + shell)
def platform_service():
    comps = copy.deepcopy(COMMON)
    comps['CategoryCount'] = {'type': 'object', 'required': ['category', 'service', 'kinds', 'count'], 'properties': {
        'category': {'type': 'string', 'enum': ['ipam', 'dcim', 'tenancy', 'extras', 'boot', 'ippools', 'certificates']},
        'service': {'type': 'string', 'description': 'BFF service that serves the category'}, 'kinds': {'type': 'integer'}, 'count': {'type': 'integer'},
        'byState': {'type': 'object', 'additionalProperties': {'type': 'integer'}}}}
    comps['AttentionItem'] = {'type': 'object', 'required': ['kind', 'segment', 'service', 'metadata', 'state'], 'properties': {
        'kind': {'type': 'string'}, 'service': {'type': 'string'}, 'segment': {'type': 'string'}, 'metadata': ref('ResourceMeta'),
        'state': ref('State'), 'error': {'type': ['string', 'null']}, 'since': {'type': ['string', 'null'], 'format': 'date-time'}}}
    comps['Me'] = {'type': 'object', 'required': ['subject', 'displayName', 'roles'], 'properties': {
        'subject': {'type': 'string'}, 'displayName': {'type': 'string'}, 'email': {'type': ['string', 'null']},
        'roles': {'type': 'array', 'items': {'type': 'string'}, 'description': 'DCops roles after the claim→role mapping, e.g. dcops:certificates:import'},
        'idp': {'type': 'string', 'description': 'Issuer the token was validated against'}}}
    comps['KindDescriptor'] = {'type': 'object', 'required': ['kind', 'plural', 'category', 'service', 'segment', 'listPath', 'description'], 'properties': {
        'kind': {'type': 'string'}, 'plural': {'type': 'string'}, 'category': {'type': 'string'}, 'service': {'type': 'string'}, 'segment': {'type': 'string'},
        'listPath': {'type': 'string', 'description': 'BFF path that lists this kind'},
        'description': {'type': 'string', 'description': 'Plain-language one-liner for empty states and the Docs page'}}}
    S = [{'BearerAuth': []}]
    paths = {
        '/summary': {'get': {'operationId': 'get_platform_summary', 'tags': ['Dashboard'], 'summary': 'Dashboard totals: resources, healthy, pending, failed, by category', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/namespace'}],
            'responses': {'200': resp('Totals', {'type': 'object', 'required': ['total', 'healthy', 'pending', 'failed', 'kinds', 'categories', 'refreshedAt'], 'properties': {
                'total': {'type': 'integer'}, 'healthy': {'type': 'integer'}, 'pending': {'type': 'integer'}, 'failed': {'type': 'integer'}, 'kinds': {'type': 'integer'},
                'categories': {'type': 'array', 'items': ref('CategoryCount')}, 'refreshedAt': {'type': 'string', 'format': 'date-time'}}},
                {'total': 7437, 'healthy': 7391, 'pending': 31, 'failed': 15, 'kinds': 30, 'refreshedAt': '2026-09-08T08:41:12Z',
                 'categories': [{'category': 'ipam', 'service': 'ipam', 'kinds': 9, 'count': 412, 'byState': {'Created': 400, 'Pending': 9, 'Failed': 3}},
                                {'category': 'dcim', 'service': 'dcim', 'kinds': 11, 'count': 1208, 'byState': {'Created': 1200, 'Failed': 8}},
                                {'category': 'certificates', 'service': 'certificates', 'kinds': 3, 'count': 5368, 'byState': {'Created': 5364, 'Failed': 4}}]}), **ERR_COMMON}}},
        '/attention': {'get': {'operationId': 'list_attention', 'tags': ['Dashboard'], 'summary': 'Resources in Failed or Pending state across every kind (dashboard “Needs attention”)', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/namespace'}, {'$ref': '#/components/parameters/limit'}, {'$ref': '#/components/parameters/cursor'},
                           {'name': 'state', 'in': 'query', 'required': False, 'schema': {'type': 'string', 'enum': ['Failed', 'Pending']}}],
            'responses': {'200': resp('Attention items, Failed first then Pending, newest first', {'type': 'object', 'required': ['items', 'page'], 'properties': {'items': {'type': 'array', 'items': ref('AttentionItem')}, 'page': ref('PageInfo')}},
                {'items': [{'kind': 'NetBoxDevice', 'service': 'dcim', 'segment': 'devices', 'metadata': {'name': 'rtr-bansko-03', 'namespace': 'default', 'uid': 'u1', 'creationTimestamp': '2026-08-01T10:00:00Z'}, 'state': 'Failed', 'error': 'device type "CCR2004" not found in NetBox', 'since': '2026-09-08T08:30:00Z'}], 'page': {'total': 46, 'nextCursor': 'eyJvIjo1MH0'}}), **ERR_COMMON}}},
        '/kinds': {'get': {'operationId': 'list_kinds', 'tags': ['Discovery'], 'summary': 'Every resource kind the suite serves, with category, service and list path (replaces browser-side CRD discovery)', 'security': S, 'x-cors': 'inherit',
            'responses': {'200': resp('Kinds', {'type': 'object', 'required': ['kinds'], 'properties': {'kinds': {'type': 'array', 'items': ref('KindDescriptor')}}},
                {'kinds': [{'kind': 'NetBoxPrefix', 'plural': 'netboxprefixes', 'category': 'ipam', 'service': 'ipam', 'segment': 'prefixes', 'listPath': '/api/v1/ipam/prefixes'}]}), **ERR_COMMON}}},
        '/namespaces': {'get': {'operationId': 'list_namespaces', 'tags': ['Discovery'], 'summary': 'Namespaces the caller may read (header namespace selector)', 'security': S, 'x-cors': 'inherit',
            'responses': {'200': resp('Namespaces', {'type': 'object', 'required': ['namespaces'], 'properties': {'namespaces': {'type': 'array', 'items': {'type': 'object', 'required': ['name'], 'properties': {'name': {'type': 'string'}, 'resourceCount': {'type': 'integer'}}}}}},
                {'namespaces': [{'name': 'default', 'resourceCount': 1650}, {'name': 'certificates', 'resourceCount': 5371}, {'name': 'tenant-a', 'resourceCount': 61}]}), **ERR_COMMON}}},
        '/events': {'get': {'operationId': 'list_events', 'tags': ['Events'], 'summary': 'Kubernetes Events for one resource (detail pages → History)', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'kind', 'in': 'query', 'required': True, 'schema': {'type': 'string'}}, {'name': 'namespace', 'in': 'query', 'required': True, 'schema': {'type': 'string'}}, {'name': 'name', 'in': 'query', 'required': True, 'schema': {'type': 'string'}}, {'$ref': '#/components/parameters/limit'}],
            'responses': {'200': resp('Events, newest first', {'type': 'object', 'required': ['events'], 'properties': {'events': {'type': 'array', 'items': ref('Event')}}},
                {'events': [{'type': 'Normal', 'reason': 'Reconciled', 'message': 'NetBoxPrefix reconciled successfully', 'count': 3, 'firstTimestamp': '2026-09-08T06:00:00Z', 'lastTimestamp': '2026-09-08T08:41:12Z', 'source': 'netbox-controller'}]}), **err('404', 'No such resource'), **ERR_COMMON}}},
        '/me': {'get': {'operationId': 'get_me', 'tags': ['Identity'], 'summary': 'Who the token says I am and which DCops roles I hold (sidebar footer, write-button gating)', 'security': S, 'x-cors': 'inherit',
            'responses': {'200': resp('Identity', ref('Me'), {'subject': 'oid-3f2a', 'displayName': 'Charles', 'email': 'charles@example.com', 'roles': ['dcops:certificates:import'], 'idp': 'https://login.microsoftonline.com/<tenant>/v2.0'}), **ERR_COMMON}}},
        '/healthz': {'get': {'operationId': 'healthz', 'tags': ['Health'], 'summary': 'Liveness', 'security': [], 'x-cors': 'inherit',
            'responses': {'200': resp('OK', {'type': 'object', 'required': ['status'], 'properties': {'status': {'type': 'string', 'const': 'ok'}}}, {'status': 'ok'})}}},
    }
    return spec_doc('platform', 'Platform (dashboard, discovery, events, identity)', paths, comps, [{'name': 'Dashboard'}, {'name': 'Discovery'}, {'name': 'Events'}, {'name': 'Identity'}, {'name': 'Health'}])

# ----------------------------------------------------------------------------- certificates (EPIC-CM-002/003/009 + mockups)
def certificates_service():
    comps = copy.deepcopy(COMMON)
    comps['ColumnMapping'] = {'type': 'object', 'required': ['column', 'meaning'], 'properties': {
        'column': {'type': 'integer', 'minimum': 0, 'description': 'Zero-based column index in the scan file'},
        'meaning': {'type': 'string', 'enum': ['host', 'port', 'expiry', 'issuer', 'subject', 'severity', 'businessUnit', 'ignore'], 'description': 'What the column is used for; ignore drops it'},
        'header': {'type': ['string', 'null'], 'description': 'Header text as found in the file'},
        'sample': {'type': ['string', 'null'], 'description': 'First non-empty value, for the mapping table'}}}
    comps['RepreviewRequest'] = {'type': 'object', 'required': ['columnMapping'], 'properties': {'columnMapping': {'type': 'array', 'minItems': 1, 'items': ref('ColumnMapping')}}}
    comps['ErrorEnvelope']['properties']['mapping'] = {'type': 'array', 'description': 'On unknown_format: the best-guess column mapping so the manager can see what was detected', 'items': ref('ColumnMapping')}
    S = [{'BearerAuth': []}]
    comps.update({
        'Bucket': {'type': 'string', 'enum': ['fast-track', 'legacy', 'unclassified']},
        'IssuanceMode': {'type': 'string', 'enum': ['cert-manager', 'external', 'unknown']},
        'CertState': {'type': 'string', 'enum': ['Healthy', 'Renewal due', 'Drift found', 'Failed', 'Unowned', 'Stale', 'Not yet checked'], 'description': 'Plain-language state derived from the record\'s conditions (design §5.2); never raw condition JSON'},
        'Ownership': {'type': 'object', 'properties': {
            'team': {'type': ['string', 'null']}, 'contact': {'type': ['string', 'null'], 'format': 'email'},
            'escalation': {'type': 'array', 'items': {'type': 'object', 'required': ['after', 'to'], 'properties': {'after': {'type': 'string', 'pattern': '^[0-9]+(h|d)$'}, 'to': {'type': 'string', 'format': 'email'}}}},
            'businessUnits': {'type': 'array', 'items': {'type': 'string'}}, 'environment': {'type': ['string', 'null']}}},
        'CertificateRow': {'type': 'object', 'required': ['name', 'namespace', 'bucket', 'state', 'maturityLevel'], 'properties': {
            'name': {'type': 'string', 'description': 'Normalised hostname; the CR name'}, 'namespace': {'type': 'string'},
            'bucket': ref('Bucket'), 'issuanceMode': ref('IssuanceMode'), 'issuer': {'type': ['string', 'null']},
            'daysRemaining': {'type': ['integer', 'null']}, 'notAfter': {'type': ['string', 'null'], 'format': 'date-time'},
            'fingerprintSha256': {'type': ['string', 'null'], 'description': 'Observed leaf fingerprint, hex; null until probed'},
            'state': ref('CertState'), 'ownerTeam': {'type': ['string', 'null']}, 'severity': {'type': ['string', 'null'], 'enum': ['Critical', 'High', 'Medium', 'Routine', None]},
            'maturityLevel': {'type': 'integer', 'minimum': 1, 'maximum': 5}, 'acknowledged': {'type': 'boolean'}}},
        'CertificateList': {'type': 'object', 'required': ['items', 'page'], 'properties': {'items': {'type': 'array', 'items': ref('CertificateRow')}, 'page': ref('PageInfo')}},
        'ObservedRow': {'type': 'object', 'required': ['field', 'result'], 'properties': {
            'field': {'type': 'string', 'enum': ['fingerprint', 'issuer', 'names', 'expires', 'chain']}, 'observed': {'type': ['string', 'null']}, 'expected': {'type': ['string', 'null']},
            'result': {'type': 'string', 'enum': ['matches', 'differs', 'breach', 'not-checked']}}},
        'StatusLine': {'type': 'object', 'required': ['badge', 'tone', 'text'], 'properties': {'badge': {'type': 'string'}, 'tone': {'type': 'string', 'enum': ['ok', 'warn', 'bad', 'info', 'neutral']}, 'text': {'type': 'string'}}},
        'CertificateDetail': {'type': 'object', 'required': ['name', 'namespace', 'bucket', 'state', 'ownership', 'observedVsExpected', 'statusLines', 'plannedAction', 'maturityLevel'], 'properties': {
            'name': {'type': 'string'}, 'namespace': {'type': 'string'}, 'bucket': ref('Bucket'), 'issuanceMode': ref('IssuanceMode'), 'state': ref('CertState'),
            'ownership': ref('Ownership'), 'acknowledgement': {'type': ['object', 'null'], 'properties': {'by': {'type': 'string'}, 'at': {'type': 'string', 'format': 'date-time'}, 'note': {'type': ['string', 'null']}}},
            'observedVsExpected': {'type': 'array', 'items': ref('ObservedRow')}, 'statusLines': {'type': 'array', 'items': ref('StatusLine')},
            'plannedAction': {'type': 'object', 'properties': {'recommendedAction': {'type': ['string', 'null']}, 'nextReminder': {'type': ['string', 'null']}, 'escalatesTo': {'type': ['string', 'null']}, 'sopUrl': {'type': ['string', 'null']}}},
            'endpoints': {'type': 'array', 'items': {'type': 'object', 'required': ['host', 'port'], 'properties': {'host': {'type': 'string'}, 'port': {'type': 'integer'}, 'sni': {'type': ['string', 'null']}}}},
            'certManagerRef': {'type': ['object', 'null'], 'properties': {'name': {'type': 'string'}, 'namespace': {'type': 'string'}}},
            'conditions': {'type': 'array', 'items': ref('Condition')}, 'maturityLevel': {'type': 'integer'}, 'lastProbedAt': {'type': ['string', 'null'], 'format': 'date-time'}, 'nextProbeAt': {'type': ['string', 'null'], 'format': 'date-time'}}},
        'OwnershipPatch': {'type': 'object', 'properties': {'ownership': ref('Ownership'), 'bucket': ref('Bucket'), 'sopUrl': {'type': ['string', 'null']}}, 'minProperties': 1},
        'BulkOwnershipRequest': {'type': 'object', 'required': ['ownership', 'dryRun'], 'properties': {
            'selector': {'type': 'object', 'description': 'Reproduces the inventory table filters', 'properties': {'namespace': {'type': 'string'}, 'state': ref('CertState'), 'bucket': ref('Bucket'), 'owner': {'type': 'string', 'description': 'Team name, or "none" for unowned'}, 'severity': {'type': 'string'}, 'businessUnit': {'type': 'string'}, 'search': {'type': 'string'}}},
            'names': {'type': 'array', 'items': {'type': 'string'}, 'maxItems': 2000}, 'ownership': ref('Ownership'), 'bucket': ref('Bucket'), 'dryRun': {'type': 'boolean'}}},
        'BulkOwnershipResult': {'type': 'object', 'required': ['matched', 'updated', 'skipped', 'failed', 'dryRun'], 'description': 'With dryRun=true, updated/skipped are the would-update/would-skip counts and failed is always empty', 'properties': {'matched': {'type': 'integer'}, 'updated': {'type': 'integer'}, 'skipped': {'type': 'integer'}, 'failed': {'type': 'array', 'description': 'Records the apply run could not update (409 from SSA, transient errors); empty on dry run', 'items': {'type': 'object', 'required': ['name', 'reason'], 'properties': {'name': {'type': 'string'}, 'reason': {'type': 'string'}}}}, 'dryRun': {'type': 'boolean'}, 'skippedReasons': {'type': 'array', 'items': {'type': 'object', 'properties': {'name': {'type': 'string'}, 'reason': {'type': 'string'}}}}}},
        'CertificatesSummary': {'type': 'object', 'required': ['total', 'unowned', 'renewalDue30d', 'drift', 'failed', 'estateLevel'], 'properties': {
            'total': {'type': 'integer'}, 'unowned': {'type': 'integer'}, 'renewalDue30d': {'type': 'integer'}, 'drift': {'type': 'integer'}, 'failed': {'type': 'integer'}, 'stale': {'type': 'integer'},
            'estateLevel': {'type': 'integer', 'minimum': 1, 'maximum': 5}, 'byBucket': {'type': 'object', 'additionalProperties': {'type': 'integer'}}, 'refreshedAt': {'type': 'string', 'format': 'date-time'}}},
        'MaturitySummary': {'type': 'object', 'required': ['estateLevel', 'distribution', 'kpis', 'groups'], 'properties': {
            'estateLevel': {'type': 'integer'}, 'pinnedBy': {'type': 'string', 'description': 'Plain-language reason for the estate level'},
            'distribution': {'type': 'object', 'properties': {f'level{i}': {'type': 'integer'} for i in range(1, 6)}},
            'kpis': {'type': 'object', 'properties': {'ownerCoveragePercent': {'type': 'number'}, 'renewalHeadroomPercent': {'type': 'number'}, 'automationCoveragePercent': {'type': 'number'}, 'medianTimeToRotateHours': {'type': ['number', 'null']}}},
            'groupBy': {'type': 'string', 'enum': ['owner_team', 'business_unit', 'bucket']},
            'groups': {'type': 'array', 'items': {'type': 'object', 'required': ['name', 'certificates', 'level'], 'properties': {'name': {'type': ['string', 'null']}, 'certificates': {'type': 'integer'}, 'critical': {'type': 'integer'}, 'headroomPercent': {'type': 'number'}, 'automatedPercent': {'type': 'number'}, 'level': {'type': 'integer'}}}},
            'computedAt': {'type': 'string', 'format': 'date-time'}}},
        # imports (EPIC-CM-002)
        'ImportPhase': {'type': 'string', 'enum': ['AwaitingUpload', 'Pending', 'Previewing', 'AwaitingConfirmation', 'Applying', 'Completed', 'Failed', 'Cancelled', 'Expired']},
        'ImportCounters': {'type': 'object', 'properties': {k: {'type': 'integer'} for k in ('created', 'updated', 'unchanged', 'conflict', 'stale', 'error')}},
        'CreateImportRequest': {'type': 'object', 'required': ['filename', 'sizeBytes', 'contentSha256', 'contentType'], 'properties': {
            'filename': {'type': 'string', 'maxLength': 256}, 'sizeBytes': {'type': 'integer', 'minimum': 1, 'maximum': 12582912}, 'contentSha256': {'type': 'string', 'pattern': '^[a-f0-9]{64}$'},
            'contentType': {'type': 'string', 'enum': ['text/csv', 'application/octet-stream']}, 'namespace': {'type': 'string', 'default': 'certificates'},
            'formatHint': {'type': ['string', 'null']}, 'note': {'type': ['string', 'null'], 'maxLength': 256},
            'sample': {'type': ['string', 'null'], 'maxLength': 12288, 'description': 'Base64 of the first 8 KiB (≤ 12,288 chars), lets the API reject an unknown format immediately'},
            'mode': {'type': 'string', 'enum': ['previewThenApply', 'applyDirect'], 'default': 'previewThenApply'}}},
        'SignedUpload': {'type': 'object', 'required': ['url', 'method', 'headers', 'expiresAt'], 'properties': {'url': {'type': 'string', 'format': 'uri'}, 'method': {'type': 'string', 'const': 'PUT'}, 'headers': {'type': 'object', 'additionalProperties': {'type': 'string'}, 'description': 'Send exactly these headers with the PUT (content type, length, one-time precondition)'}, 'expiresAt': {'type': 'string', 'format': 'date-time'}}},
        'ImportAccepted': {'type': 'object', 'required': ['importId', 'phase', 'statusUrl'], 'properties': {'importId': {'type': 'string'}, 'phase': ref('ImportPhase'), 'upload': {'oneOf': [ref('SignedUpload'), {'type': 'null'}]}, 'statusUrl': {'type': 'string'}, 'previewUrl': {'type': 'string'}, 'eventsUrl': {'type': 'string'}}},
        'ImportStatus': {'type': 'object', 'required': ['importId', 'phase', 'requestedBy', 'createdAt'], 'properties': {
            'importId': {'type': 'string'}, 'namespace': {'type': 'string'}, 'phase': ref('ImportPhase'), 'filename': {'type': 'string'}, 'sizeBytes': {'type': 'integer'}, 'contentSha256': {'type': 'string'},
            'requestedBy': {'type': 'object', 'properties': {'subject': {'type': 'string'}, 'displayName': {'type': 'string'}}}, 'note': {'type': ['string', 'null']},
            'createdAt': {'type': 'string', 'format': 'date-time'}, 'updatedAt': {'type': 'string', 'format': 'date-time'},
            'preview': {'type': ['object', 'null'], 'properties': {'summary': ref('ImportCounters'), 'sha256': {'type': 'string'}, 'expiresAt': {'type': 'string', 'format': 'date-time'}, 'detectedFormat': {'type': 'string'}, 'columnMapping': {'type': 'array', 'items': ref('ColumnMapping')}, 'warnings': {'type': 'array', 'items': {'type': 'string'}}}},
            'result': {'type': ['object', 'null'], 'properties': {'summary': ref('ImportCounters'), 'completedAt': {'type': 'string', 'format': 'date-time'}, 'durationSeconds': {'type': 'number'}, 'errors': {'type': 'array', 'items': {'type': 'object', 'properties': {'line': {'type': 'integer'}, 'host': {'type': 'string'}, 'reason': {'type': 'string'}}}}}},
            'confirmation': {'type': ['object', 'null'], 'properties': {'confirmedBy': {'type': 'string'}, 'confirmedAt': {'type': 'string', 'format': 'date-time'}}},
            'artifacts': {'type': 'object', 'properties': {k: {'type': ['object', 'null'], 'properties': {'sizeBytes': {'type': 'integer'}, 'sha256': {'type': 'string'}}} for k in ('scan', 'preview', 'result')}},
            'lastError': {'type': ['string', 'null']}}},
        'ImportList': {'type': 'object', 'required': ['items', 'page'], 'properties': {'items': {'type': 'array', 'items': ref('ImportStatus')}, 'page': ref('PageInfo')}},
        'DiffRow': {'type': 'object', 'required': ['outcome', 'host'], 'properties': {'outcome': {'type': 'string', 'enum': ['created', 'updated', 'unchanged', 'conflict', 'stale', 'error']}, 'host': {'type': 'string'}, 'before': {'type': ['string', 'null']}, 'after': {'type': ['string', 'null']}, 'line': {'type': ['integer', 'null']}, 'reason': {'type': ['string', 'null']}}},
        'DiffPage': {'type': 'object', 'required': ['items', 'page'], 'properties': {'items': {'type': 'array', 'items': ref('DiffRow')}, 'page': ref('PageInfo')}},
        'ConfirmRequest': {'type': 'object', 'required': ['previewSha256'], 'properties': {'previewSha256': {'type': 'string'}}},
        'AcknowledgeRequest': {'type': 'object', 'properties': {'note': {'type': ['string', 'null'], 'maxLength': 512}}},
    })
    row_ex = {'name': 'www-dev.metro-vto.es', 'namespace': 'certificates', 'bucket': 'legacy', 'issuanceMode': 'external', 'issuer': 'Sectigo Limited', 'daysRemaining': 6, 'notAfter': '2026-09-14T23:59:59Z', 'state': 'Renewal due', 'ownerTeam': 'Network', 'severity': 'Critical', 'maturityLevel': 2, 'acknowledged': False}
    det_ex = {**{k: row_ex[k] for k in ('name', 'namespace', 'bucket', 'issuanceMode', 'state', 'maturityLevel')},
              'ownership': {'team': 'Network', 'contact': 'net-oncall@example.com', 'escalation': [{'after': '14d', 'to': 'platform-lead@example.com'}, {'after': '7d', 'to': 'security@example.com'}], 'businessUnits': ['METRO France', 'METRO digital'], 'environment': 'production'},
              'acknowledgement': None,
              'observedVsExpected': [{'field': 'fingerprint', 'observed': 'AB:CD:12:…:9F', 'expected': 'AB:CD:12:…:9F', 'result': 'matches'}, {'field': 'expires', 'observed': '2026-09-14 · 6 days', 'expected': '> 30 days (policy)', 'result': 'breach'}],
              'statusLines': [{'badge': 'Renewal due', 'tone': 'warn', 'text': 'Renewal is due — 6 days left. Escalation to platform-lead@ in 24 h if unacknowledged.'}],
              'plannedAction': {'recommendedAction': 'Onboard certificate and owner into central lifecycle management', 'nextReminder': 'in 6 days → expiry', 'escalatesTo': 'platform-lead@example.com', 'sopUrl': 'https://wiki.example.com/certs/legacy-renewal'},
              'endpoints': [{'host': 'www-dev.metro-vto.es', 'port': 443, 'sni': 'www-dev.metro-vto.es'}], 'certManagerRef': None,
              'conditions': [{'type': 'OwnerAssigned', 'status': 'True'}, {'type': 'Expiring', 'status': 'True', 'reason': 'Below7Days'}], 'lastProbedAt': '2026-09-08T08:40:12Z', 'nextProbeAt': '2026-09-08T14:40:12Z'}
    imp_ex = {'importId': '01J7QZ3M8V2K7Y9XQ4B1F0C2D3', 'namespace': 'certificates', 'phase': 'AwaitingConfirmation', 'filename': 'certificates.csv', 'sizeBytes': 1079797, 'contentSha256': '9f1c' + 'a' * 60,
              'requestedBy': {'subject': 'oid-3f2a', 'displayName': 'Charles'}, 'note': 'September BitSight export', 'createdAt': '2026-09-08T08:31:00Z', 'updatedAt': '2026-09-08T08:33:10Z',
              'preview': {'summary': {'created': 312, 'updated': 76, 'unchanged': 4979, 'conflict': 1, 'stale': 41, 'error': 0}, 'sha256': 'b7e2' + 'c' * 60, 'expiresAt': '2026-09-08T08:48:10Z', 'detectedFormat': 'headerless-scanner-v1',
                          'columnMapping': [{'column': 1, 'meaning': 'host', 'sample': 'authors.metro-vto.es'}, {'column': 4, 'meaning': 'expiry', 'sample': '12/18/2026'}], 'warnings': ['2 duplicate hosts; latest expiry kept']},
              'result': None, 'confirmation': None, 'artifacts': {'scan': {'sizeBytes': 1079797, 'sha256': '9f1c' + 'a' * 60}, 'preview': {'sizeBytes': 148221, 'sha256': 'b7e2' + 'c' * 60}, 'result': None}, 'lastError': None}
    W = 'Requires role dcops:certificates:import.'
    paths = {
        '/summary': {'get': {'operationId': 'get_certificates_summary', 'tags': ['Inventory'], 'summary': 'Headline tiles for the Certificates page', 'security': S, 'x-cors': 'inherit', 'parameters': [{'$ref': '#/components/parameters/namespace'}],
            'responses': {'200': resp('Summary', ref('CertificatesSummary'), {'total': 5368, 'unowned': 2829, 'renewalDue30d': 664, 'drift': 12, 'failed': 4, 'stale': 41, 'estateLevel': 1, 'byBucket': {'fast-track': 3211, 'legacy': 1172, 'unclassified': 985}, 'refreshedAt': '2026-09-08T08:41:12Z'}), **ERR_COMMON}}},
        '/inventory': {'get': {'operationId': 'list_certificates', 'tags': ['Inventory'], 'summary': 'Inventory table rows, filtered and paged (served from the controller reflector, not raw Kubernetes selectors)', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/namespace'}, {'$ref': '#/components/parameters/search'}, {'$ref': '#/components/parameters/limit'}, {'$ref': '#/components/parameters/cursor'}, {'$ref': '#/components/parameters/sort'},
                           {'name': 'state', 'in': 'query', 'required': False, 'schema': ref('CertState')}, {'name': 'bucket', 'in': 'query', 'required': False, 'schema': ref('Bucket')},
                           {'name': 'owner', 'in': 'query', 'required': False, 'schema': {'type': 'string'}, 'description': 'Owner team; "none" for unowned'},
                           {'name': 'businessUnit', 'in': 'query', 'required': False, 'schema': {'type': 'string'}}, {'name': 'severity', 'in': 'query', 'required': False, 'schema': {'type': 'string'}},
                           {'name': 'expiringWithinDays', 'in': 'query', 'required': False, 'schema': {'type': 'integer'}}],
            'responses': {'200': resp('Rows', ref('CertificateList'), {'items': [row_ex], 'page': {'total': 5368, 'nextCursor': 'eyJvIjo1MH0'}}), **ERR_COMMON}}},
        '/inventory/{namespace}/{name}': {'get': {'operationId': 'get_certificate', 'tags': ['Inventory'], 'summary': 'Record detail: observed vs expected, plain-language status, planned action', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/pathNamespace'}, {'$ref': '#/components/parameters/pathName'}],
            'responses': {'200': resp('Detail', ref('CertificateDetail'), det_ex), **err('404', 'No such certificate'), **ERR_COMMON}}},
        '/inventory/{namespace}/{name}/ownership': {'patch': {'operationId': 'patch_certificate_ownership', 'tags': ['Ownership'], 'summary': f'Assign owner, escalation, business units, bucket (SSA manager dcops-ui, force=false). {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/pathNamespace'}, {'$ref': '#/components/parameters/pathName'}],
            'requestBody': {'required': True, 'content': {'application/json': {'schema': ref('OwnershipPatch'), 'example': {'ownership': {'team': 'Network', 'contact': 'net-oncall@example.com', 'escalation': [{'after': '14d', 'to': 'platform-lead@example.com'}]}, 'bucket': 'legacy'}}}},
            'responses': {'200': resp('Updated detail', ref('CertificateDetail'), det_ex), **err('404', 'No such certificate'), **err('409', 'field_conflict: A field is owned by another manager (Git); message names the manager and path'), **err('422', 'validation_failed: Validation failed; fields[] lists the problems'), **ERR_COMMON}}},
        '/inventory/{namespace}/{name}/acknowledge': {'post': {'operationId': 'acknowledge_certificate', 'tags': ['Ownership'], 'summary': f'Record that someone is handling the current alert; pauses escalation for the current threshold. {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/pathNamespace'}, {'$ref': '#/components/parameters/pathName'}],
            'requestBody': {'required': False, 'content': {'application/json': {'schema': ref('AcknowledgeRequest'), 'example': {'note': 'Renewal ticket INC-4411 raised'}}}},
            'responses': {'200': resp('Updated detail', ref('CertificateDetail'), {**det_ex, 'acknowledgement': {'by': 'Charles', 'at': '2026-09-08T09:12:00Z', 'note': 'Renewal ticket INC-4411 raised'}}), **err('404', 'No such certificate'), **ERR_COMMON}}},
        '/bulk/ownership': {'post': {'operationId': 'bulk_assign_ownership', 'tags': ['Ownership'], 'summary': f'Assign ownership to many records by selector or name list; dryRun=true reports without writing. {W}', 'security': S, 'x-cors': 'inherit',
            'requestBody': {'required': True, 'content': {'application/json': {'schema': ref('BulkOwnershipRequest'), 'example': {'selector': {'namespace': 'certificates', 'state': 'Unowned', 'severity': 'Critical', 'businessUnit': 'METRO France'}, 'ownership': {'team': 'Network', 'contact': 'net-oncall@example.com'}, 'bucket': 'legacy', 'dryRun': True}}}},
            'responses': {'200': resp('Result', ref('BulkOwnershipResult'), {'matched': 312, 'updated': 300, 'skipped': 12, 'failed': [], 'dryRun': True, 'skippedReasons': [{'name': 'api.metro-vto.es', 'reason': 'spec.ownership.team is managed by kubectl (Git)'}]}), **err('422', 'invalid_selection: Selector and names both empty, or more than 2000 targets'), **ERR_COMMON}}},
        '/maturity': {'get': {'operationId': 'get_maturity', 'tags': ['Maturity'], 'summary': 'Estate maturity on the CMMI scale, KPIs and grouped breakdown', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'$ref': '#/components/parameters/namespace'}, {'name': 'environment', 'in': 'query', 'required': False, 'schema': {'type': 'string'}}, {'name': 'groupBy', 'in': 'query', 'required': False, 'schema': {'type': 'string', 'enum': ['owner_team', 'business_unit', 'bucket'], 'default': 'owner_team'}}],
            'responses': {'200': resp('Maturity', ref('MaturitySummary'), {'estateLevel': 1, 'pinnedBy': '2,829 unowned certificates, 137 of them critical', 'distribution': {'level1': 2829, 'level2': 1340, 'level3': 640, 'level4': 320, 'level5': 239},
                'kpis': {'ownerCoveragePercent': 47.3, 'renewalHeadroomPercent': 87.6, 'automationCoveragePercent': 77.5, 'medianTimeToRotateHours': None}, 'groupBy': 'owner_team',
                'groups': [{'name': 'Platform', 'certificates': 1204, 'critical': 12, 'headroomPercent': 98, 'automatedPercent': 91, 'level': 4}, {'name': None, 'certificates': 2829, 'critical': 137, 'headroomPercent': 81, 'automatedPercent': 74, 'level': 1}], 'computedAt': '2026-09-08T08:41:12Z'}), **ERR_COMMON}}},
        # imports
        '/imports': {
            'post': {'operationId': 'create_import', 'tags': ['Imports'], 'summary': f'Register a scan upload: validates metadata, sniffs the sample, dedupes by sha256, returns a one-time signed PUT URL. {W}', 'security': S, 'x-cors': 'inherit',
                'requestBody': {'required': True, 'content': {'application/json': {'schema': ref('CreateImportRequest'), 'example': {'filename': 'certificates.csv', 'sizeBytes': 1079797, 'contentSha256': '9f1c' + 'a' * 60, 'contentType': 'text/csv', 'namespace': 'certificates', 'note': 'September BitSight export', 'sample': 'YXV0aG9ycy5tZXRyby12dG8uZXMsMTEs…'}}}},
                'responses': {'201': resp('New import awaiting upload', ref('ImportAccepted'), {'importId': imp_ex['importId'], 'phase': 'AwaitingUpload', 'upload': {'url': 'https://storage.googleapis.com/dcops-imports-prod/imports/certificates/01J7QZ…/scan.csv?X-Goog-Algorithm=GOOG4-RSA-SHA256&…', 'method': 'PUT', 'headers': {'Content-Type': 'text/csv', 'Content-Length': '1079797', 'x-goog-if-generation-match': '0'}, 'expiresAt': '2026-09-08T08:46:00Z'}, 'statusUrl': '/api/v1/certificates/imports/01J7QZ…', 'previewUrl': '/api/v1/certificates/imports/01J7QZ…/preview', 'eventsUrl': '/api/v1/certificates/imports/01J7QZ…/events'}),
                              '200': resp('Same file already imported within 24 h; existing import returned, no new upload', ref('ImportAccepted'), {'importId': imp_ex['importId'], 'phase': 'Completed', 'upload': None, 'statusUrl': '/api/v1/certificates/imports/01J7QZ…'}),
                              **err('413', 'File too large'), **err('415', 'Content type not allowed'), **err('422', 'unknown_format: The file does not look like a scan export we know; mapping[] carries the best-guess column mapping'), **err('503', 'Upload store unavailable (signing failed)'), **ERR_COMMON}},
            'get': {'operationId': 'list_imports', 'tags': ['Imports'], 'summary': 'Import history, newest first', 'security': S, 'x-cors': 'inherit',
                'parameters': [{'$ref': '#/components/parameters/namespace'}, {'name': 'phase', 'in': 'query', 'required': False, 'schema': ref('ImportPhase')}, {'$ref': '#/components/parameters/limit'}, {'$ref': '#/components/parameters/cursor'}],
                'responses': {'200': resp('Imports', ref('ImportList'), {'items': [imp_ex], 'page': {'total': 4, 'nextCursor': None}}), **ERR_COMMON}}},
        '/imports/{importId}': {'get': {'operationId': 'get_import', 'tags': ['Imports'], 'summary': 'Import status, counters and artifact summaries', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'responses': {'200': resp('Status', ref('ImportStatus'), imp_ex), **err('404', 'No such import'), **ERR_COMMON}}},
        '/imports/{importId}/uploaded': {'post': {'operationId': 'mark_import_uploaded', 'tags': ['Imports'], 'summary': f'Client signals the PUT completed; the API verifies the object and hands off to processing. {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'responses': {'202': resp('Verified; phase Pending', ref('ImportStatus'), {**imp_ex, 'phase': 'Pending', 'preview': None}), **err('404', 'No such import'), **err('409', 'upload_incomplete: The uploaded object is missing or its size does not match — retry the upload (code is phase_mismatch when the import is not awaiting an upload)'), **ERR_COMMON}}},
        '/imports/{importId}/preview': {'get': {'operationId': 'get_import_preview', 'tags': ['Imports'], 'summary': 'Paged diff rows from the preview artifact', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}, {'name': 'outcome', 'in': 'query', 'required': False, 'schema': {'type': 'string', 'enum': ['created', 'updated', 'unchanged', 'conflict', 'stale', 'error']}}, {'$ref': '#/components/parameters/limit'}, {'$ref': '#/components/parameters/cursor'}],
            'responses': {'200': resp('Diff rows', ref('DiffPage'), {'items': [{'outcome': 'updated', 'host': 'www-dev.metro-vto.es', 'before': 'expiry 2026-09-12', 'after': 'expiry 2026-09-14 · issuer unchanged', 'line': 2, 'reason': None}, {'outcome': 'conflict', 'host': 'api.metro.cloud', 'before': 'spec.endpoints owned by kubectl', 'after': 'port 8443 in scan — not applied', 'line': 118, 'reason': 'field owned by manager kubectl at spec.endpoints'}], 'page': {'total': 430, 'nextCursor': 'eyJvIjo1MH0'}}), **err('404', 'No such import'), **err('409', 'preview_not_ready: The preview is still being generated'), **ERR_COMMON}}},
        '/imports/{importId}/repreview': {'post': {'operationId': 'repreview_import', 'tags': ['Imports'], 'summary': f'Re-run the preview with a corrected column mapping (manager fixed a misdetected column); the previous preview is discarded. {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'requestBody': {'required': True, 'content': {'application/json': {'schema': ref('RepreviewRequest'), 'example': {'columnMapping': [{'column': 0, 'meaning': 'host'}, {'column': 3, 'meaning': 'expiry'}, {'column': 5, 'meaning': 'issuer'}, {'column': 7, 'meaning': 'ignore'}]}}}},
            'responses': {'202': resp('Accepted; phase back to Previewing', ref('ImportStatus'), {**imp_ex, 'phase': 'Previewing', 'preview': None}), **err('404', 'No such import'), **err('409', 'phase_mismatch: Re-preview is only possible while awaiting confirmation'), **err('422', 'invalid_mapping: host and expiry must each be mapped exactly once'), **ERR_COMMON}}},
        '/imports/{importId}/artifacts/{kind}': {'get': {'operationId': 'get_import_artifact', 'tags': ['Imports'], 'summary': 'Redirect to a 5-minute signed GET for the original scan, full preview or full result', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}, {'name': 'kind', 'in': 'path', 'required': True, 'schema': {'type': 'string', 'enum': ['scan', 'preview', 'result']}}],
            'responses': {'302': {'description': 'Location: signed GET URL (≤ 5 min)', 'headers': {'Location': {'schema': {'type': 'string', 'format': 'uri'}}}}, **err('404', 'Artifact absent'), **ERR_COMMON}}},
        '/imports/{importId}/confirm': {'post': {'operationId': 'confirm_import', 'tags': ['Imports'], 'summary': f'Confirm the previewed diff; refused if the preview changed or expired. {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'requestBody': {'required': True, 'content': {'application/json': {'schema': ref('ConfirmRequest'), 'example': {'previewSha256': 'b7e2' + 'c' * 60}}}},
            'responses': {'202': resp('Confirmed; apply starts', ref('ImportStatus'), {**imp_ex, 'phase': 'Applying', 'confirmation': {'confirmedBy': 'Charles', 'confirmedAt': '2026-09-08T08:36:00Z'}}), **err('404', 'No such import'), **err('409', 'preview_changed: The preview changed or expired since you reviewed it — reload and review again (code is phase_mismatch when the import is not awaiting confirmation)'), **ERR_COMMON}}},
        '/imports/{importId}/cancel': {'post': {'operationId': 'cancel_import', 'tags': ['Imports'], 'summary': f'Cancel a non-terminal import. {W}', 'security': S, 'x-cors': 'inherit',
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'responses': {'202': resp('Cancelling', ref('ImportStatus'), {**imp_ex, 'phase': 'Cancelled'}), **err('404', 'No such import'), **err('409', 'phase_mismatch: This import has already finished'), **ERR_COMMON}}},
        '/imports/{importId}/events': {'get': {'operationId': 'stream_import_events', 'tags': ['Imports'], 'summary': 'Server-sent events: {phase, counters, updatedAt} on every change; closes on a terminal phase', 'security': S, 'x-cors': 'inherit', 'x-sse': True,
            'parameters': [{'name': 'importId', 'in': 'path', 'required': True, 'schema': {'type': 'string'}}],
            'responses': {'200': {'description': 'Stream of events', 'content': {'text/event-stream': {'schema': {'type': 'string'}, 'example': 'event: phase\ndata: {"phase":"Applying","counters":{"created":120},"updatedAt":"2026-09-08T08:36:30Z"}\n\n'}}}, **err('404', 'No such import'), **ERR_COMMON}}},
        '/healthz': {'get': {'operationId': 'healthz', 'tags': ['Health'], 'summary': 'Liveness', 'security': [], 'x-cors': 'inherit', 'responses': {'200': resp('OK', {'type': 'object', 'required': ['status'], 'properties': {'status': {'type': 'string', 'const': 'ok'}}}, {'status': 'ok'})}}},
    }
    return spec_doc('certificates', 'Certificates (inventory, ownership, maturity, scan imports)', paths, comps, [{'name': 'Inventory'}, {'name': 'Ownership'}, {'name': 'Maturity'}, {'name': 'Imports'}, {'name': 'Health'}])

# ----------------------------------------------------------------------------- suite config + BFF composition
def suite_config(services):
    return {
        'suite': 'dcops', 'bff_service_name': 'dcopsBFF', 'package_prefix': 'dcops', 'architecture': 'multi-suite',
        'openapi_base_dir': 'openapi/dcops', 'output_path': 'openapi/dcops/openapi_bff.yaml',
        'services': {s: {'base_path': f'/api/v1/{s}', 'gateway_path_style': 'prefixed', 'spec_path': f'{s}/openapi.yaml'} for s in services},
        'metadata': {'title': 'DCops API Gateway', 'version': '0.1.0',
                     'description': 'Backend for Frontend (BFF) for the DCops web UI. The browser talks only to this gateway with an OIDC bearer token; the gateway proxies to per-domain BRRTRouter services which read CRDs and call the certificate controller. This spec is automatically generated by openapi/dcops/scripts/build_specs.py — do not edit manually.',
                     'servers': [{'url': '/api/v1', 'description': 'DCops gateway (BFF route prefix)'}], 'bff_impl_operations': []},
    }

def compose_bff(specs, cfg):
    bff = {'openapi': '3.1.0', 'info': {'title': cfg['metadata']['title'], 'description': cfg['metadata']['description'], 'version': cfg['metadata']['version']},
           'servers': [{'url': '/api/v1', 'description': 'DCops gateway (BFF route prefix)'}], 'tags': [], 'paths': {},
           'components': {'schemas': {}, 'parameters': copy.deepcopy(COMMON_PARAMS), 'securitySchemes': SECURITY}}
    all_ids = [op['operationId'] for spec in specs.values() for item in spec['paths'].values() for m, op in item.items() if m in ('get', 'post', 'put', 'patch', 'delete')]
    colliding = {i for i in all_ids if all_ids.count(i) > 1}
    for svc, spec in specs.items():
        base = cfg['services'][svc]['base_path']  # /api/v1/<svc>
        prefix = base[len('/api/v1'):]
        for tag in spec.get('tags', []):
            if tag['name'] not in [t['name'] for t in bff['tags']]: bff['tags'].append({'name': tag['name'], 'description': tag.get('description', f'{svc} service')})
        for path, item in spec['paths'].items():
            out_path = prefix + ('' if path == '/' else path)
            newitem = {}
            for method, op in item.items():
                op = copy.deepcopy(op)
                if op['operationId'] in colliding: op['operationId'] = f'{svc}_{op["operationId"]}'
                op['x-brrtrouter-impl'] = False  # proxied to the downstream service; BFF-orchestrated ops would be true
                op['x-service'] = svc; op['x-service-base-path'] = base; op['x-brrtrouter-downstream-path'] = base + ('' if path == '/' else path)
                # rewrite $refs to namespaced component names
                def rewrite(o):
                    if isinstance(o, dict):
                        if '$ref' in o and o['$ref'].startswith('#/components/schemas/'):
                            n = o['$ref'].rsplit('/', 1)[1]
                            o['$ref'] = f'#/components/schemas/{n if n in COMMON else svc.capitalize() + n}'
                        for v in o.values(): rewrite(v)
                    elif isinstance(o, list):
                        for v in o: rewrite(v)
                rewrite(op); newitem[method] = op
            bff['paths'][out_path] = newitem
        for name, schema in spec['components']['schemas'].items():
            key = name if name in COMMON else f'{svc.capitalize()}{name}'
            sch = copy.deepcopy(schema)
            def rw(o):
                if isinstance(o, dict):
                    if '$ref' in o and o['$ref'].startswith('#/components/schemas/'):
                        n = o['$ref'].rsplit('/', 1)[1]; o['$ref'] = f'#/components/schemas/{n if n in COMMON else svc.capitalize() + n}'
                    for v in o.values(): rw(v)
                elif isinstance(o, list):
                    for v in o: rw(v)
            rw(sch); bff['components']['schemas'][key] = sch
    return bff

# ----------------------------------------------------------------------------- main
class Dumper(yaml.SafeDumper):
    def ignore_aliases(self, data): return True
def str_presenter(dumper, data):
    if '\n' in data: return dumper.represent_scalar('tag:yaml.org,2002:str', data, style='|')
    return dumper.represent_scalar('tag:yaml.org,2002:str', data)
Dumper.add_representer(str, str_presenter)
def dump(obj, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f: yaml.dump(obj, f, Dumper=Dumper, sort_keys=False, allow_unicode=True, width=120)

def main():
    ap = argparse.ArgumentParser(); ap.add_argument('--crds', default='config/crd/all-crds.yaml'); ap.add_argument('--out', default='openapi/dcops')
    a = ap.parse_args()
    crds = load_crds(a.crds)
    specs = {'platform': platform_service()}
    for svc, (title, kinds) in SERVICES.items(): specs[svc] = crd_service(svc, title, kinds, crds)
    specs['certificates'] = certificates_service()
    cfg = suite_config(list(specs.keys()))
    for svc, spec in specs.items(): dump(spec, os.path.join(a.out, svc, 'openapi.yaml'))
    dump(cfg, os.path.join(a.out, 'bff-suite-config.yaml'))
    dump(compose_bff(specs, cfg), os.path.join(a.out, 'openapi_bff.yaml'))
    n_ops = sum(len(m) for s in specs.values() for m in s['paths'].values())
    print(f'wrote {len(specs)} service specs, {n_ops} operations, BFF with {len(compose_bff(specs, cfg)["paths"])} paths')

if __name__ == '__main__':
    main()
