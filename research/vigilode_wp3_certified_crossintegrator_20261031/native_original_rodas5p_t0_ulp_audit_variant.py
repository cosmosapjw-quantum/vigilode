#!/usr/bin/env python3
"""Source-frozen native RODAS5P achieved-error auditor (cannot run until native outputs).
Strict input: WP1 original Rust trace, WP2 certified reference bits, actual native JSONL.
This script never synthesizes RODAS8 observations.
"""
import argparse,csv,hashlib,json,math,struct,sys
from fractions import Fraction as F
from pathlib import Path

def paths_for(args):
    root=Path(__file__).resolve().parents[1]
    if args.fixture_root:
        root=Path(args.fixture_root)
    return root

def validate_records(source_log,wp1_summary,wp2_cert,refdir,native_lines):
    sys.path.insert(0,str(Path(source_log).resolve().parent))
    try:
        import DEFECT_CERTIFIER as d
    except ImportError as e:
        raise RuntimeError('WP2_ORIGINAL_CERTIFIER_SOURCE_IS_REQUIRED') from e
    assert hashlib.sha256(Path(source_log).read_bytes()).hexdigest()==d.EXPECTED_SWEEP_SHA256
    assert hashlib.sha256(Path(wp1_summary).read_bytes()).hexdigest()==d.EXPECTED_WP1_SUMMARY_SHA256
    traces,_=d.parse_log(source_log)
    cert=json.loads(Path(wp2_cert).read_text())
    records=[json.loads(t) for t in Path(native_lines).read_text().splitlines() if t.strip()]
    cases={r['name']:r for r in records if r.get('kind')=='WP3_CASE'}
    rows=[];errors=[]
    expected_names=set(d.COHORTS)
    assert set(cases)==expected_names,('MISSING_OR_UNAUTHORIZED_SOURCE',set(cases))
    seen=set()
    for cohort in d.COHORTS:
        baseline=traces[(cohort,2,False)]
        source=d.parse_source(baseline)
        steps=len(baseline['points'])-1
        meta=cases[cohort]
        assert meta['source_bits']==baseline['source_bits'],('SOURCE_BITS_MISMATCH',cohort)
        assert meta['h_bits']==baseline['h_bits'],('H_BITS_MISMATCH',cohort)
        assert meta['steps']==steps
        assert meta['initial_state_bits']==baseline['points'][0]['state'],('CALLER_INITIAL_BITS_MISMATCH',cohort)
        ref=next(x for x in cert['micro_cohorts'][cohort] if x['reference_micro']==64)
        by=Path(refdir,ref['reference_states_name']).read_bytes()
        assert hashlib.sha256(by).hexdigest()==ref['reference_states_sha256']
        width=source['n']*8
        assert len(by)==(steps*64+1)*width
        states=[struct.unpack('>'+('Q'*source['n']),by[i*64*width:(i*64+1)*width]) for i in range(steps+1)]
        error_delta=F.from_float(ref['total_reference_plus_clock_delta_up'])
        for row in (p for p in records if p.get('kind')=='WP3_NATIVE_TRACE' and p.get('name')==cohort):
            idx=int(row['rtol_index']);key=(cohort,idx)
            assert key not in seen;seen.add(key)
            # Failures remain visible and are not silently excluded from denominator.
            if not row['success'] or not row['complete_output']:
                errors.append({'cohort':cohort,'rtol_index':idx,'status':'NATIVE_INCOMPLETE','message':row['message']})
                continue
            assert len(row['time_bits'])==steps+1==len(row['state_bits'])
            assert row['time_bits']==[p['time'] for p in baseline['points']],('CLOCK_LABEL_MISMATCH',cohort,idx)
            # Dense output may round a represented initial state by one ULP.
            # This is not a change of the caller's actual initial condition.
            initial_sample_gap=max(abs(d.rational(v)-d.rational(w)) for v,w in zip(row['state_bits'][0],meta['initial_state_bits']))
            assert initial_sample_gap<=F(1,10**12),('SAMPLED_INITIAL_STATE_DEFECT',cohort,idx)
            maxgap=F(0)
            for yy,rr in zip(row['state_bits'],states):
                assert len(yy)==source['n']
                assert all(math.isfinite(d.bits_to_f(v)) and d.bits_to_f(v)>=-1e-12 for v in yy)
                maxgap=max(maxgap,*(abs(d.rational(a)-d.rational(b)) for a,b in zip(yy,rr)))
            low=max(F(0),maxgap-error_delta);high=maxgap+error_delta
            ns=sorted(row['wall_ns_raw'])
            rows.append({'cohort':cohort,'method':'native_original_RODAS5P_8stage_5th_order',
                'rtol_index':idx,'rtol':d.bits_to_f(row['rtol_bits']),
                'atol':d.bits_to_f(row['atol_bits']), 'status':'SOURCE_MATCHED_COMPLETED',
                'achieved_peak_error_lower':float(low),'achieved_peak_error_upper':float(high),
                'reference_delta_up':float(error_delta),'wall_ns_median':ns[len(ns)//2],
                'wall_ns_raw':ns, 'counters':row['counters']})
    assert len(seen)==len(expected_names)*4,('MISSING_NATIVE_TRACE_RECORDS',len(seen))
    assert rows or errors
    return {'schema':'vigilode.wp3.native_original_rodas5p.v1','complete_runs':len(rows),
      'incomplete_runs':len(errors),'work_precision':rows,'failures':errors,
      'claims':'Native Rust original API source-matched achieved-error work precision; no production switch'}

def main():
    p=argparse.ArgumentParser()
    for x in ['--log','--summary','--certificate','--reference-dir','--native','--out']:p.add_argument(x,required=True)
    p.add_argument('--fixture-root')
    a=p.parse_args()
    d=validate_records(a.log,a.summary,a.certificate,a.reference_dir,a.native)
    Path(a.out).write_text(json.dumps(d,indent=2,sort_keys=True)+'\n')
    print('WP3_NATIVE_VERIFIED',len(d['work_precision']),'SUCCESS',len(d['failures']),'PRESERVED_FAILURES')

if __name__=='__main__':main()
