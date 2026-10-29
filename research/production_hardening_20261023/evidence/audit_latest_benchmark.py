import json,hashlib,tarfile,math,statistics,subprocess
from pathlib import Path
root=Path(__file__).resolve().parents[3]
b=root/'research/nonstiff_external_adapters_20261023'
out=root/'research/production_hardening_20261023'
def sha(x):return hashlib.sha256(x).hexdigest()
manifest=json.loads((b/'AUDIT_MANIFEST.json').read_text());issues=[];checked=[]
for e in manifest['files']:
 p=b/e['path'];data=p.read_bytes();ok=len(data)==e['bytes'] and sha(data)==e['sha256'];checked.append({'path':e['path'],'valid':ok})
 if not ok:issues.append('manifest:'+e['path'])
archive_results={};external=[]
for cohort in ['external-comparison','merged-comparison']:
 p=b/'evidence'/cohort;m=json.loads((p/'RAW_MANIFEST.json').read_text());data=(p/'RAW_OUTPUTS.tar.gz').read_bytes();valid_archive=sha(data)==m['sha256'];entries={e['path']:e for e in m['files']};members=[]
 with tarfile.open(p/'RAW_OUTPUTS.tar.gz') as t:
  for f in t.getmembers():
   if not f.isfile():continue
   raw=t.extractfile(f).read();e=entries.get(f.name);ok=e is not None and len(raw)==e['bytes'] and sha(raw)==e['sha256'];members.append({'name':f.name,'valid':ok})
   if not ok:issues.append(cohort+':'+f.name)
   if cohort=='external-comparison' and f.name.endswith('.stdout.json'):external.append((f.name,json.loads(raw)))
 archive_results[cohort]={'archive_sha256_valid':valid_archive,'files_checked':len(members),'manifest_entries':len(entries),'failures':[x for x in members if not x['valid']]}
 if not valid_archive:issues.append(cohort+':archive')
def initial(case,i):
 if case=='logistic':return .05+.01*(i%8)
 if case=='mixed-scales':return 10.**(i%12-8)
 if case=='nonnormal':return float(i%2)
 if case in ['harmonic','rapid','damped']:return 1.-i%2
 return 1./(1+i%8)
def rate(case,i):
 if case=='logistic':return .25+.1*(i%8)
 if case=='large-vector':return .5+.05*(i%8)
 if case=='moderate-stiff':return [1.,30.,300.][i%3]
 if case=='stiff-control':return [1.,1000.,100000.][i%3]
 return [.25,1.,3.,.5][i%4]
def exact(case,i,t):
 y=initial(case,i);r=rate(case,i)
 if case=='logistic':return 1./(1.+(1./y-1.)*math.exp(-r*t))
 if case=='driven':return math.sin(t)+y*math.exp(-r*t)
 if case=='nonnormal':return math.exp(-2*t) if i%2 else 30*(math.exp(-t)-math.exp(-2*t))
 if case in ['harmonic','rapid','damped']:
  a,d=(100.,5.) if case=='rapid' else (10.,1.) if case=='damped' else (1.,.25)
  w=a+d*((i//2)%4);env=math.exp(-3*t) if case=='damped' else 1.
  return env*(math.sin(w*t) if i%2 else math.cos(w*t))
 return y*math.exp(-r*t)
replay=[]
for name,r in external:
 n=r['dimension'];case=r['case'];refdiff=absdiff=normdiff=0.;wrms=0.;valid=r['success'] is True and len(r['times'])==5 and len(r['states'])==5
 for t,ys,refs in zip(r['times'],r['states'],r['reference']):
  valid=valid and len(ys)==len(refs)==n;terms=[]
  for i,(y,provided) in enumerate(zip(ys,refs)):
   ref=exact(case,i,t);valid=valid and math.isfinite(y);refdiff=max(refdiff,abs(provided-ref));err=abs(y-ref);absdiff=max(absdiff,err);normdiff=max(normdiff,err/max(1.,abs(initial(case,i)),abs(ref)));terms.append((err/(r['atol']+r['rtol']*abs(ref)))**2)
  wrms=max(wrms,math.sqrt(math.fsum(terms)/n))
 delta=abs(normdiff-r['max_normalized_error']);valid=valid and refdiff<=5e-14 and delta<=5e-14 and abs(absdiff-r['max_abs_error'])<=5e-14
 replay.append({'path':name,'case':case,'dimension':n,'mode':r['mode'],'valid':valid,'reference_disagreement':refdiff,'reported_normalized_error_disagreement':delta,'max_normalized_error':normdiff,'wrms_recomputed':wrms})
 if not valid:issues.append('replay:'+name)
s=json.loads((b/'evidence/merged-comparison/SUMMARY.json').read_text());rows=s['rows'];ratios=[]
for c in s['common_error_comparisons']:
 cap=c['normalized_error_threshold'];group=[r for r in rows if (r['case'],r['dimension'])==(c['case'],c['dimension']) and r['all_runs_successful'] and r['max_normalized_error']<=cap]
 dp=[r for r in group if r['mode']=='dopri5'];other=[r for r in group if r['mode']==c['comparator']];ok=True
 if dp and other:
  a=min(dp,key=lambda r:r['median_seconds']);z=min(other,key=lambda r:r['median_seconds']);ratio=z['median_seconds']/a['median_seconds'];ok=c['status']=='COMPARABLE' and c['explicit_config']==a['id'] and c['comparator_config']==z['id'] and math.isclose(c['comparator_over_explicit_time'],ratio,rel_tol=1e-14)
 else:ok=c['status']!='COMPARABLE'
 ratios.append({'case':c['case'],'dimension':c['dimension'],'cap':cap,'comparator':c['comparator'],'valid':ok})
 if not ok:issues.append('selection:'+str(c))
identities={}
for commit in ['45037fc24e8f8101605f309e1fdb3a9a4e3df56d','1cf87238ab79ee698621d80bc0a3afd903d2f6ff','fbc3c272fa06b443de148631ea78bfa6a34f1b2b']:
 identities[commit]=subprocess.check_output(['git','rev-parse',commit+'^{tree}'],cwd=root,text=True).strip()
native_diff=subprocess.check_output(['git','diff','--name-only','45037fc2..fbc3c272','--','crates','Cargo.toml','Cargo.lock'],cwd=root,text=True).splitlines()
result={'schema':'vigilode.latest-benchmark-independent-audit.v1','date':'2026-10-23','source_head':'fbc3c272fa06b443de148631ea78bfa6a34f1b2b','method':'Read-only artifact hashes, archived records independently rescored with Python math formulas, selection arithmetic; no solver reruns. This does not regenerate or audit every old proof.','git_trees':identities,'native_or_root_cargo_changes_since_executed_source':native_diff,'published_manifest':{'entries_checked':len(checked),'failures':[x for x in checked if not x['valid']]},'raw_archives':archive_results,'external_numerical_replay':{'records_checked':len(replay),'valid':sum(r['valid'] for r in replay),'max_reference_disagreement':max(r['reference_disagreement'] for r in replay),'max_reported_error_disagreement':max(r['reported_normalized_error_disagreement'] for r in replay),'failed_records':[r for r in replay if not r['valid']]},'comparison_selection_arithmetic':{'checked':len(ratios),'valid':sum(r['valid'] for r in ratios),'failures':[r for r in ratios if not r['valid']]},'issues':issues,'artifact_identity_status':'PASS' if not issues else 'FAIL','scientific_scope':'finite sampled analytic fixtures; cross-cohort timing remains diagnostic_only; broad production remains HOLD'}
(out/'evidence'/'LATEST_BENCHMARK_HASH_REPLAY.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False,indent=2))
