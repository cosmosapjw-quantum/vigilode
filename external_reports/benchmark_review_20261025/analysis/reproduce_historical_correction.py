#!/usr/bin/env python3
"""Verify two historical reporting corrections without running any solver."""
import hashlib
import json
import math
from pathlib import Path
import statistics

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[2]
B = ROOT / 'external_audits/vigilode_ncp_intensive_120m_v1_20261021'

def source(p):
    return {'path':str(p.relative_to(ROOT)), 'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}

rawpath=B/'ROTATION_TOLERANCE_RESULTS_RECOVERED.jsonl'
rows=[(i,json.loads(l)) for i,l in enumerate(rawpath.read_text().splitlines(),1)]
rows=[(i,r) for i,r in rows if not r['warmup']]
selected=[(i,r) for i,r in rows if r['solver']=='vigilode' and r['rtol']==1e-10]
case=next(x for x in json.loads((B/'harness/cases.json').read_text()) if x['case_id']=='C-rotation-w10')
t=case['t_span'][1];w=case['parameters']['omega']
ref=[math.cos(w*t),math.sin(w*t)]
checked=[]
for i,r in selected:
    e=max(abs(a-z)/(at+r['rtol']*abs(z)) for a,z,at in zip(r['states_final'],ref,r['atol']))
    assert math.isclose(e,r['accuracy']['endpoint_max_scaled_error'],rel_tol=1e-13)
    assert e<=1 and r['accuracy']['accuracy_pass_scaled_le_1']
    checked.append({'line_1_based':i,'row':r,'recomputed_endpoint_reference':ref,'recomputed_endpoint_scaled_error':e})
passes=sum(bool(r['accuracy']['accuracy_pass_scaled_le_1']) for i,r in rows)
assert len(selected)==7 and passes==7 and len(rows)==140
native=[r for i,r in selected]
old=json.loads((B/'WORK_PRECISION_SUMMARY.json').read_text())['C-rotation-w10|vigilode|None|1e-10']
measured=statistics.median(r['timing']['timed_seconds'] for r in native)
process=statistics.median(r['process_wall_seconds'] for r in native)
assert process==old['median_s']
d={'schema_version':1,'scope':'Reaggregation and endpoint arithmetic of historical published records; original files preserved',
 'corrections':[{'id':'rotation-accuracy-prose','original_source':source(B/'REPORT_KO.md'),
 'original_claim':'rotation work-precision 0 pass / 140 measurements',
 'corrected_claim':'7 pass / 140; all seven are VigilODE at rtol=1e-10, atol=[1e-10,1e-10]',
 'evidence_source':source(rawpath),'selector':'warmup==false; subset solver==vigilode && rtol==1e-10',
 'total':140,'passes':7,'fails':133,'selected_rows':checked,
 'verification':'Recomputed analytic endpoint reference and scaled endpoint error for each of seven rows. Full sampled max is inherited from emitted accuracy field; reported maximum equals endpoint value. Original WORK_PRECISION_SUMMARY already has this 7/7 subset, so the error is in report prose / inherited partial aggregate, not a new solver result.'},
 {'id':'native-timing-scope','original_source':source(B/'WORK_PRECISION_SUMMARY.json'),
 'original_median_field':'C-rotation-w10|vigilode|None|1e-10.median_s',
 'original_value_seconds':old['median_s'],'interpretation':'Parent process wall time, including native subprocess overhead',
 'reported_native_solve_scope':'warmed_solver_setup_integration_output_certificate',
 'solve_median_seconds':measured,'process_median_seconds':process,
 'analysis':'Keep both timing scopes distinct. The old work-precision aggregator selects elapsed_seconds or process_wall_seconds, ignoring native timing.timed_seconds. New figure uses native reported solver scope and explicitly describes adapter differences; no equal-error speed ranking.'}],
 'scientific_effect':'Corrects the historical tight-tolerance rotation subset. The rtol=1e-6 failure and latest local oscillatory failures remain unchanged. No current-source production promotion follows.',
 'source_files':[source(rawpath),source(B/'harness/cases.json'),source(B/'harness/analyze_campaign.py')], 'solver_runs':0}
(OUT/'HISTORICAL_CORRECTION.json').write_text(json.dumps(d,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'rotation_passes':passes,'rotation_total':len(rows),'native_solve_median':measured,'native_process_median':process,'solver_runs':0}))
