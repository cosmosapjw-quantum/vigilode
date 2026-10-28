"""Independent 80-digit Decimal endpoint recomputation of native pilot outputs."""
from decimal import Decimal as D, localcontext
import hashlib
import json
from pathlib import Path

HERE=Path(__file__).resolve().parent
PI=D('3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679')


def sin_cos(x):
    # Absolutely convergent series after high-precision range reduction.
    x=x % (2*PI)
    if x>PI: x-=2*PI
    sine=st=x; cosine=ct=D(1)
    for k in range(1,180):
        st *= -x*x/D((2*k)*(2*k+1)); ct *= -x*x/D((2*k-1)*(2*k))
        sine+=st; cosine+=ct
    return sine,cosine


def main():
    source=HERE/'PILOT_RESULTS.json'
    data=json.loads(source.read_text())
    rows=[]
    with localcontext() as ctx:
        ctx.prec=80
        for row in data['rows']:
            t=D(row['final_time']); beta=D(1.2); gamma=D(0.8)
            epsilon=D(0.3) if row['case']=='equivariant-cubic-chirp' else D(0)
            omega=(beta*t).exp(); phi=(omega-1)/beta
            radius=(-gamma*t).exp()/(1+(epsilon/gamma)*(1-(-2*gamma*t).exp())).sqrt()
            sine,cosine=sin_cos(phi)
            reference=[radius*cosine,radius*(-gamma*cosine-omega*sine)]
            error=max(abs(D(v)-r) for v,r in zip(row['physical_final_state'],reference))
            refdiff=max(abs(D(v)-r) for v,r in zip(row['analytic_binary64_reference'],reference))
            rows.append({'case':row['case'],'method':row['method'],
                'physical_local_atol':row['physical_local_atol'],
                'endpoint_reference_80digits':[str(v) for v in reference],
                'endpoint_error_80digits':str(error),
                'binary64_reference_difference':str(refdiff),
                'endpoint_cap_pass':row['success'] and error<=D('1e-4')})
            assert row['jacobian_builds']==row['jvp_calls']==row['linear_solves']==row['nonlinear_solves']==0
            if row['method']=='declared-chart-dopri5':
                assert row['success'] and error<D('1e-4')
    out={'schema':'vigilode.chart-independent-endpoint-check.v1',
         'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
         'precision_decimal_digits':80,
         'reference_status':'numerically checked; exact binary64 model parameters; not directed interval arithmetic',
         'rows':rows}
    (HERE/'INDEPENDENT_ENDPOINT_CHECK.json').write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps({'status':'PASS','rows':len(rows),'chart_endpoint_pass':sum(x['endpoint_cap_pass'] for x in rows if x['method']=='declared-chart-dopri5')}))


if __name__=='__main__': main()
