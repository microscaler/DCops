"""Generate DCops UI mockup artboards (.dc.html) sharing one shell, matching ui/ tokens."""
import json, os, html as H

# ---- tokens lifted from ui/src (App.tsx, dashboard components, index.css/tailwind defaults)
BG='#faf9f7'; SURF='#ffffff'; BORDER='#e5e3df'; PRIMARY='#4a5a4c'; DEEP='#2d4a2f'
TEXT='#2d3748'; MUTED='#4a5568'; SOFT='#6b7280'; FAINT='#9ca3af'; TINT='#f1f0ed'; TINT2='#f7f6f4'; GREENT='#e8f0e9'
AMBER='#f59e0b'; RED='#dc2626'; REDT='#fdecec'; AMBT='#fff6e7'; BLUE='#2f80ed'; BLUET='#eaf3ff'; OKG='#2fa36b'
FONT="ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif"
MONO="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace"
W,Hh=1440,900

def icon(name, size=18, color=MUTED):
    p={
     'grid':'<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
     'server':'<rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/><circle cx="7" cy="7" r="1"/><circle cx="7" cy="17" r="1"/>',
     'net':'<circle cx="12" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M12 7v5M12 12l-6 5M12 12l6 5"/>',
     'users':'<circle cx="9" cy="8" r="3"/><path d="M3 20c0-3.3 2.7-6 6-6s6 2.7 6 6"/><circle cx="17" cy="9" r="2.5"/><path d="M21 19c0-2.5-1.8-4.5-4-4.5"/>',
     'tag':'<path d="M3 12V4h8l9 9-8 8-9-9z"/><circle cx="7.5" cy="8.5" r="1.2"/>',
     'boot':'<rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M8 20h8M12 16v4M7 10l2.5 2L7 14"/>',
     'pool':'<path d="M4 6c2-2 4-2 6 0s4 2 6 0 4-2 4 0"/><path d="M4 12c2-2 4-2 6 0s4 2 6 0 4-2 4 0"/><path d="M4 18c2-2 4-2 6 0s4 2 6 0 4-2 4 0"/>',
     'cert':'<rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 8h8M8 12h8M8 16h5"/>',
     'upload':'<path d="M12 16V5M7 10l5-5 5 5"/><path d="M4 19h16"/>',
     'book':'<path d="M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z"/><path d="M4 21V5M8 7h7"/>',
     'search':'<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.3-4.3"/>',
     'chev':'<path d="M9 6l6 6-6 6"/>',
     'down':'<path d="M6 9l6 6 6-6"/>',
     'check':'<path d="M5 12l4 4 10-10"/>',
     'x':'<path d="M6 6l12 12M18 6L6 18"/>',
     'warn':'<path d="M12 3l10 18H2z"/><path d="M12 10v4M12 17v.5"/>',
     'refresh':'<path d="M20 12a8 8 0 1 1-2.3-5.7"/><path d="M20 4v5h-5"/>',
     'user':'<circle cx="12" cy="8" r="4"/><path d="M4 21c0-4.4 3.6-8 8-8s8 3.6 8 8"/>',
     'filter':'<path d="M3 5h18l-7 8v6l-4 2v-8z"/>',
     'plus':'<path d="M12 5v14M5 12h14"/>',
     'file':'<path d="M6 3h8l4 4v14H6z"/><path d="M14 3v4h4"/>',
     'clock':'<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
     'link':'<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>',
     'lock':'<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
    }[name]
    return f'<svg width="{size}" height="{size}" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="flex: none;">{p}</svg>'

NAV=[('Dashboard','grid'),('DCIM','server'),('IPAM','net'),('Tenancy','users'),('Extras','tag'),('Boot','boot'),('IP Pools','pool'),('Certificates','cert'),('Imports','upload'),('Docs','book')]

def shell(active, title, crumbs, body, ns='all', actions='', width=W, height=Hh, banner=''):
    items=''
    for label,ic in NAV:
        on = label==active
        bg = GREENT if on else 'transparent'; col = DEEP if on else MUTED; wt='600' if on else '500'
        items+=f'<div style="display: flex; align-items: center; gap: 10px; padding: 9px 12px; border-radius: 8px; background: {bg}; color: {col}; font-size: 14px; font-weight: {wt}; cursor: pointer;">{icon(ic,18,col)}<span>{label}</span></div>'
    crumb=''.join(f'<span style="color: {SOFT};">{H.escape(c)}</span>'+('<span style="color: '+FAINT+';">/</span>' if i<len(crumbs)-1 else '') for i,c in enumerate(crumbs))
    return f'''<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <script src="./support.js"></script>
</head>
<body>
<x-dc>
<helmet>
  <style>
    body {{ margin: 0; font-family: {FONT}; color: {TEXT}; -webkit-font-smoothing: antialiased; }}
    a {{ color: {DEEP}; }} a:hover {{ color: {PRIMARY}; }}
    table {{ border-collapse: collapse; }}
  </style>
</helmet>
<div style="width: {width}px; height: {height}px; background: {BG}; display: flex; overflow: hidden;">
  <aside style="width: 232px; flex: none; background: {SURF}; border-right: 1px solid {BORDER}; display: flex; flex-direction: column; padding: 16px 12px; gap: 4px;">
    <div style="display: flex; align-items: center; gap: 10px; padding: 6px 12px 18px 12px;">
      <div style="width: 28px; height: 28px; border-radius: 8px; background: {PRIMARY};"></div>
      <div style="font-size: 16px; font-weight: 700; color: {DEEP}; letter-spacing: -0.01em;">DCops</div>
    </div>
    {items}
    <div style="flex-grow: 1;"></div>
    <div style="display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-top: 1px solid {BORDER}; margin-top: 8px;">
      <div style="width: 28px; height: 28px; border-radius: 50%; background: {GREENT}; display: flex; align-items: center; justify-content: center;">{icon('user',16,DEEP)}</div>
      <div style="display: flex; flex-direction: column; gap: 1px;"><div style="font-size: 13px; font-weight: 600; color: {TEXT};">Charles</div><div style="font-size: 11px; color: {SOFT};">certificates:import</div></div>
    </div>
  </aside>
  <main style="flex-grow: 1; display: flex; flex-direction: column; min-width: 0;">
    <header style="height: 60px; flex: none; background: {SURF}; border-bottom: 1px solid {BORDER}; display: flex; align-items: center; padding: 0 28px; gap: 16px;">
      <div style="display: flex; align-items: center; gap: 8px; font-size: 13px;">{crumb}</div>
      <div style="flex-grow: 1;"></div>
      <div style="display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 10px 0 12px; border: 1px solid {BORDER}; border-radius: 8px; background: {SURF}; font-size: 13px; color: {MUTED}; min-width: 220px;">{icon('search',16,FAINT)}<span style="color: {FAINT};">Search resources…</span></div>
      <div style="display: flex; align-items: center; gap: 8px; height: 34px; padding: 0 10px 0 12px; border: 1px solid {BORDER}; border-radius: 8px; background: {SURF}; font-size: 13px; color: {MUTED};"><span style="color: {SOFT};">Namespace</span><strong style="color: {TEXT}; font-weight: 600;">{ns}</strong>{icon('down',14,FAINT)}</div>
      <div style="display: flex; align-items: center; gap: 6px; font-size: 12px; color: {SOFT};">{icon('refresh',14,SOFT)}<span>Last refresh 32 s ago</span></div>
    </header>
    {banner}
    <section style="flex-grow: 1; overflow: auto; padding: 24px 28px 28px 28px; display: flex; flex-direction: column; gap: 20px;">
      <div style="display: flex; align-items: flex-end; justify-content: space-between; gap: 16px;">
        <h1 style="margin: 0; font-size: 22px; font-weight: 700; color: {TEXT}; letter-spacing: -0.01em;">{H.escape(title)}</h1>
        <div style="display: flex; gap: 8px; align-items: center;">{actions}</div>
      </div>
      {body}
    </section>
  </main>
</div>
</x-dc>
</body>
</html>
'''

# ---- primitives
def btn(label, kind='primary', ic=None):
    if kind=='primary': st=f'background: {PRIMARY}; color: #ffffff; border: 1px solid {PRIMARY};'
    elif kind=='danger': st=f'background: {SURF}; color: {RED}; border: 1px solid {RED};'
    else: st=f'background: {SURF}; color: {TEXT}; border: 1px solid {BORDER};'
    return f'<div style="display: flex; align-items: center; gap: 8px; height: 36px; padding: 0 14px; border-radius: 8px; font-size: 13px; font-weight: 600; {st} cursor: pointer;">{icon(ic,16,"#ffffff" if kind=="primary" else (RED if kind=="danger" else MUTED)) if ic else ""}<span>{H.escape(label)}</span></div>'

def badge(text, tone='neutral'):
    m={'ok':(GREENT,DEEP),'warn':(AMBT,'#b45309'),'bad':(REDT,RED),'info':(BLUET,'#1d4ed8'),'neutral':(TINT,MUTED)}[tone]
    return f'<span style="display: inline-flex; align-items: center; height: 22px; padding: 0 9px; border-radius: 999px; background: {m[0]}; color: {m[1]}; font-size: 12px; font-weight: 600; white-space: nowrap;">{H.escape(text)}</span>'

def card(inner, pad=20, extra=''):
    return f'<div style="background: {SURF}; border: 1px solid {BORDER}; border-radius: 12px; padding: {pad}px; {extra}">{inner}</div>'

def stat(value, label, sub='', color=TEXT):
    return card(f'<div style="display: flex; flex-direction: column; gap: 6px;"><div style="font-size: 28px; font-weight: 700; color: {color}; letter-spacing: -0.02em;">{H.escape(value)}</div><div style="font-size: 13px; font-weight: 600; color: {TEXT};">{H.escape(label)}</div><div style="font-size: 12px; color: {SOFT};">{H.escape(sub)}</div></div>', 18)

def table(cols, rows, widths=None):
    th=''.join(f'<th style="text-align: left; font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: {SOFT}; padding: 10px 14px; border-bottom: 1px solid {BORDER}; background: {TINT2}; {("width: "+str(widths[i])+"px;") if widths else ""}">{H.escape(c)}</th>' for i,c in enumerate(cols))
    trs=''
    for r in rows:
        tds=''.join(f'<td style="padding: 11px 14px; border-bottom: 1px solid {BORDER}; font-size: 13px; color: {TEXT}; vertical-align: middle;">{c}</td>' for c in r)
        trs+=f'<tr>{tds}</tr>'
    return f'<div style="background: {SURF}; border: 1px solid {BORDER}; border-radius: 12px; overflow: hidden;"><table style="width: 100%;"><thead><tr>{th}</tr></thead><tbody>{trs}</tbody></table></div>'

def mono(t): return f'<span style="font-family: {MONO}; font-size: 12.5px;">{H.escape(t)}</span>'
def muted(t): return f'<span style="color: {SOFT};">{H.escape(t)}</span>'
def pager(shown, total):
    return f'<div style="display: flex; align-items: center; justify-content: space-between; font-size: 12px; color: {SOFT};"><span>Showing {shown} of {total}</span><div style="display: flex; gap: 6px;">{btn("Previous","secondary")}{btn("Next","secondary")}</div></div>'
def filterbar(chips, extra=''):
    c=''.join(f'<div style="display: flex; align-items: center; gap: 6px; height: 32px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 8px; background: {SURF}; font-size: 12.5px; color: {MUTED};"><span style="color: {SOFT};">{H.escape(k)}</span><strong style="font-weight: 600; color: {TEXT};">{H.escape(v)}</strong>{icon("down",13,FAINT)}</div>' for k,v in chips)
    return f'<div style="display: flex; align-items: center; gap: 8px;">{icon("filter",16,SOFT)}{c}{extra}</div>'
def kv(pairs):
    return '<div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px 24px;">'+''.join(f'<div style="display: flex; flex-direction: column; gap: 3px;"><div style="font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: {SOFT};">{H.escape(k)}</div><div style="font-size: 13px; color: {TEXT};">{v}</div></div>' for k,v in pairs)+'</div>'
def section(title, inner, right=''):
    return f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="display: flex; align-items: center; justify-content: space-between;"><div style="font-size: 14px; font-weight: 700; color: {TEXT};">{H.escape(title)}</div>{right}</div>{inner}</div>'
def bar(pct, color=PRIMARY, w=120):
    return f'<div style="display: flex; align-items: center; gap: 8px;"><div style="width: {w}px; height: 6px; border-radius: 3px; background: {TINT};"><div style="width: {int(w*pct/100)}px; height: 6px; border-radius: 3px; background: {color};"></div></div><span style="font-size: 12px; color: {SOFT};">{pct}%</span></div>'
def tabs(names, active):
    return '<div style="display: flex; gap: 4px; border-bottom: 1px solid '+BORDER+';">'+''.join(f'<div style="padding: 8px 14px; font-size: 13px; font-weight: {600 if n==active else 500}; color: {DEEP if n==active else SOFT}; border-bottom: 2px solid {DEEP if n==active else "transparent"}; margin-bottom: -1px;">{H.escape(n)}</div>' for n in names)+'</div>'

PRE_MANIFEST='''apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxPrefix
metadata:
  name: control-plane-prefix
  namespace: default
spec:
  prefix: 10.10.0.0/24
  vrf:
    kind: NetBoxVRF
    name: core
  status: Active'''
PRE_TENANT='''apiVersion: dcops.microscaler.io/v1alpha1
kind: NetBoxTenant
metadata:
  name: datacenter-tenant
spec:
  name: Datacenter Tenant
  tokenSecretRef:
    name: netbox-token-platform'''
boards={}
def add(name, html_): boards[name]=html_

# ============ Sign-in
add('SignIn', f'''<!doctype html>
<html><head><meta charset="utf-8"><script src="./support.js"></script></head>
<body><x-dc><helmet><style>body {{ margin: 0; font-family: {FONT}; color: {TEXT}; }} a {{ color: {DEEP}; }} a:hover {{ color: {PRIMARY}; }}</style></helmet>
<div style="width: {W}px; height: {Hh}px; background: {BG}; display: flex; align-items: center; justify-content: center;">
  <div style="width: 420px; background: {SURF}; border: 1px solid {BORDER}; border-radius: 16px; padding: 36px; display: flex; flex-direction: column; gap: 22px;">
    <div style="display: flex; align-items: center; gap: 12px;"><div style="width: 36px; height: 36px; border-radius: 10px; background: {PRIMARY};"></div><div style="font-size: 20px; font-weight: 700; color: {DEEP};">DCops</div></div>
    <div style="display: flex; flex-direction: column; gap: 6px;"><div style="font-size: 18px; font-weight: 700;">Sign in</div><div style="font-size: 13px; color: {SOFT}; line-height: 1.5;">Use your organisation account. DCops never stores a password; you are redirected to the identity provider and returned here.</div></div>
    <div style="display: flex; align-items: center; justify-content: center; gap: 10px; height: 44px; border-radius: 10px; background: {PRIMARY}; color: #ffffff; font-size: 14px; font-weight: 600;">{icon('lock',16,'#ffffff')}<span>Continue with Microscaler ID</span></div>
    <div style="font-size: 12px; color: {FAINT}; text-align: center;">Read-only access is granted to everyone in the organisation. Writes require a role assigned by your platform team.</div>
  </div>
</div>
</x-dc></body></html>''')

# ============ Dashboard
cats=[('IPAM','net','9 kinds','412'),('DCIM','server','11 kinds','1,208'),('Tenancy','users','2 kinds','23'),('Extras','tag','1 kind','41'),('Boot','boot','2 kinds','63'),('IP Pools','pool','2 kinds','318'),('Certificates','cert','3 kinds','5,368')]
catgrid='<div style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px;">'+''.join(card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="display: flex; align-items: center; justify-content: space-between;"><div style="display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 600;">{icon(ic,16,DEEP)}<span>{n}</span></div>{icon("chev",14,FAINT)}</div><div style="font-size: 24px; font-weight: 700; letter-spacing: -0.02em;">{cnt}</div><div style="font-size: 12px; color: {SOFT};">{k} · resources</div></div>',16) for n,ic,k,cnt in cats)+'</div>'
errs=table(['Resource','Kind','Namespace','State','Error'],[
 [mono('rtr-bansko-03'),'NetBoxDevice','default',badge('Failed','bad'),muted('device type "CCR2004" not found in NetBox')],
 [mono('shop.example.com'),'CertificateInventory','certificates',badge('Failed','bad'),muted('TLS probe: connection refused (2 attempts)')],
 [mono('10.40.0.0/16'),'NetBoxPrefix','default',badge('Pending','warn'),muted('waiting on NetBoxVRF "core"')],
])
dash=f'''
<div style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px;">
  {stat('7,437','Resources','across 30 kinds')}{stat('7,391','Healthy','Created or Updated',DEEP)}{stat('31','Pending','awaiting dependencies','#b45309')}{stat('15','Failed','need attention',RED)}
</div>
{section('By category', catgrid)}
{section('Needs attention', errs, f'<a style="font-size: 13px;">View all 46</a>')}
'''
add('Dashboard', shell('Dashboard','Platform overview',['Dashboard'],dash, actions=btn('Refresh','secondary','refresh')))

# ============ Generic list: IPAM → Prefixes
kinds_ipam=['Prefixes','IP addresses','IP ranges','Aggregates','VLANs','RIRs','VRFs','Route targets','Roles']
subnav='<div style="display: flex; gap: 6px; flex-wrap: wrap;">'+''.join(f'<div style="height: 30px; display: flex; align-items: center; padding: 0 12px; border-radius: 999px; font-size: 12.5px; font-weight: 600; background: {GREENT if k=="Prefixes" else SURF}; color: {DEEP if k=="Prefixes" else MUTED}; border: 1px solid {"transparent" if k=="Prefixes" else BORDER};">{k}</div>' for k in kinds_ipam)+'</div>'
prefix_rows=[
 [mono('control-plane-prefix'),mono('10.10.0.0/24'),'default','core','Active',badge('Created','ok'),bar(17),'42',muted('3 min ago')],
 [mono('workers-a'),mono('10.10.1.0/24'),'default','core','Active',badge('Created','ok'),bar(88,'#b45309'),'43',muted('5 min ago')],
 [mono('storage-lan'),mono('10.20.0.0/22'),'default','storage','Reserved',badge('Updated','ok'),bar(14),'57',muted('1 h ago')],
 [mono('10.40.0.0/16'),mono('10.40.0.0/16'),'default','—','Container',badge('Pending','warn'),bar(0),muted('—'),muted('12 min ago')],
 [mono('pxe-boot'),mono('192.168.50.0/24'),'tenant-a','—','Active',badge('Created','ok'),bar(31),'61',muted('2 d ago')],
 [mono('mgmt-oob'),mono('172.16.0.0/24'),'default','oob','Active',badge('Failed','bad'),bar(9),muted('—'),muted('now')],
]
lst=f'''
{subnav}
<div style="display: flex; align-items: center; justify-content: space-between;">{filterbar([('State','All'),('VRF','All'),('Site','All')])}<div style="display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 10px; border: 1px solid {BORDER}; border-radius: 8px; background: {SURF}; font-size: 12.5px; color: {FAINT}; width: 260px;">{icon('search',14,FAINT)}<span>Filter by name or CIDR</span></div></div>
{table(['Name','Prefix','Namespace','VRF','Status','State','Utilisation','NetBox ID','Reconciled'],prefix_rows)}
{pager('6','128')}
'''
add('IpamPrefixes', shell('IPAM','Prefixes',['IPAM','Prefixes'],lst, actions=btn('Open in NetBox','secondary','link')))

# ============ Generic detail: NetBoxPrefix
cond=table(['Type','Status','Reason','Last transition','Message'],[
 ['Ready',badge('True','ok'),'Reconciled',muted('2026-09-08 08:41'),muted('prefix 10.10.0.0/24 present in NetBox (id 42)')],
 ['DependenciesResolved',badge('True','ok'),'VRFFound',muted('2026-09-08 08:41'),muted('vrf "core" → NetBox id 7')],
])
events=table(['Age','Type','Reason','Message'],[
 [muted('3m'),badge('Normal','ok'),'Reconciled',muted('NetBoxPrefix reconciled successfully')],
 [muted('41m'),badge('Normal','ok'),'Updated',muted('description changed; NetBox object 42 patched')],
 [muted('2d'),badge('Warning','warn'),'DependencyPending',muted('waiting on NetBoxVRF "core"')],
])
detail=f'''
<div style="display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); gap: 20px;">
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {card(kv([('Prefix',mono('10.10.0.0/24')),('State',badge('Created','ok')),('VRF','core'),('Tenant','datacenter-tenant'),('Site','datacenter-1'),('Status','Active'),('Description','Control plane nodes'),('Tags','managed-by=gitops · owner=microscaler')]))}
    {section('Conditions', cond)}
    {section('Events', events)}
  </div>
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">NetBox</div>{kv([("ID","42"),("URL",f"<a>ipam/prefixes/42/</a>")])}<div style="font-size: 12px; color: {SOFT}; line-height: 1.5;">Git declares this prefix; NetBox is the authoritative IPAM store. Edit the YAML in Git to change it.</div></div>')}
    {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">Manifest</div><pre style="margin: 0; font-family: {MONO}; font-size: 11.5px; line-height: 1.5; color: {MUTED}; white-space: pre-wrap;">{PRE_MANIFEST}</pre><div style="display: flex; gap: 8px;">{btn("Copy YAML","secondary")}{btn("View in Git","secondary","link")}</div></div>')}
  </div>
</div>
'''
add('IpamPrefixDetail', shell('IPAM','control-plane-prefix',['IPAM','Prefixes','control-plane-prefix'],detail, actions=badge('NetBoxPrefix','info')))

# ============ DCIM landing (sites/devices)
dcim_kinds=[('Sites','4'),('Locations','11'),('Regions','2'),('Site groups','1'),('Devices','186'),('Device types','14'),('Device roles','7'),('Manufacturers','5'),('Platforms','6'),('Interfaces','940'),('MAC addresses','32')]
dgrid='<div style="display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 12px;">'+''.join(card(f'<div style="display: flex; flex-direction: column; gap: 4px;"><div style="font-size: 20px; font-weight: 700;">{c}</div><div style="font-size: 12.5px; color: {MUTED};">{n}</div></div>',14) for n,c in dcim_kinds)+'</div>'
dev_rows=[
 [mono('cp-01'),'datacenter-1','Raspberry Pi CM4','control-plane','Active',mono('10.10.0.11'),badge('Created','ok')],
 [mono('cp-02'),'datacenter-1','Raspberry Pi CM4','control-plane','Active',mono('10.10.0.12'),badge('Created','ok')],
 [mono('worker-07'),'datacenter-1','Raspberry Pi 4','worker','Planned',mono('10.10.1.27'),badge('Pending','warn')],
 [mono('sw-core-01'),'datacenter-1','MikroTik CRS326','switch','Active',mono('172.16.0.2'),badge('Created','ok')],
 [mono('rtr-bansko-03'),'bansko-edge','MikroTik CCR2004','router','Active',mono('172.16.9.1'),badge('Failed','bad')],
]
dcim=f'''
{section('Inventory', dgrid)}
{section('Devices', table(['Name','Site','Device type','Role','Status','Primary IP','State'],dev_rows), filterbar([('Site','All'),('Role','All')]))}
'''
add('DcimOverview', shell('DCIM','Data centre inventory',['DCIM'],dcim, actions=btn('Open in NetBox','secondary','link')))

# ============ Device detail with interfaces
ifaces=table(['Interface','Type','MAC','IP','Connected to','State'],[
 [mono('eth0'),'1000BASE-T',mono('dc:a6:32:1f:04:9a'),mono('10.10.0.11/24'),'sw-core-01 · ether3',badge('Created','ok')],
 [mono('wlan0'),'IEEE 802.11ac',mono('dc:a6:32:1f:04:9b'),muted('—'),muted('—'),badge('Created','ok')],
])
devd=f'''
{tabs(['Overview','Interfaces','Boot','Events','Manifest'],'Overview')}
<div style="display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); gap: 20px;">
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {card(kv([('Device type','Raspberry Pi CM4'),('Role','control-plane'),('Site','datacenter-1 · rack A2 · U14'),('Platform','Talos Linux 1.9'),('Primary IPv4',mono('10.10.0.11')),('Serial','10000000a3b4c5d6'),('Tenant','datacenter-tenant'),('Status','Active')]))}
    {section('Interfaces', ifaces)}
  </div>
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">Boot intent</div>{kv([("Profile","talos-1.9-cm4"),("Lifecycle",badge("Installed","ok")),("MAC",mono("dc:a6:32:1f:04:9a")),("Last boot",muted("2026-09-02 04:12"))])}<a style="font-size: 13px;">Open BootIntent</a></div>')}
    {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">NetBox</div>{kv([("ID","118"),("URL","<a>dcim/devices/118/</a>")])}</div>')}
  </div>
</div>
'''
add('DcimDeviceDetail', shell('DCIM','cp-01',['DCIM','Devices','cp-01'],devd, actions=badge('NetBoxDevice','info')))

# ============ Tenancy
ten=table(['Tenant','Group','Namespace','Sites','Prefixes','Token secret','State'],[
 [mono('datacenter-tenant'),'platform','default','2','61',mono('netbox-token-platform'),badge('Created','ok')],
 [mono('tenant-a'),'customers','tenant-a','1','9',mono('netbox-token-tenant-a'),badge('Created','ok')],
 [mono('lab'),'internal','lab','1','4',mono('netbox-token-lab'),badge('Updated','ok')],
])
tenancy=f'''
<div style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px;">{stat('3','Tenants')}{stat('3','Tenant groups')}{stat('3','Namespaces mapped')}{stat('3','API tokens resolved',sub='from Tenant CRDs',color=DEEP)}</div>
{section('Tenants', ten)}
{card(f'<div style="font-size: 12.5px; color: {SOFT}; line-height: 1.6;">Multi-tenant mode: each NetBoxTenant carries a reference to the Secret holding its NetBox API token. Controllers resolve the token per tenant; no cluster-wide token exists.</div>',14)}
'''
add('Tenancy', shell('Tenancy','Tenants',['Tenancy'],tenancy))

# ============ Extras (tags)
tagrows=[[mono('managed-by=gitops'),'#4a5a4c','1,204','All controllers',badge('Created','ok')],[mono('owner=microscaler'),'#2d4a2f','1,198','Platform',badge('Created','ok')],[mono('env=lab'),'#f59e0b','32','Lab',badge('Created','ok')]]
ex=f'''
{section('Tags', table(['Tag','Colour','Objects','Scope','State'],[[r[0],f'<span style="display: inline-flex; align-items: center; gap: 8px;"><span style="width: 12px; height: 12px; border-radius: 3px; background: {r[1]};"></span>{mono(r[1])}</span>',r[2],r[3],r[4]] for r in tagrows]))}
{card(f'<div style="font-size: 12.5px; color: {SOFT}; line-height: 1.6;">Tags mark NetBox objects that DCops owns. Objects without the managed-by tag are never modified by controllers.</div>',14)}
'''
add('Extras', shell('Extras','Tags',['Extras','Tags'],ex))

# ============ Boot
bp=table(['Profile','Talos version','Schematic','Kernel args','Intents','State'],[
 [mono('talos-1.9-cm4'),'v1.9.4',mono('376567988a…'),muted('talos.platform=metal'),'12',badge('Created','ok')],
 [mono('talos-1.9-pi4'),'v1.9.4',mono('b1d4f0c2e7…'),muted('talos.platform=metal'),'48',badge('Created','ok')],
 [mono('rescue-alpine'),'—',muted('—'),muted('console=ttyS0'),'0',badge('Created','ok')],
])
bi=table(['MAC','Device','Profile','Lifecycle','Last boot','State'],[
 [mono('dc:a6:32:1f:04:9a'),'cp-01','talos-1.9-cm4',badge('Installed','ok'),muted('2026-09-02 04:12'),badge('Created','ok')],
 [mono('dc:a6:32:1f:05:11'),'worker-07','talos-1.9-pi4',badge('Installing','warn'),muted('now'),badge('Created','ok')],
 [mono('dc:a6:32:1f:05:12'),'worker-08','talos-1.9-pi4',badge('Discovered','info'),muted('—'),badge('Pending','warn')],
 [mono('dc:a6:32:1f:03:70'),'cp-03','talos-1.9-cm4',badge('Locked','neutral'),muted('2026-08-14 22:05'),badge('Created','ok')],
])
boot=f'''
<div style="display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px;">{stat('60','Boot intents')}{stat('54','Installed',color=DEEP)}{stat('2','Installing',color='#b45309')}{stat('1','Discovered',sub='not yet booted')}{stat('3','Locked',sub='reinstall prevented')}</div>
{section('Boot profiles', bp)}
{section('Boot intents', bi, filterbar([('Lifecycle','All'),('Profile','All')]))}
'''
add('Boot', shell('Boot','PXE boot control',['Boot'],boot))

# ============ IP Pools
pools=table(['Pool','Prefix','VRF','Allocated','Utilisation','Claims','State'],[
 [mono('control-plane'),mono('10.10.0.0/24'),'core','42 / 251',bar(17),'42',badge('Created','ok')],
 [mono('workers'),mono('10.10.1.0/24'),'core','221 / 251',bar(88,'#b45309'),'221',badge('Created','ok')],
 [mono('pxe'),mono('192.168.50.0/24'),'—','55 / 251',bar(22),'55',badge('Created','ok')],
])
claims=table(['Claim','Pool','Requested by','Address','MAC','State'],[
 [mono('cp-01-mgmt'),'control-plane','NetBoxDevice/cp-01',mono('10.10.0.11'),mono('dc:a6:32:1f:04:9a'),badge('Created','ok')],
 [mono('worker-07-mgmt'),'workers','NetBoxDevice/worker-07',mono('10.10.1.27'),mono('dc:a6:32:1f:05:11'),badge('Created','ok')],
 [mono('worker-08-mgmt'),'workers','NetBoxDevice/worker-08',muted('allocating…'),mono('dc:a6:32:1f:05:12'),badge('Pending','warn')],
])
ipp=f'''
{section('Pools', pools)}
{section('Recent claims', claims, f'<a style="font-size: 13px;">View all 318</a>')}
'''
add('IpPools', shell('IP Pools','IP pools & claims',['IP Pools'],ipp))

# ============ Certificates inventory
def state(s):
    return {'Healthy':badge('Healthy','ok'),'Renewal due':badge('Renewal due','warn'),'Drift found':badge('Drift found','bad'),'Failed':badge('Failed','bad'),'Unowned':badge('Unowned','neutral'),'Stale':badge('Stale','neutral')}[s]
certrows=[
 [mono('api.metro.cloud'),badge('fast-track','info'),'cert-manager','89 d',state('Healthy'),'Platform','L4'],
 [mono('www-dev.metro-vto.es'),badge('legacy','neutral'),'Sectigo','6 d',state('Renewal due'),'Network','L2'],
 [mono('360.metroag.de'),badge('fast-track','info'),"Let's Encrypt",'41 d',state('Drift found'),'Web','L3'],
 [mono('shop.example.com'),badge('legacy','neutral'),'DigiCert','—',state('Failed'),'Platform','L2'],
 [mono('authors.arbeitgeber-ahd.de'),badge('unclassified','neutral'),'Sectigo','23 d',state('Unowned'),muted('— assign'),'L1'],
 [mono('ar-guide.miag.com'),badge('legacy','neutral'),'Sectigo','101 d',state('Stale'),'Network','L2'],
]
certs=f'''
<div style="display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px;">{stat('5,368','Certificates')}{stat('2,829','Unowned',sub='53% · Level 1',color=RED)}{stat('664','Renewal due ≤ 30 d',color='#b45309')}{stat('12','Drift found',color=RED)}{stat('L1','Estate maturity',sub='weakest critical certificate',color=DEEP)}</div>
<div style="display: flex; align-items: center; justify-content: space-between;">{filterbar([('State','All'),('Bucket','All'),('Owner','All'),('Business unit','All')])}<div style="display: flex; gap: 8px;">{btn('Assign owners','secondary','users')}{btn('Export CSV','secondary','file')}</div></div>
{table(['Target','Bucket','Issuer','Expires','State','Owner','Level'],[[f'<span style="display: inline-flex; align-items: center; gap: 10px;"><span style="width: 14px; height: 14px; border: 1px solid {BORDER}; border-radius: 3px; background: {SURF};"></span>{r[0]}</span>']+r[1:] for r in certrows])}
{pager('6','5,368')}
'''
add('CertificatesInventory', shell('Certificates','Certificates',['Certificates'],certs, ns='certificates', actions=btn('Import scan','primary','upload')))

# ============ Certificate detail
obs=table(['','Observed on live site','Expected','Result'],[
 ['Fingerprint',mono('AB:CD:12:…:9F'),mono('AB:CD:12:…:9F'),badge('matches','ok')],
 ['Issuer','Sectigo RSA DV CA','Sectigo Limited',badge('matches','ok')],
 ['Names covered','www-dev.metro-vto.es','www-dev.metro-vto.es',badge('matches','ok')],
 ['Expires','14 Sep 2026 · 6 days','> 30 days (policy)',badge('breach','bad')],
 ['Chain','valid','valid',badge('matches','ok')],
])
statuslist=''.join(f'<div style="display: flex; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid {BORDER}; font-size: 13px;">{b}<span>{t}</span></div>' for b,t in [(badge('Renewal due','warn'),'Renewal is due — 6 days left. Reminder sent at 14 days; escalation to platform-lead@ in 24 h if unacknowledged.'),(badge('Owned','ok'),'Owner: Network team (net-oncall@). Assigned by Charles, 3 Sep.'),(badge('Checked','ok'),'Last checked 8 Sep 08:40 · next check in 5 h 20 m.'),(badge('Level 2','info'),'Managed — reaches Level 3 once an SOP link and escalation chain are set.')])
hist=table(['When','Event','Detail'],[[muted('08:40 today'),'Probed',muted('presented fingerprint AB:CD…9F, 6 days remaining')],[muted('Yesterday'),'Renewal due',muted('7-day threshold crossed → Warning event, Teams notified')],[muted('3 Sep'),'OwnerAssigned',muted('Network team, by Charles')],[muted('1 Sep'),'Imported',muted('scan import #01J7… (BitSight export)')]])
panel=f'''
{tabs(['Assign owner','Acknowledge','Change bucket'],'Acknowledge').replace('padding: 8px 14px; font-size: 13px;','padding: 8px 8px; font-size: 12.5px; white-space: nowrap;')}
<div style="display: flex; flex-direction: column; gap: 10px;">
  <div style="font-size: 12.5px; color: {SOFT}; line-height: 1.5;">Acknowledging records that someone is handling this renewal and pauses escalation for the current threshold.</div>
  <div style="border: 1px solid {BORDER}; border-radius: 8px; padding: 10px 12px; font-size: 13px; color: {FAINT}; min-height: 64px;">Note (e.g. ticket number)…</div>
  {btn('Acknowledge','primary','check')}
  <div style="font-size: 12px; color: {SOFT};">Last acknowledged: never</div>
</div>
'''
cd=f'''
<div style="display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); gap: 20px;">
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {section('Observed vs expected', obs)}
    {section('Status', card(statuslist,4))}
    {section('History', hist)}
  </div>
  <div style="display: flex; flex-direction: column; gap: 20px;">
    {card(panel)}
    {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">Planned action</div><div style="font-size: 13px; line-height: 1.5;">{H.escape("Onboard certificate and owner into central lifecycle management")}</div>{kv([("Next reminder","in 6 days → expiry"),("Escalates to","platform-lead@"),("Business units","METRO France · METRO digital"),("SOP","<a>Legacy renewal runbook</a>")])}</div>')}
  </div>
</div>
'''
add('CertificateDetail', shell('Certificates','www-dev.metro-vto.es',['Certificates','www-dev.metro-vto.es'],cd, ns='certificates', actions=state('Renewal due')+badge('legacy','neutral')))

# ============ Import wizard (4 steps) — one artboard per step
def steps(active):
    names=['Upload','Review changes','Confirm','Summary']
    out='<div style="display: flex; align-items: center; gap: 0;">'
    for i,n in enumerate(names):
        done=i<active; cur=i==active
        circ=f'<div style="width: 28px; height: 28px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: 700; background: {PRIMARY if (cur or done) else SURF}; color: {"#ffffff" if (cur or done) else SOFT}; border: 1px solid {PRIMARY if (cur or done) else BORDER};">{icon("check",14,"#ffffff") if done else i+1}</div>'
        out+=f'<div style="display: flex; align-items: center; gap: 10px;">{circ}<span style="font-size: 13px; font-weight: {700 if cur else 500}; color: {TEXT if cur else SOFT};">{n}</span></div>'
        if i<3: out+=f'<div style="width: 64px; height: 1px; background: {BORDER}; margin: 0 14px;"></div>'
    return out+'</div>'

up=f'''
{steps(0)}
<div style="display: grid; grid-template-columns: minmax(0, 3fr) minmax(0, 2fr); gap: 20px;">
  {card(f'<div style="display: flex; flex-direction: column; gap: 14px;"><div style="border: 2px dashed {BORDER}; border-radius: 12px; height: 240px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; background: {TINT2};">{icon("upload",28,PRIMARY)}<div style="font-size: 15px; font-weight: 600;">Drop the scan export here</div><div style="font-size: 12.5px; color: {SOFT};">CSV up to 12 MB · or</div>{btn("Choose file","secondary")}</div><div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px;"><div style="display: flex; flex-direction: column; gap: 4px;"><div style="font-size: 12px; font-weight: 600; color: {MUTED};">Namespace</div><div style="height: 36px; border: 1px solid {BORDER}; border-radius: 8px; display: flex; align-items: center; justify-content: space-between; padding: 0 12px; font-size: 13px;"><span>certificates</span>{icon("down",14,FAINT)}</div></div><div style="display: flex; flex-direction: column; gap: 4px;"><div style="font-size: 12px; font-weight: 600; color: {MUTED};">Note (optional)</div><div style="height: 36px; border: 1px solid {BORDER}; border-radius: 8px; display: flex; align-items: center; padding: 0 12px; font-size: 13px; color: {FAINT};">e.g. September BitSight export</div></div></div></div>')}
  {card(f'<div style="display: flex; flex-direction: column; gap: 12px;"><div style="font-size: 13px; font-weight: 700;">What happens next</div><div style="font-size: 13px; color: {MUTED}; line-height: 1.6;">The file goes straight to secure storage and is kept as the audit copy. DCops then compares it with the register and shows you exactly what would change before anything changes.</div><div style="display: flex; flex-direction: column; gap: 8px; font-size: 12.5px; color: {SOFT};"><div style="display: flex; gap: 8px;">{icon("check",14,OKG)}<span>Safe to repeat — the same file twice changes nothing</span></div><div style="display: flex; gap: 8px;">{icon("check",14,OKG)}<span>Owners you have assigned are never overwritten</span></div><div style="display: flex; gap: 8px;">{icon("check",14,OKG)}<span>Every import is recorded: who, when, what changed</span></div></div></div>')}
</div>
<div style="display: flex; justify-content: flex-end; gap: 8px;">{btn('Cancel','secondary')}{btn('Upload and review','primary','upload')}</div>
'''
add('ImportUpload', shell('Imports','Import scan',['Certificates','Imports','New import'],up, ns='certificates'))

diffrows=[
 [badge('Created','ok'),mono('new-shop.metro.de'),muted('—'),'expires 14 Jan 2027 · Sectigo',muted('row 4,911')],
 [badge('Updated','info'),mono('www-dev.metro-vto.es'),'expiry 12 Sep 2026','expiry 14 Sep 2026 · issuer unchanged',muted('row 2')],
 [badge('Conflict','bad'),mono('api.metro.cloud'),'endpoint port 443 (set in Git)','port 8443 in scan — not applied',muted('row 118')],
 [badge('Stale','neutral'),mono('old-promo.metro.fr'),'seen 1 Aug','absent from this scan — flagged, kept',muted('—')],
 [badge('Unchanged','neutral'),mono('360.metroag.de'),muted('—'),muted('identical'),muted('row 3')],
]
rev=f'''
{steps(1)}
<div style="display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px;">{stat('312','Created',color=DEEP)}{stat('76','Updated',color='#1d4ed8')}{stat('4,979','Unchanged')}{stat('1','Conflict',sub='owned elsewhere, not applied',color=RED)}{stat('41','Stale',sub='flagged, never deleted')}</div>
<div style="display: flex; align-items: center; justify-content: space-between;">{filterbar([('Show','Changes only')])}<div style="font-size: 12.5px; color: {SOFT};">Detected format: <strong style="color: {TEXT};">headerless scanner v1</strong> · 10 columns mapped · <a>review mapping</a> · 2 warnings</div></div>
{table(['Outcome','Certificate','Before','After','Source'],diffrows)}
<div style="display: flex; align-items: center; justify-content: space-between;"><div style="display: flex; align-items: center; gap: 8px; font-size: 12.5px; color: {SOFT};">{icon('clock',14,SOFT)}<span>This preview expires in 12 min</span></div><div style="display: flex; gap: 8px;">{btn('Back','secondary')}{btn('Continue to confirm','primary')}</div></div>
'''
add('ImportReview', shell('Imports','Import scan',['Certificates','Imports','01J7QZ…'],rev, ns='certificates'))

conf=f'''
{steps(2)}
<div style="display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 20px;">
  {card(f'<div style="display: flex; flex-direction: column; gap: 14px;"><div style="font-size: 15px; font-weight: 700;">Apply this import?</div><div style="font-size: 13px; color: {MUTED}; line-height: 1.6;">312 certificates will be added to the register, 76 will have their scan details refreshed, 4,979 are unchanged, 41 will be flagged as no longer seen. 1 conflict will be skipped. Nothing is deleted and no owner is changed.</div>{kv([("File","certificates.csv · 1.1 MB"),("Uploaded by","Charles · 08:31 today"),("Checksum",mono("sha256 9f1c…e2a0")),("Namespace","certificates")])}<div style="display: flex; gap: 8px;">{btn("Back","secondary")}{btn("Confirm and apply","primary","check")}</div></div>')}
  {card(f'<div style="display: flex; flex-direction: column; gap: 10px;"><div style="font-size: 13px; font-weight: 700;">Audit record</div><div style="font-size: 12.5px; color: {SOFT}; line-height: 1.6;">Confirming records your name and the preview you saw. The original file stays in storage for 400 days; the apply runs in the background and you can watch progress or come back later.</div></div>')}
</div>
'''
add('ImportConfirm', shell('Imports','Import scan',['Certificates','Imports','01J7QZ…'],conf, ns='certificates'))

summ=f'''
{steps(3)}
{card(f'<div style="display: flex; align-items: center; gap: 14px;"><div style="width: 40px; height: 40px; border-radius: 50%; background: {GREENT}; display: flex; align-items: center; justify-content: center;">{icon("check",20,DEEP)}</div><div style="display: flex; flex-direction: column; gap: 3px;"><div style="font-size: 16px; font-weight: 700;">Import completed</div><div style="font-size: 13px; color: {SOFT};">Applied in 48 s · 5,368 rows · confirmed by Charles at 08:36</div></div><div style="flex-grow: 1;"></div>{btn("Assign owners for 312 new certificates","primary","users")}</div>')}
<div style="display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px;">{stat('312','Created',color=DEEP)}{stat('76','Updated',color='#1d4ed8')}{stat('4,979','Unchanged')}{stat('1','Conflict',color=RED)}{stat('41','Stale')}</div>
{section('Conflicts and errors', table(['Outcome','Certificate','Reason'],[[badge('Conflict','bad'),mono('api.metro.cloud'),muted('spec.endpoints is owned by kubectl (Git) — scan value not applied')]]))}
<div style="display: flex; gap: 8px;">{btn('Download full diff','secondary','file')}{btn('View import history','secondary')}</div>
'''
add('ImportSummary', shell('Imports','Import scan',['Certificates','Imports','01J7QZ…'],summ, ns='certificates'))

# ============ Import history
ih=table(['Import','Source file','Uploaded by','When','Created','Updated','Stale','Phase'],[
 [mono('01J7QZ…'),'certificates.csv','Charles',muted('today 08:31'),'312','76','41',badge('Completed','ok')],
 [mono('01J6RA…'),'certificates-aug.csv','Charles',muted('1 Aug'),'5,097','0','0',badge('Completed','ok')],
 [mono('01J6R9…'),'certificates-aug.csv','Charles',muted('1 Aug'),muted('—'),muted('—'),muted('—'),badge('Expired','neutral')],
 [mono('01J5K2…'),'test-export.csv','Ops manager',muted('12 Jul'),muted('—'),muted('—'),muted('—'),badge('Failed','bad')],
])
add('ImportHistory', shell('Imports','Import history',['Certificates','Imports'],f'{ih}{pager("4","4")}', ns='certificates', actions=btn('Import scan','primary','upload')))

# ============ Bulk assign
def field(l,v,dropdown):
    return f'<div style="display: flex; flex-direction: column; gap: 4px;"><div style="font-size: 12px; font-weight: 600; color: {MUTED};">{H.escape(l)}</div><div style="height: 36px; border: 1px solid {BORDER}; border-radius: 8px; display: flex; align-items: center; justify-content: space-between; padding: 0 12px; font-size: 13px; color: {TEXT};"><span>{H.escape(v)}</span>{icon("down",14,FAINT) if dropdown else ""}</div></div>'
bulk_fields=''.join(field(l,v,d) for l,v,d in [("Team","Network",1),("Contact","net-oncall@example.com",0),("Escalate after 14 days to","platform-lead@example.com",0),("Escalate after 7 days to","security@example.com",0),("Bucket","Legacy",1)])
bulk_rows=[[f'<span style="width: 14px; height: 14px; border-radius: 3px; background: {PRIMARY}; display: inline-block;"></span>',mono(h),badge('Critical','bad'),e,muted('METRO France · MIAG')] for h,e in [('authors.metro-vto.es','101 d'),('arbeitgeber-ahd.de','92 d'),('aro.info','47 d'),('www.metro-tr.com','23 d'),('authors.arbeitgeber-ahd.de','101 d')]]
bulk_panel=f'<div style="display: flex; flex-direction: column; gap: 12px;"><div style="font-size: 15px; font-weight: 700;">Assign owner to 312 certificates</div>{bulk_fields}<div style="display: flex; align-items: center; gap: 8px; font-size: 12.5px; color: {SOFT};">{icon("warn",14,"#b45309")}<span>Dry run first: nothing changes until you confirm the result.</span></div><div style="display: flex; gap: 8px;">{btn("Dry run","secondary")}{btn("Assign 312","primary","check")}</div></div>'
bulk=f'''
<div style="display: grid; grid-template-columns: minmax(0, 3fr) minmax(0, 2fr); gap: 20px;">
  <div style="display: flex; flex-direction: column; gap: 14px;">
    <div style="display: flex; align-items: center; justify-content: space-between;">{filterbar([('State','Unowned'),('Severity','Critical'),('Business unit','METRO France')])}<div style="font-size: 12.5px; color: {SOFT};">312 match · <strong style="color: {TEXT};">312 selected</strong></div></div>
    {table(['','Target','Severity','Expires','Business units'],bulk_rows)}
  </div>
  {card(bulk_panel)}
</div>
'''
add('BulkAssign', shell('Certificates','Assign owners',['Certificates','Assign owners'],bulk, ns='certificates'))

# ============ Maturity overview
dist=[('L1','2,829',53,RED),('L2','1,340',25,'#b45309'),('L3','640',12,BLUE),('L4','320',6,BLUE),('L5','239',4,OKG)]
bars='<div style="display: flex; flex-direction: column; gap: 10px;">'+''.join(f'<div style="display: grid; grid-template-columns: 40px minmax(0, 1fr) 70px; gap: 12px; align-items: center; font-size: 13px;"><strong>{l}</strong><div style="height: 14px; border-radius: 4px; background: {TINT};"><div style="height: 14px; border-radius: 4px; background: {c}; width: {p}%;"></div></div><span style="color: {SOFT}; text-align: right;">{n}</span></div>' for l,n,p,c in dist)+'</div>'
byowner=table(['Owner team','Certificates','Critical','Headroom > 30 d','Automated','Level'],[
 ['Platform','1,204','12','98%','91%',badge('Level 4','info')],['Web','891','40','94%','96%',badge('Level 2','info')],['Network','444','118','77%','12%',badge('Level 2','info')],[muted('Unowned'),'2,829','137','81%','74%',badge('Level 1','bad')],
])
mat=f'''
<div style="display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 14px;">{stat('Level 1','Estate maturity',sub='pinned by 2,829 unowned certificates',color=RED)}{stat('47%','Owner coverage',sub='target 100%')}{stat('88%','Renewal headroom > 30 d',sub='target 100%')}{stat('77%','Automation coverage',sub='cert-manager or ACME')}</div>
<div style="display: grid; grid-template-columns: minmax(0, 2fr) minmax(0, 3fr); gap: 20px;">
  {card(f'<div style="display: flex; flex-direction: column; gap: 14px;"><div style="font-size: 13px; font-weight: 700;">Certificates by level</div>{bars}<div style="font-size: 12px; color: {SOFT}; line-height: 1.5;">Based on the CMMI Certificate Management Maturity Scale. A certificate is scored from facts DCops can see: owned, checked, meets the 30-day target, renews itself.</div></div>')}
  {section('By owner', byowner, filterbar([('Group by','Owner team')]))}
</div>
'''
add('CertificatesMaturity', shell('Certificates','Maturity',['Certificates','Maturity'],mat, ns='certificates', actions=btn('Open Datadog dashboard','secondary','link')))

# ============ Docs
toc=''.join(f'<div style="padding: 6px 10px; font-size: 13px; color: {DEEP if a else MUTED}; font-weight: {600 if a else 500}; background: {GREENT if a else "transparent"}; border-radius: 6px;">{t}</div>' for t,a in [('Overview',0),('Installation',0),('Quick Start',1),('GitOps Workflow',0),('IP Address Allocation',0),('PXE Boot Control',0),('NetBox Setup',0),('CRD Reference',0)])
docs=f'''
<div style="display: grid; grid-template-columns: 220px minmax(0, 1fr) 200px; gap: 24px;">
  <div style="display: flex; flex-direction: column; gap: 2px;"><div style="font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: {SOFT}; padding: 6px 10px;">User guide</div>{toc}<div style="font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: {SOFT}; padding: 14px 10px 6px 10px;">Contributor</div><div style="padding: 6px 10px; font-size: 13px; color: {MUTED};">Development</div><div style="padding: 6px 10px; font-size: 13px; color: {MUTED};">Contributing</div></div>
  {card(f'<div style="display: flex; flex-direction: column; gap: 14px; max-width: 720px;"><div style="font-size: 24px; font-weight: 700; letter-spacing: -0.01em;">Quick Start</div><div style="font-size: 14px; color: {MUTED}; line-height: 1.7;">Declare an IP pool and a boot profile in Git, apply the manifests, and watch DCops reconcile them into NetBox and the PXE service. This walkthrough takes about ten minutes on the shared development cluster.</div><div style="font-size: 16px; font-weight: 700; margin-top: 6px;">1. Create a tenant</div><pre style="margin: 0; background: {TINT2}; border: 1px solid {BORDER}; border-radius: 8px; padding: 14px; font-family: {MONO}; font-size: 12px; line-height: 1.6; color: {MUTED};">{PRE_TENANT}</pre><div style="font-size: 14px; color: {MUTED}; line-height: 1.7;">The controller resolves the NetBox API token from the referenced Secret and creates the tenant. Its status shows the NetBox ID once reconciled.</div></div>',28)}
  <div style="display: flex; flex-direction: column; gap: 6px;"><div style="font-size: 11px; font-weight: 600; letter-spacing: 0.04em; text-transform: uppercase; color: {SOFT};">On this page</div>{"".join(f'<div style="font-size: 12.5px; color: {MUTED}; padding-left: {p}px;">{t}</div>' for t,p in [("1. Create a tenant",0),("2. Declare a prefix",0),("3. Add a boot profile",0),("Verify",8),("Next steps",0)])}</div>
</div>
'''
add('Docs', shell('Docs','Quick Start',['Docs','User guide','Quick Start'],docs))

# ============ Empty + error states
empty=f'''
{card(f'<div style="display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; padding: 60px 0;">{icon("cert",36,FAINT)}<div style="font-size: 16px; font-weight: 700;">No certificates yet</div><div style="font-size: 13px; color: {SOFT}; text-align: center; max-width: 420px; line-height: 1.6;">Import the security team’s scan export to build the register. You can re-import at any time; DCops only records what changed.</div>{btn("Import scan","primary","upload")}</div>')}
'''
errbanner=f'<div style="background: {REDT}; border-bottom: 1px solid #f5c2c2; padding: 10px 28px; display: flex; align-items: center; gap: 10px; font-size: 13px; color: {RED};">{icon("warn",16,RED)}<span>The Kubernetes API is not reachable (403 Forbidden from the proxy). Showing data from 4 minutes ago.</span><div style="flex-grow: 1;"></div>{btn("Retry","secondary","refresh")}</div>'
add('EmptyState', shell('Certificates','Certificates',['Certificates'],empty, ns='certificates', actions=btn('Import scan','primary','upload')))
add('ErrorState', shell('Dashboard','Platform overview',['Dashboard'],dash, banner=errbanner, actions=btn('Refresh','secondary','refresh')))

# ---- write files + canvas.json
os.makedirs('out', exist_ok=True)
for n,h in boards.items(): open(f'out/{n}.dc.html','w').write(h)
# Main = Dashboard (entry)
os.replace('out/Dashboard.dc.html','out/Main.dc.html')
pages=[('page-1','Shell & dashboard',['SignIn','Main','ErrorState']),
       ('page-2','NetBox inventory',['IpamPrefixes','IpamPrefixDetail','DcimOverview','DcimDeviceDetail','Tenancy','Extras']),
       ('page-3','Boot & IP pools',['Boot','IpPools']),
       ('page-4','Certificates',['CertificatesInventory','CertificateDetail','CertificatesMaturity','BulkAssign','EmptyState']),
       ('page-5','Scan import',['ImportUpload','ImportReview','ImportConfirm','ImportSummary','ImportHistory']),
       ('page-6','Docs',['Docs'])]
titles={'Main':'Dashboard','SignIn':'Sign in','ErrorState':'Dashboard — API unreachable','IpamPrefixes':'IPAM — Prefixes list','IpamPrefixDetail':'IPAM — Prefix detail','DcimOverview':'DCIM — Inventory','DcimDeviceDetail':'DCIM — Device detail','Tenancy':'Tenancy','Extras':'Extras — Tags','Boot':'Boot','IpPools':'IP Pools','CertificatesInventory':'Certificates — Inventory','CertificateDetail':'Certificates — Record detail','CertificatesMaturity':'Certificates — Maturity','BulkAssign':'Certificates — Bulk assign owners','EmptyState':'Certificates — Empty state','ImportUpload':'Import — 1 Upload','ImportReview':'Import — 2 Review changes','ImportConfirm':'Import — 3 Confirm','ImportSummary':'Import — 4 Summary','ImportHistory':'Import history','Docs':'Docs'}
ab=[]; notes=[]
for pid,pname,files in pages:
    for i,f in enumerate(files):
        col=i%3; row=i//3
        ab.append({'file':f'{f}.dc.html','x':col*(W+120),'y':row*(Hh+160),'w':W,'h':Hh,'title':titles[f],'page':pid})
notes.append({'id':'shell-note','x':0,'y':-170,'w':760,'page':'page-1','text':'Shell: left sidebar replaces the old Docs / Contributor / Admin top tabs; one app for docs, dashboard and operations. Tokens lifted from ui/src (bg #faf9f7, sage #4a5a4c / #2d4a2f, border #e5e3df). Sign-in is OIDC (bearer to dcops-api); reads stay on the read-only Kubernetes client.'})
notes.append({'id':'netbox-note','x':0,'y':-150,'w':760,'page':'page-2','text':'NetBox sections share one generic list + detail pattern (auto-discovered CRDs, CrdTable/CrdDetail evolved). Prefixes and Devices shown as the representative kinds; every other kind uses the same two screens with its own columns.'})
notes.append({'id':'cert-note','x':0,'y':-150,'w':760,'page':'page-4','text':'Certificates: state column derives from CRD conditions (Healthy · Renewal due · Drift found · Failed · Unowned · Stale); no YAML anywhere. Numbers come from the September scan (5,368 rows, 2,829 unowned). Maturity per CMMI scale.'})
notes.append({'id':'import-note','x':0,'y':-150,'w':760,'page':'page-5','text':'Import wizard follows EPIC-CM-002: file goes straight to the bucket via a one-time signed URL; the API only handles metadata; preview → confirm → background apply. Copy is written for non-technical Operations managers.'})
json.dump({'pages':[{'id':p,'name':n} for p,n,_ in pages],'artboards':ab,'annotations':notes,'launch':{'view':'canvas','page':'page-1'}},open('out/canvas.json','w'),indent=1)
print(len(boards),'artboards')
