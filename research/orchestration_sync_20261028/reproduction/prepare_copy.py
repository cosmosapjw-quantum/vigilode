"""Prepare an isolated reproduction layout; never execute scientific workers."""
import argparse,pathlib,shutil,json,hashlib

def main():
 p=argparse.ArgumentParser();p.add_argument('--package',type=pathlib.Path,required=True);p.add_argument('--destination',type=pathlib.Path,required=True);p.add_argument('--hd-source',type=pathlib.Path);args=p.parse_args();src=args.package.resolve();dst=args.destination.resolve()
 if dst.exists():raise SystemExit('Destination must not exist; original package/evidence stays untouched')
 if src==dst or src in dst.parents:raise SystemExit('Destination must be outside source package')
 shutil.copytree(src,dst/'saved_evidence')
 workroot=dst/'execution';root=workroot/'release/research/orchestration_sync_20261028';shutil.copytree(src,root);changes=[]
 helper=root/'helpers/bounded_child.py'
 if helper.exists():
  target=workroot/'helpers/bounded_child.py';target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(helper,target);changes.append('Original REI nested execution layout and external relative helper location recreated with unchanged helper bytes')
 # Metadata-only input location adapter. Frozen scientific code bytes stay unchanged.
 ref=root/'implementation/he_birth/source_reference.json';cert=root/'inputs/HE_ENDPOINT14_SAVED_PLOT_DATA.json'
 if ref.exists() and cert.exists():
  data=json.loads(ref.read_text());assert hashlib.sha256(cert.read_bytes()).hexdigest()==data['certificate_sha256'];data['certificate_path']=str(cert);ref.write_text(json.dumps(data,indent=2)+'\n');changes.append('HE copied source_reference certificate_path redirected to included exact input')
 if args.hd_source:
  raw=args.hd_source.read_bytes();assert len(raw)==2289 and hashlib.sha256(raw).hexdigest()=='0c99fe168235d6d6dac3063093bbbe8b4a2ea15cca53c7e2987763c801fdaefb'
  q=root/'authority/thermal/lamda_hd_20261025.dat';q.parent.mkdir(parents=True,exist_ok=True);q.write_bytes(raw);changes.append('Exact externally supplied HD input materialized in copied layout')
 bindings=root/'units/SHARED_EVENT_CONTRAST_SUPPORT_20261028/INPUT_BINDINGS.json'
 if bindings.exists():
  manifest=json.loads(bindings.read_text())
  for row in manifest['files']:
   rel=row['orchestration_relative_path'];prefix='release/research/orchestration_sync_20261028/'
   if rel.startswith(prefix):rel=rel[len(prefix):]
   source=dst/'saved_evidence'/rel
   assert hashlib.sha256(source.read_bytes()).hexdigest()==row['sha256']
   row['path']=str(source)
  bindings.write_text(json.dumps(manifest,indent=2)+'\n');changes.append('Synthesis copied INPUT_BINDINGS paths redirected to seven unchanged saved-evidence bytes; no producer replay')
 # Original raw remains intact in saved_evidence. Clear only copied attempt outputs
 # needed by create-only wrappers, never worker code or inputs.
 names={'PRIMARY.json','REFERENCE.json','RESULTS.json','RESOURCE_SUMMARY.json','RESOURCE_LEDGER.json','PREEXECUTION.json','primary.resource.json','independent.resource.json','primary.stdout.log','primary.stderr.log','independent.stdout.log','independent.stderr.log','results.json','RUN_INPUT.json','RUN_RECEIPT.json','SCIENCE.receipt.json','SCIENCE.stdout.log','SCIENCE.stderr.log','RENDER_INPUT.json','RENDER.receipt.json','RENDER.stdout.log','RENDER.stderr.log'}
 removed=[]
 for base in [root/'units',root/'implementation']:
  if not base.exists():continue
  for q in base.rglob('*'):
   if q.is_file() and q.name in names:removed.append(str(q.relative_to(root)));q.unlink()
 receipt={'science_executed':False,'original_package_unchanged':True,'saved_evidence':str(dst/'saved_evidence'),'execution_layout':str(root),'metadata_adaptations':changes,'copied_attempt_outputs_cleared':removed,'HD_external_input_supplied':bool(args.hd_source),'rerun_authorization':'New allocation required; no old budget transfer'}
 (dst/'PREPARATION.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt,indent=2))
if __name__=='__main__':main()
