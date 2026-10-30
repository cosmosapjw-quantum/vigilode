#!/usr/bin/env python3
"""Pure-Python independent exact-dyadic test of Rust's TwoSum clock admission.
This is a deterministic 100000-case test, not a proof of all floating hardware.
"""
import json, math, random
from fractions import Fraction

def represented_clock_exceeds(t, h, horizon):
    if not all(map(math.isfinite,(t,h,horizon))):return True
    s=t+h
    if not math.isfinite(s) or s>horizon:return True
    if s<horizon:return False
    shifted=s-t
    return ((t-(s-shifted))+(h-shifted))>0.0

def main():
    r=random.Random(20261030);fail=[];cases=100000
    for idx in range(cases):
        t=0.5*r.getrandbits(53)/(2**53)
        h=0.5*r.getrandbits(53)/(2**53)
        if idx%20==0:
            t=0.25
            h=math.nextafter(0.25,math.inf) if idx%40==0 else math.nextafter(0.25,-math.inf)
        if idx%20==1:t,h=0.5,math.nextafter(0.0, math.inf)
        if idx%20==2:t,h=0.0,0.5
        if idx%20==3:t,h=0.5,0.0
        expected=Fraction(t)+Fraction(h)>Fraction(0.5)
        got=represented_clock_exceeds(t,h,0.5)
        if got!=expected:fail.append({'index':idx,'t':t,'h':h,'expected':expected,'got':got});break
    result={'test':'two_sum_binary64_as_exact_real_clock_horizon','cases':cases,'failures':fail,'status':'PASS' if not fail else 'FAIL','tie_witness_rounded':0.25+math.nextafter(0.25,math.inf),'tie_witness_exact_exceeds':Fraction(0.25)+Fraction(math.nextafter(0.25,math.inf))>Fraction(0.5)}
    print(json.dumps(result,indent=2))
    if fail:raise SystemExit(1)
if __name__=='__main__':main()
