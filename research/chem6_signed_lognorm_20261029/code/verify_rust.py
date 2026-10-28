"""Independent Fraction-from-binary64 check of Rust 1.94.1 interval upper results."""
import csv, json
from fractions import Fraction as F
from chem6_exact import sharp_mu, default_fixture, mu_weighted, full_jac


def inputs(case):
    e,w,rates=default_fixture()
    e=list(e);w=list(w);rates=[list(x) for x in rates]
    if case==1:
        w=list(map(F.from_float,[1.0,1.25,1.5,1.0,1.25,1.5]))
    elif case==2:
        a,b,c=map(F.from_float,[0.25,0.125,0.0625])
        e=[a,b,2*b,c,2*c,3*c]
    elif case==3:
        for a in range(12):
            if a not in (0,3,7,11):rates[a][1]=rates[a][0]
    elif case==4:
        rates=[[F(0),F(0)] for _ in range(12)]
    elif case==5:
        rates=[[F.from_float(1.0/(64.0+8.0*a))]*2 for a in range(12)]
    elif case==6:
        rates=[[F.from_float((a+1)/1024.0),F.from_float((a+2)/512.0)] for a in range(12)]
    return tuple(e),tuple(w),tuple(map(tuple,rates))


out=[]
with open('RUST_PROBE.csv',newline='') as stream:
    for line in csv.reader(stream):
        case=int(line[0]);upper=F.from_float(float(line[1]))
        e,w,rates=inputs(case)
        witness=sharp_mu(e,w,rates)
        exact=witness.value
        assert upper>=exact, (case,'UNSOUND_UNDERBOUND',upper,exact)
        assert float(upper-exact)<1e-12, (case,'UNEXPECTED_WIDENING',float(upper-exact))
        assert mu_weighted(full_jac(witness.state,e,witness.rate_endpoints),w)==exact
        out.append({'case':case,'rust_upper':str(upper),'exact':str(exact),
                    'difference':str(upper-exact),'difference_float':float(upper-exact),
                    'row_rust':int(line[2]),'vertex_rust':int(line[3]),
                    'row_exact':witness.row,'vertex_exact':witness.vertex_index,
                    'inclusion':'PASS'})
assert len(out)==7
with open('RUST_FRACTION_AUDIT.json','w') as f:
    json.dump({'schema':'vigilode.chem6.mu01.rust-fraction-audit.v1',
               'cases':len(out),'all_inclusions':True,'provenance':'native rustc1.94.1 vs independent Python Fraction from represented f64 bits',
               'source':'conditional source topology, synthetic declared rates, not BASS30 accepted',
               'rows':out},f,indent=2)
print('RUST_FRACTION_AUDIT_PASS',len(out),'cases',
      'max_excess',max(x['difference_float'] for x in out))
