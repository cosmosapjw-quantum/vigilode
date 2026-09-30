#!/usr/bin/env python3
"""Exact rational check of nilpotency premise for native coefficient bits."""
from pathlib import Path
from fractions import Fraction as F
import json,struct,hashlib,argparse
def matmul(a,b):
    return [[sum((x*y for x,y in zip(row,col)),F(0)) for col in zip(*b)] for row in a]
def main():
    ap=argparse.ArgumentParser();ap.add_argument('--coefficients',type=Path,required=True);ap.add_argument('--out',type=Path,required=True);args=ap.parse_args()
    raw=json.loads(args.coefficients.read_text());results={}
    for key in ['alpha','l']:
        a=[[F(struct.unpack('>d',bytes.fromhex(v))[0]) for v in row] for row in raw[key]]
        entries=[{'i':i,'j':j,'hex':raw[key][i][j],'value':float(v)} for i,row in enumerate(a) for j,v in enumerate(row) if j>=i and v]
        power=a
        for _ in range(7):power=matmul(power,a)
        pmax=max(abs(v) for row in power for v in row)
        lower=[[v if j<i else F(0) for j,v in enumerate(row)] for i,row in enumerate(a)]
        plower=lower
        for _ in range(7):plower=matmul(plower,lower)
        assert all(v==0 for row in plower for v in row)
        results[key]={'nonzero_upper_diagonal':entries,'max_leakage':max(abs(e['value']) for e in entries),
            'eighth_power_exact_zero':pmax==0,'eighth_power_max_abs':float(pmax),
            'eighth_power_max_fraction_numerator':str(pmax.numerator),'eighth_power_max_fraction_denominator':str(pmax.denominator),
            'projected_eighth_power_exact_zero':True}
    out={'schema':'vigilode.r3.coefficient_structure.v1','source_commit':'cc2cd041737e7ff543624d1b59893a3b4397369f',
         'coefficient_sha256':hashlib.sha256(args.coefficients.read_bytes()).hexdigest(),'results':results,
         'classification':'certification_premise_failure; no demonstrated macroscopic solver error',
         'scope':'exact arithmetic on native binary64 coefficients, not historical exact method coefficients'}
    args.out.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({k:{z:v[z] for z in ['max_leakage','eighth_power_exact_zero','eighth_power_max_abs','projected_eighth_power_exact_zero']} for k,v in results.items()},indent=2))
if __name__=='__main__':main()
