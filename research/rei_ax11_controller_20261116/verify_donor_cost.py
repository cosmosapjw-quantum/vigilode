#!/usr/bin/env python3
"""Reuse, do not rerun, BASS's measured original AXBI geometry comparison.
Verify saved outputs and timings, preserving producer identity and window scope.
"""
import argparse,csv,hashlib,json,statistics,subprocess
from pathlib import Path

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def need(x,msg):
 if not x:raise ValueError(msg)
def run(unit,scratch,out):
 contract=json.loads((unit/'SOURCE_LOCK.json').read_text())
 need(contract['source_identity_sha256']=='d7676b9309ac9b084499bbf599e803baecd661c80765489be45839ef5a7cf185','DONOR_ORIGINAL_SOURCE_MISMATCH')
 comparison=json.loads((unit/'results/EXACT_COMPARISON.json').read_text())
 need(comparison['status']=='BYTE_EXACT_PASS','DONOR_PARITY_NOT_PASS')
 checked=0
 for c in comparison['comparisons']:
  path=scratch/'runs'/f"{c['case']}_{c['window']}_{c['mode']}"/f"rep{c['rep']}"/c['file']
  need(c['byte_exact'] and sha(path)==c['sha256']==c['reference_sha256'] and path.stat().st_size==c['bytes'],'DONOR_RAW_OUTPUT_NOT_MATCHED')
  checked+=1
 rows=[]
 for path in sorted((scratch/'runs').glob('*/TIMING.csv')):
  with path.open() as file:rows.extend(csv.DictReader(file))
 need(len(rows)==36,'DONOR_TIMING_CENSUS')
 summaries=[]
 for case in ['ZERO','SHEAR_PLUS']:
  for window in ['early','plateau']:
   group={m:[r for r in rows if r['case']==case and r['window']==window and r['mode']==m] for m in ['original','profile_uncached','exact_reuse']}
   need(all(len(v)==3 for v in group.values()),'DONOR_REPETITIONS')
   for r in sum(group.values(),[]):
    need(int(r['new_accepted'])==32 and int(r['new_rhs'])>0 and int(r['new_attempts'])==3*int(r['new_comparisons']),'DONOR_WORK_COUNTS')
   med=lambda m,k:statistics.median(float(r[k]) for r in group[m])
   initial_accepted={int(r['initial_accepted']) for r in sum(group.values(),[])}
   need(len(initial_accepted)==1,'MISMATCHED_DONOR_PARENT')
   summaries.append({'case':case,'window':window,'initial_accepted':next(iter(initial_accepted)),'accepted_per_run':32,'repetitions':3,'producer_original_wall_ms':med('original','wall_ns')/1e6,'producer_instrumented_wall_ms':med('profile_uncached','wall_ns')/1e6,'producer_candidate_wall_ms':med('exact_reuse','wall_ns')/1e6,'producer_instrumented_background_quadrature_share':med('profile_uncached','quadrature_ns')/med('profile_uncached','wall_ns'),'producer_instrumentation_overhead_ratio':med('profile_uncached','wall_ns')/med('original','wall_ns'),'candidate_over_original_wall_ratio':med('exact_reuse','wall_ns')/med('original','wall_ns'),'adaptive_original_ms':med('original','adaptive_ns')/1e6,'output_original_ms':med('original','output_ns')/1e6,'timing_semantics':'quadrature nested inside adaptive/output; do not sum as disjoint CPU categories','unmeasured':['chemistry-only','feedback-only','directory-accounting-only']})
 relevant=['SOURCE_LOCK.json','PROFILE_ALLOCATION.json','OWNER_CONSUMPTION_CONTRACT.json','results/KERNEL_COST_PROFILE.json','results/EXACT_COMPARISON.json','results/TIMING_ALL.csv','results/RESOURCE_SUMMARY.json']
 head=subprocess.check_output(['git','-C',str(unit),'rev-parse','HEAD'],text=True).strip()
 status=subprocess.check_output(['git','-C',str(unit),'status','--short','--',str(unit)],text=True)
 data={'schema':'vigilode.ax11.readonly-donor-cost-consumption.v1','producer':'cosmosapjw-quantum/bass','producer_head':head,'producer_worktree_has_uncommitted_results':bool(status.strip()),'publication_status':'LOCAL_SOURCE_VERIFIED_NO_PUBLIC_ADOPTION_ASSUMED','source_corpus_sha256':contract['source_identity_sha256'],'source_background_sha256':contract['files']['rust/rei_microphysics/src/axbi_background.rs']['sha256'],'raw_output_files_checked':checked,'source_files':{name:sha(unit/name) for name in relevant},'records':summaries,'same_accuracy':'all saved states/gas/photon/ledger/output/cadence files checked byte-exact within producer windows','our_new_native_runs':0,'scope_difference':'BASS plateau starts at accepted200000, independent VigilODE controller probe at199980; do not merge these as identical timed windows','new_full_IVP_speedup_claim':False,'AX11_complete':False,'named_consumer':'REI owner PR377 AX11 measured-cost bottleneck selection'}
 out.write_text(json.dumps(data,indent=2,sort_keys=True,allow_nan=False)+'\n')
 print(json.dumps({'verified_output_files':checked,'rows':summaries},sort_keys=True))
 return data
if __name__=='__main__':
 p=argparse.ArgumentParser()
 for n in ['unit','scratch','out']:p.add_argument('--'+n,type=Path,required=True)
 a=p.parse_args();run(a.unit,a.scratch,a.out)
