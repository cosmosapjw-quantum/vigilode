import json,statistics,pathlib,hashlib,math
root=pathlib.Path('/tmp/vigilode-adapter-local-followup.1PRDjIEs/final_diagnostics')
out={"schema":"vigilode.adapter-attempt-trace-summary.v1","interpretation":"DOPRI observed h near negative-real linear stability radius supports a stability bottleneck; does not prove nonlinear stages globally stable or eliminate adaptation defects","cases":{}}
radius=3.3065678926349467
for case in (5,12):
 p=root/f'case{case}_attempts.stdout'; rows=[json.loads(x) for x in p.read_text().splitlines() if x.strip()]
 head=rows[0]; a=[x for x in rows if x.get('kind')=='attempt']; accepted=[x for x in a if x['accepted']]
 late=accepted[-500:]
 lam=1e6 if case==5 else 16/(32*math.pi/1024)**4 - 4/(32*math.pi/1024)**2
 # KS lambda is 4/dx² - 16/dx⁴ at Nyquist.
 limit=radius/lam
 out['cases'][str(case)]={'raw_sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'source_model_id':head['model_id'],'attempts':len(a),'accepted':len(accepted),'rejected':len(a)-len(accepted),'clipped':sum(x['clipped'] for x in a),'last_t':a[-1]['t_after'],'target_t':head['t_span'][1],'last_500_accepted_median_h':statistics.median(x['trial_h'] for x in late),'last_500_accepted_median_error_norm':statistics.median(x['error_norm'] for x in late),'negative_real_linear_limit_h':limit,'median_h_over_linear_limit':statistics.median(x['trial_h'] for x in late)/limit,'rhs_sum':sum(x['rhs_delta'] for x in a),'first_rejection':next((x['attempt'] for x in a if not x['accepted']),None)}
(root/'TRACE_SUMMARY.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out['cases'],indent=2))
