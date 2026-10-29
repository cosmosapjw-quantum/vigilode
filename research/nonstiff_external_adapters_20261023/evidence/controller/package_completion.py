from pathlib import Path
import json,hashlib,tarfile,shutil
r=Path(__file__).resolve().parent;s=Path('/home/cosmosapjw/vigilode-nonstiff-adapters-source-7htbCC/research/nonstiff_external_adapters_20261023');ev=s/'evidence'
def h(p):return hashlib.sha256(p.read_bytes()).hexdigest()
for name in ['REPORT_KO.md','RESULTS.json','EXTERNAL_ADAPTERS.json','CAMPAIGN_ENVIRONMENT.json','PYTHON_LOCK_REUSE.json','python-environment.lock','python-environment-all.lock','FINAL_CHECKS.json','REVIEW.json','EXTERNAL_EXECUTION.json','DEVELOPMENT_EVIDENCE.tar.gz','DEVELOPMENT_MANIFEST.json','ADAPTER_SOURCE_COMMIT.txt','ADAPTER_SOURCE_TREE.txt']:
 p=r/name
 if p.exists():shutil.copyfile(p,s/name)
for name in ['external-comparison','merged-comparison']:
 dest=ev/name;dest.mkdir(exist_ok=False)
 for p in (r/name).iterdir():
  if p.is_file() and not p.name.endswith(('.stdout.json','.stderr.log')):shutil.copyfile(p,dest/p.name)
frozen=r/'frozen-execution';dest=ev/'frozen-execution';dest.mkdir(exist_ok=False)
files=sorted(p for p in frozen.rglob('*') if p.is_file() and 'target' not in p.relative_to(frozen).parts)
with tarfile.open(dest/'RAW_TEST_OUTPUTS.tar.gz','w:gz') as tar:
 for p in files:tar.add(p,arcname=str(p.relative_to(frozen)),recursive=False)
(dest/'RAW_TEST_MANIFEST.json').write_text(json.dumps({'adapter_source_commit':(r/'ADAPTER_SOURCE_COMMIT.txt').read_text().strip(),'archive_sha256':h(dest/'RAW_TEST_OUTPUTS.tar.gz'),'files':[{'path':str(p.relative_to(frozen)),'sha256':h(p),'bytes':p.stat().st_size} for p in files]},indent=2)+'\n')
for name in ['COMMANDS.json','BINARY_IDENTITY.json','preflight/SUMMARY.json','preflight/PLAN.json','preflight/RECEIPTS.json','diffsol-protocol/SUMMARY.json']:
 target=dest/name;target.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(frozen/name,target)
ctrl=ev/'controller';ctrl.mkdir(exist_ok=False)
for name in ['execute_frozen_adapters.py','execute_external_campaign.py','execute_analysis.py','capture_campaign_environment.py','capture_final_checks.py','monitor_once.py','write_completion_report.py','package_completion.py','archive_development.py','EXTERNAL_EXECUTION.json','external-controller.log','external-campaign.stdout.log','external-campaign.stderr.log','frozen-execution-pipeline.log','CAMPAIGN_MONITOR.jsonl','BASELINE_BUNDLE_CHECK.json','BASE_IDENTITY.json','analysis.stdout.log','analysis.stderr.log','ANALYSIS_COMMAND.json','FINAL_CHECKS_COMMAND.json','final-checks.stdout.log','final-checks.stderr.log','report-generation.stdout.log','report-generation.stderr.log','REPORT_GENERATION_COMMAND.json','REPORT_GENERATED_KO.md','REPORT_AMENDMENT.json']:
 p=r/name
 if p.exists():shutil.copyfile(p,ctrl/name)
rev=ev/'independent-review';rev.mkdir(exist_ok=False)
for p in (r/'independent-review').iterdir():
 if p.is_file():shutil.copyfile(p,rev/p.name)
print('Packaged audit evidence; generate AUDIT_MANIFEST after final reviewer decision')
