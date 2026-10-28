import json, pathlib, mpmath as mp, hashlib, datetime
mp.mp.dps = 80
ROOT=pathlib.Path('/tmp/vigilode-adapter-local-followup.1PRDjIEs')
D=ROOT/'final_diagnostics'
res={"schema":"vigilode.adapter-reference-crosscheck.v1","method":"independent Python mpmath 80 decimal digit analytic-formula evaluation against emitted binary64 analytic state; empirical crosscheck, not certified enclosure","mpmath_version":mp.__version__,"observed_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"sources":{},"results":{}}
def read(name):
 p=D/name;res['sources'][name]=hashlib.sha256(p.read_bytes()).hexdigest();return [json.loads(x) for x in p.read_text().splitlines() if x.strip()]
chirp=read('chirp_default.stdout')
mx=0;at=None
for x in chirp:
 if x.get('kind')!='sample':continue
 t=mp.mpf(str(x['t'])); gamma=mp.mpf('0.8'); beta=mp.mpf('1.2'); epsilon=mp.mpf('0.3')
 phase=mp.expm1(beta*t)/beta
 rad=mp.exp(-gamma*t)/mp.sqrt(1+(epsilon/gamma)*(1-mp.exp(-2*gamma*t)))
 omega=mp.exp(beta*t)
 state=(rad*mp.cos(phase), rad*(-gamma*mp.cos(phase)-omega*mp.sin(phase)))
 e=max(abs(float(v)-a) for v,a in zip(state,x['analytic_exact_state']))
 if e>mx:mx=e;at=x['t']
res['results']['chirp']={'grid_points':129,'max_formula_binary64_difference':mx,'at_t':at,'scale':'max component absolute full physical state','reference_uncertainty_proposal':max(1e-12,2*mx),'certification':'empirical'}
gyro=read('gyro_default.stdout');mx=0;at=None
for x in gyro:
 if x.get('kind')!='sample':continue
 t=mp.mpf(str(x['t']));gamma=mp.sqrt(mp.mpf('2.04'));ph=mp.mpf('500')*t/gamma
 state=(mp.sin(ph)/500,(mp.cos(ph)-1)/500,mp.mpf('0.2')*t/gamma,mp.cos(ph),-mp.sin(ph),mp.mpf('0.2'))
 e=max(abs(float(v)-a) for v,a in zip(state,x['analytic_exact_state']))
 if e>mx:mx=e;at=x['t']
res['results']['gyro']={'grid_points':65,'max_formula_binary64_difference':mx,'at_t':at,'scale':'max component absolute full 6D state','reference_uncertainty_proposal':max(1e-13,2*mx),'certification':'empirical'}
p=ROOT/'ANALYTIC_REFERENCE_CROSSCHECK.json';p.write_text(json.dumps(res,indent=2)+'\n');print(json.dumps(res['results'],indent=2))
