#!/usr/bin/env python3
"""Read-only cross-thread replay and latency analysis; never reruns a solver."""
import json, statistics, struct, tarfile
from pathlib import Path
BASE=Path(__file__).resolve().parents[1]
RESULTS=BASE/'results'

def bits(x):
    if isinstance(x,float):return ('f64',struct.pack('!d',x).hex())
    if isinstance(x,list):return [bits(v) for v in x]
    if isinstance(x,dict):return {k:bits(v) for k,v in x.items()}
    return x

def raw_json(directory, suffix):
    archive = directory/'RAW.tar.gz'
    if archive.exists():
        with tarfile.open(archive, 'r:gz') as tar:
            return {m.name: json.load(tar.extractfile(m)) for m in tar.getmembers()
                    if m.isfile() and m.name.endswith(suffix)}
    return {p.name: json.loads(p.read_text()) for p in directory.glob('*'+suffix)}

def main():
    dirs=[RESULTS/'causal_campaign',RESULTS/'causal_campaign_threads2']
    plans=[json.loads((d/'PLAN.json').read_text()) for d in dirs]
    assert plans[0]['source_commit']==plans[1]['source_commit']
    assert plans[0]['binary_sha256']==plans[1]['binary_sha256']
    summaries=[json.loads((d/'SUMMARY.json').read_text()) for d in dirs]
    comparisons=[];replay=[]
    one=raw_json(dirs[0], '_t1.json');two=raw_json(dirs[1], '_t2.json')
    assert len(one)==len(two)==42
    for name in sorted(one):
        a=one[name];b=two[name.replace('_t1.json','_t2.json')]
        assert len(a['runs']) >= len(b['runs']) > 0
        samefields=['success','reaches_end','observation_times','observation_states','counters','candidate_totals','terminal_failure']
        for j,(ra,rb) in enumerate(zip(a['runs'],b['runs'])):
            mismatch=[k for k in samefields if bits(ra[k])!=bits(rb[k])]
            replay.append({'case':a['case'],'method':a['mode'],'rtol':a['rtol'],'repeat':j,'bitwise_fields_match':not mismatch,'mismatching_fields':mismatch})
        ta=[r['elapsed_seconds'] for r in a['runs']];tb=[r['elapsed_seconds'] for r in b['runs']]
        comparisons.append({'case':a['case'],'method':a['mode'],'rtol':a['rtol'],'one_thread_median_seconds':statistics.median(ta),'two_thread_median_seconds':statistics.median(tb),'one_over_two_thread_speed_ratio':statistics.median(ta)/statistics.median(tb),'one_thread_min_max':[min(ta),max(ta)],'two_thread_min_max':[min(tb),max(tb)],'work_equal':a['runs'][0]['counters']==b['runs'][0]['counters'],'one_thread_repeats':len(ta),'two_thread_repeats':len(tb)})
    method_summary=[]
    for method in ['residual-first','causal-forward','causal-picard']:
        rows=[r for r in comparisons if r['method']==method];ratios=[r['one_over_two_thread_speed_ratio'] for r in rows]
        method_summary.append({'method':method,'configurations':len(rows),'ratio_min_median_max':[min(ratios),statistics.median(ratios),max(ratios)],'two_thread_faster_count':sum(r>1 for r in ratios),'two_thread_material_faster_1p2_count':sum(r>=1.2 for r in ratios)})
    output={'schema':'vigilode.causal-cross-thread.v1','source_commit':plans[0]['source_commit'],'source_tree':plans[0]['source_tree'],'binary_sha256':plans[0]['binary_sha256'],'successful_runs':[s['successful_runs'] for s in summaries],'failed_runs':[s['failed_runs'] for s in summaries],'bitwise_replay_pairs':len(replay),'bitwise_replay_passes':sum(r['bitwise_fields_match'] for r in replay),'replay':replay,'same_configuration_latency':comparisons,'method_summary':method_summary,'caveat':'latencies from sequential campaign blocks with3vs2repeats; no statistical significance or universal scaling claim; worker pool setup excluded from solve timers in both cohorts'}
    (RESULTS/'CAUSAL_THREADS_AUDIT.json').write_text(json.dumps(output,indent=2)+'\n')
    print(json.dumps({k:output[k] for k in ['successful_runs','failed_runs','bitwise_replay_pairs','bitwise_replay_passes','method_summary']},indent=2))
    assert all(x['bitwise_fields_match'] for x in replay)
    assert all(x['work_equal'] for x in comparisons)

if __name__=='__main__':main()
