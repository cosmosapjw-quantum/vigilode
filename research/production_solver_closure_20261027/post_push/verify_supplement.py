#!/usr/bin/env python3
"""Verify frozen publication and post-push metadata identities, not numerical proof."""
from pathlib import Path, PurePosixPath
import hashlib, json, subprocess, sys, tarfile
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
def digest(data): return hashlib.sha256(data).hexdigest()
def path(root,name):
    p=PurePosixPath(name)
    if not name or p.is_absolute() or '..' in p.parts or p.as_posix()!=name or chr(92) in name:
        raise SystemExit('unsafe path: '+name)
    x=root
    for part in p.parts:
        x=x/part
        if x.is_symlink(): raise SystemExit('symlink: '+name)
    return x
m=json.loads((HERE/'SUPPLEMENT_MANIFEST.json').read_text())
for name,rec in m['files'].items():
    data=path(ROOT,name).read_bytes()
    if len(data)!=rec['bytes'] or digest(data)!=rec['sha256']: raise SystemExit('identity mismatch: '+name)
raw=json.loads((HERE/'PUBLIC_LOGS_MANIFEST.json').read_text())
archive=HERE/'PUBLIC_LOGS.tar.xz'
if digest(archive.read_bytes())!=raw['archive_sha256']: raise SystemExit('archive mismatch')
with tarfile.open(archive,'r:xz') as tf:
    members=tf.getmembers(); names=[x.name for x in members]
    if len(names)!=len(set(names)) or set(names)!=set(raw['members']): raise SystemExit('archive membership mismatch')
    for x in members:
        path(HERE,x.name)
        if not x.isfile(): raise SystemExit('non-file archive member')
        data=tf.extractfile(x).read(); rec=raw['members'][x.name]
        if len(data)!=rec['bytes'] or digest(data)!=rec['sha256']: raise SystemExit('archive member mismatch')
p=json.loads((HERE/'LOCAL_DEVELOPMENT_PLAN.json').read_text()); nodes={n['id']:n for n in p['nodes']}
if len(nodes)!=len(p['nodes']): raise SystemExit('duplicate node')
active=set(); done=set()
def visit(k):
    if k not in nodes or k in active: raise SystemExit('missing dependency or cycle')
    if k in done:return
    active.add(k)
    for d in nodes[k]['dependencies']:visit(d)
    active.remove(k);done.add(k)
for k in nodes:visit(k)
edges={(d,n['id']) for n in p['nodes'] for d in n['dependencies']}
actual=set()
for line in (HERE/'LOCAL_DEVELOPMENT_DAG.mmd').read_text().splitlines():
    if ' --> ' in line:
        a,b=line.strip().split(' --> ');actual.add((a.replace('_','-'),b.replace('_','-')))
if actual!=edges: raise SystemExit('JSON/Mermaid edge mismatch')
for n in nodes.values():
    if 'correction_loops_used' in n and not 0<=n['correction_loops_used']<=n['correction_loops_max']<=3: raise SystemExit('budget mismatch')
r=subprocess.run([sys.executable,str(HERE.parent/'verify_package.py')],cwd=ROOT)
if r.returncode:raise SystemExit(r.returncode)
print(json.dumps({'status':'pass_scoped','scope':'frozen package and post-push identity; not native or release qualification','supplement_files':len(m['files']),'public_log_members':len(raw['members']),'dag_nodes':len(nodes),'dag_edges':len(edges)}))
