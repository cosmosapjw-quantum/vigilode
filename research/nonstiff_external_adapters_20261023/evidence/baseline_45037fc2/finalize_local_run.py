#!/usr/bin/env python3
"""Attach independent review and seal local artifacts; no integration or builds."""
import pathlib,json,hashlib,os,subprocess
R=pathlib.Path(__file__).resolve().parent
def digest(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
review=json.loads((R/'REVIEW.json').read_text());assert digest(R/'REPORT_KO.md')==review['reviewed_report_sha256']
results=json.loads((R/'RESULTS.json').read_text());results['independent_review']={'decision':review['decision'],'receipt':'REVIEW.json','report_sha256':review['reviewed_report_sha256']};results['review_attachment_script_sha256']=digest(pathlib.Path(__file__));results['final_checkout_checks']='FINAL_CHECKS.json';results['artifact_manifest']='ARTIFACT_MANIFEST.json';(R/'RESULTS.json').write_text(json.dumps(results,indent=2)+'\n')
checks=json.loads((R/'FINAL_CHECKS.json').read_text())
for path,expected in [(checks['original_checkout'],checks['original_state_after']),(checks['source_worktree'],checks['source_state_after'])]:
 for name,cmd in {'head':['git','rev-parse','HEAD'],'tree':['git','rev-parse','HEAD^{tree}'],'branch':['git','branch','--show-current'],'status':['git','status','--porcelain=v1']}.items():
  assert subprocess.check_output(cmd,cwd=path,text=True).strip()==expected[name]
entries={}
for folder,dirs,files in os.walk(R):
 if pathlib.Path(folder)==R:dirs[:]=[d for d in dirs if d not in ['build','matplotlib-cache','__pycache__']]
 for name in files:
  p=pathlib.Path(folder)/name
  if p==R/'ARTIFACT_MANIFEST.json':continue
  entries[str(p.relative_to(R))]={'bytes':p.stat().st_size,'sha256':digest(p)}
for name in ['nonstiff_bench','nonstiff_ensemble_bench']:
 p=R/'build/measurement/examples'/name;entries[str(p.relative_to(R))]={'bytes':p.stat().st_size,'sha256':digest(p)}
manifest={'schema':'vigilode.nonstiff.local-artifact-manifest.v1','source_commit':results['source_identity']['source_commit'],'source_tree':results['source_identity']['source_tree'],'review':review['decision'],'report_sha256':review['reviewed_report_sha256'],'excluded':['manifest self hash','build intermediate/cache files (both measurement binaries included explicitly)','Matplotlib cache','Python bytecode cache'],'files':entries}
(R/'ARTIFACT_MANIFEST.json').write_text(json.dumps(manifest,indent=2)+'\n')
print(json.dumps({'decision':review['decision'],'artifacts_hashed':len(entries),'original_and_source_checkouts_verified':True,'report_sha256':review['reviewed_report_sha256'],'manifest_sha256':digest(R/'ARTIFACT_MANIFEST.json')},indent=2))
