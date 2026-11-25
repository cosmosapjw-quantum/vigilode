import json, os, pathlib, subprocess
from concurrent.futures import ThreadPoolExecutor
repo=pathlib.Path('/home/cosmosapjw/worktrees/vigilode-csas-20261125')
dest=pathlib.Path(__file__).resolve().parent
def git(*args, cwd=repo):
    return subprocess.check_output(['git','-C',str(cwd),*args],stderr=subprocess.STDOUT)
def parse():
    for block in git('worktree','list','--porcelain','-z').decode().split('\0\0'):
        if not block:continue
        d={}
        for line in block.strip('\0').split('\0'):
            k,_,v=line.partition(' ');d[k]=v
        yield d
# Only use process CWD and open file paths for liveness, not process content.
active={}
for proc in pathlib.Path('/proc').iterdir():
    if not proc.name.isdigit():continue
    try:
        if proc.stat().st_uid!=os.getuid() or int(proc.name)==os.getpid():continue
        paths=[os.readlink(proc/'cwd')]
        # A process may keep an open file in a different worktree.
        try:
            for fd in (proc/'fd').iterdir():
                try:paths.append(os.readlink(fd))
                except OSError:pass
        except OSError:pass
        for v in paths:
            if v.startswith(('/home/cosmosapjw/worktrees/vigilode','/tmp/vigilode','/home/cosmosapjw/vigilode/')):
                active.setdefault(v,set()).add(proc.name)
    except OSError:pass
def inspect(d):
    p=pathlib.Path(d['worktree'])
    d['protected']=str(p) in ['/home/cosmosapjw/vigilode',str(repo)]
    d['active_pids']=sorted({pid for v,pids in active.items() if v==str(p) or v.startswith(str(p)+'/') for pid in pids})
    if not p.is_dir():d['state']='missing';return d
    try:
        d['status']=git('status','--porcelain=v1','-z','--untracked-files=all',cwd=p).decode(errors='replace').split('\0')[:-1]
        d['ignored']=git('ls-files','--others','--ignored','--exclude-standard','--directory','-z',cwd=p).decode(errors='replace').split('\0')[:-1]
        d['refs']=git('for-each-ref','--contains',d['HEAD'],'--format=%(refname)','refs/heads','refs/remotes/origin').decode().splitlines()
        d['size_bytes']=int(subprocess.check_output(['du','-sb',str(p)]).split()[0])
        d['state']='inspectable'
    except Exception as e:d['state']='error';d['error']=str(e)
    return d
with ThreadPoolExecutor(max_workers=4) as e: rows=list(e.map(inspect,parse()))
(dest/'WORKTREES_BEFORE.json').write_text(json.dumps(rows,indent=2)+'\n')
for r in rows:
    print(pathlib.Path(r['worktree']).name,r.get('HEAD','')[:10],r['state'], 'MiB',round(r.get('size_bytes',0)/2**20,1),'dirty',len(r.get('status',[])),'ignored',len(r.get('ignored',[])),'refs',len(r.get('refs',[])),'active',r['active_pids'],'protected',r['protected'], 'locked', 'locked' in r)
    if r.get('status'):print('  dirty',r['status'][:5])
    if r.get('ignored'):print('  ignored',r['ignored'][:8])
