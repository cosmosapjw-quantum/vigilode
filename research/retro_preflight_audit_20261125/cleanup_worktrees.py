"""Remove redundant clean VigilODE checkouts, not branches or unique work."""
import json,os,subprocess,pathlib,tarfile,hashlib
repo=pathlib.Path('/home/cosmosapjw/worktrees/vigilode-csas-20261125')
root=pathlib.Path(__file__).resolve().parent
rows=json.loads((root/'WORKTREES_BEFORE.json').read_text())
def git(*a,cwd=repo,check=True):
    p=subprocess.run(['git','-C',str(cwd),*a],capture_output=True,check=check)
    return p
def active(p):
    for proc in pathlib.Path('/proc').iterdir():
        if not proc.name.isdigit() or int(proc.name)==os.getpid():continue
        try:
            if proc.stat().st_uid!=os.getuid():continue
            paths=[os.readlink(proc/'cwd')]
            try:
                paths.extend(os.fsdecode(x) for x in (proc/'cmdline').read_bytes().split(b'\0'))
            except OSError:pass
            try:
                for fd in (proc/'fd').iterdir():
                    try:paths.append(os.readlink(fd))
                    except OSError:pass
            except OSError:pass
            if any(v==p or v.startswith(p+'/') for v in paths):return proc.name
        except OSError:pass
    return None
def status(p):return git('status','--porcelain=v1','-z','--untracked-files=all',cwd=p).stdout
def ignored(p):return [os.fsdecode(x) for x in git('ls-files','--others','--ignored','--exclude-standard','-z',cwd=p).stdout.split(b'\0') if x]
def cache_only(files):return all(any(x in ('.mypy_cache','__pycache__') for x in pathlib.PurePosixPath(f).parts) for f in files)
records=[];eligible=[];refs=[]
for r in rows:
    p=r['worktree'];reason=None
    if r['protected']:reason='primary_or_current'
    elif p.endswith('vigilode-prod-loop-20261116-w4-v33'):reason='retain_latest_separate_production_track'
    elif r['state']!='inspectable' or 'locked' in r:reason='not_safe_to_remove'
    elif r['active_pids'] or active(p):reason='active_process'
    elif r.get('status') or status(p):reason='unique_dirty_or_untracked'
    elif not cache_only(ignored(p)):reason='noncache_ignored'
    if reason:
        records.append(dict(path=p,head=r['HEAD'],status='retained',reason=reason));continue
    assert pathlib.Path(p).parent in (pathlib.Path('/home/cosmosapjw/worktrees'),pathlib.Path('/tmp')) and pathlib.Path(p).name.startswith('vigilode-')
    assert git('rev-parse','HEAD',cwd=p).stdout.decode().strip()==r['HEAD']
    # Anchor ALL detached candidates, even if currently reachable, so restore
    # does not depend on reflog or future edits of an unrelated branch.
    if 'detached' in r:
        ref='refs/heads/archive/preflight-20261125/'+pathlib.Path(p).name
        git('update-ref',ref,r['HEAD'],'0'*40)
    else:
        ref=r['branch'];assert git('rev-parse',ref).stdout.decode().strip()==r['HEAD']
    refs.append(ref);eligible.append((r,ref))
(root/'WORKTREE_ANCHORS.json').write_text(json.dumps([dict(path=r['worktree'],head=r['HEAD'],ref=ref,original_branch=r.get('branch'),detached='detached' in r) for r,ref in eligible],indent=2)+'\n')
# Preserve ignored data even though all are generated cache files.
archive=root/'obsolete_worktree_caches.tar.gz'
expected={}
with tarfile.open(archive,'w:gz') as t:
    for r,ref in eligible:
        p=pathlib.Path(r['worktree'])
        for f in ignored(p):
            q=p/f
            assert q.is_file() and not q.is_symlink(),q
            arc=p.name+'/'+f;expected[arc]=hashlib.sha256(q.read_bytes()).hexdigest()
            t.add(q,arcname=arc,recursive=False)
with tarfile.open(archive,'r:gz') as t:
    actual={m.name:hashlib.sha256(t.extractfile(m).read()).hexdigest() for m in t.getmembers() if m.isfile()}
assert actual==expected
(root/'CACHE_MEMBERS.json').write_text(json.dumps(expected,indent=2)+'\n')
# Bundle commits not already reachable from the pinned active source.
# Existing named branch refs are retained even when already in that base.
base='a150485a5f998b8fffc72effcabdec9030c77f12'
unique_refs=[ref for r,ref in eligible if git('merge-base','--is-ancestor',r['HEAD'],base,check=False).returncode==1]
if unique_refs:
    bundle=root/'obsolete_worktree_commits.bundle'
    git('bundle','create',str(bundle),*unique_refs,'^'+base)
    verify=git('bundle','verify',str(bundle));(root/'WORKTREE_BUNDLE_VERIFY.txt').write_bytes(verify.stdout+verify.stderr)
    heads={line.split()[1]:line.split()[0] for line in git('bundle','list-heads',str(bundle)).stdout.decode().splitlines()}
    for r,ref in eligible:
        if ref in unique_refs:assert heads.get(ref)==r['HEAD'],(ref,heads.get(ref))
print('Preserved eligible checkout anchors',len(eligible),'unique-to-base refs',len(unique_refs),'cache files',len(expected),flush=True)
# Use standard worktree removal with no force. If Git refuses a checkout
# (e.g. ignored files), leave it intact and report the reason.
for r,ref in eligible:
    p=r['worktree']
    if status(p) or active(p) or git('rev-parse','HEAD',cwd=p).stdout.decode().strip()!=r['HEAD'] or not cache_only(ignored(p)):
        records.append(dict(path=p,head=r['HEAD'],status='retained',reason='changed_since_inventory'));continue
    # Recheck archived ignored bytes immediately before removal.
    if any(expected.get(pathlib.Path(p).name+'/'+f)!=hashlib.sha256((pathlib.Path(p)/f).read_bytes()).hexdigest() for f in ignored(p)):
        records.append(dict(path=p,head=r['HEAD'],status='retained',reason='cache_changed'));continue
    z=git('worktree','remove',p,check=False)
    rec=dict(path=p,head=r['HEAD'],ref=ref,logical_bytes=r['size_bytes'],status='removed' if z.returncode==0 and not pathlib.Path(p).exists() else 'retained',exit=z.returncode,detail=z.stderr.decode(errors='replace'))
    records.append(rec);print(rec['status'],p,rec['detail'][:150],flush=True)
    (root/'WORKTREE_CLEANUP.json').write_text(json.dumps(records,indent=2)+'\n')
(root/'WORKTREE_CLEANUP.json').write_text(json.dumps(records,indent=2)+'\n')
print('RESULT',sum(r['status']=='removed' for r in records),len(records),sum(r.get('logical_bytes',0) for r in records if r['status']=='removed'),flush=True)
