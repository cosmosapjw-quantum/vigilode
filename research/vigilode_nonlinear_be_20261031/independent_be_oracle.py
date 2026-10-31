#!/usr/bin/env python3
"""Independent original-source physical-error replay of executed Rust BE traces.

Exact rational numbers reconstruct each binary64 input, result and bound. This
script imports NO VigilODE solver source, uses NO solver intermediate residuals
as authority, and recomputes BE physics & whole trajectory error recurrence.
An 80-digit analytic solution supplies a separate numerical cross-check.
"""
import json
import struct
import sys
from fractions import Fraction as Q
from decimal import Decimal, localcontext

def readbits(bits):
    return Q.from_float(struct.unpack('>d',struct.pack('>Q',int(bits)))[0])
def dec(q):return Decimal(q.numerator)/Decimal(q.denominator)

def inspect(row):
    k,g,Y=(readbits(x) for x in row['source_bits'])
    h=readbits(row['h_bits']);eps=readbits(row['epsilon_bits'])
    assert k>0 and g>=0 and Y>0 and h>0 and eps>0
    data=row['steps'];n=len(data)-1
    assert n>0 and n&(n-1)==0
    assert readbits(data[0][1])==Y
    E=Q(0);M=Y;out=[];a=1+h*k;b=h*g
    P0=k*Y+g*Y**3
    total_rhs=sum(1 for _ in data)
    for i in range(1,len(data)):
        lab,y,bound,res,q_up=[readbits(z) for z in data[i]]
        old=readbits(data[i-1][1]);assert 0<=y<=old
        R=a*y+b*y**3-old
        assert abs(R)<=res,('original source residual enclosure',row['name'],i,float(abs(R)),float(res))
        u=old/a
        q=3*b*u*u/a
        assert q<=q_up and q_up<1,('nonlinear q',i)
        assert 3*b*u*u/a < 1 and u-b*u**3/a>=0
        L=k+3*g*M**2;P=k*M+g*M**3
        D=h*h*L*P/2
        E=(E+D+abs(R))/a
        delta=abs(lab-i*h)*P0
        cert=E+delta
        assert cert<=bound,('whole-IVP error enclosure',i,float(cert),float(bound))
        assert bound<=eps,('physical budget',i,float(bound),float(eps))
        with localcontext() as ctx:
            ctx.prec = 80
            tt=dec(lab)
            exp1=(-dec(k)*tt).exp()
            exact=dec(Y)*exp1/(Decimal(1)+(dec(g)/dec(k))*dec(Y)**2*(Decimal(1)-exp1**2)).sqrt()
            actual=abs(dec(y)-exact)
            assert actual <= dec(bound)+Decimal("1e-60"),("analytic root outside bound",i)
        out.append(float(actual))
        M/=a
    assert row['total_rhs']==row['interval'] and row['total_rhs']>=n
    return {'name':row['name'],'fast':row['fast'],'steps':n,'callbacks':row['total_rhs'],
            'maximum_numerical_physical_error':max(out),'epsilon':float(eps)}

def main(path):
    rows=[]
    for line in open(path,encoding='utf-8'):
        if 'NLB_TRACE=' in line:
            data=json.loads(line.split('NLB_TRACE=',1)[1].strip())
            rows.append(inspect(data))
    assert len(rows)==6,f'Expected 6 source-matched native traces, got {len(rows)}'
    s={(r['name'],r['fast']) for r in rows}
    assert len(s)==6
    print('INDEPENDENT_ORIGINAL_SOURCE_BE_6_OF_6_PASS')
    for r in rows:print('ORACLE_RESULT',json.dumps(r,sort_keys=True))
if __name__=='__main__':
    if len(sys.argv)!=2:sys.exit('usage: independent_be_oracle.py native_benchmark.log')
    main(sys.argv[1])
