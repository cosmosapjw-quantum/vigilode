#!/usr/bin/env python3
"""Exact rational certificate for a previously grid-only real-spectrum claim.
Not a binary64 arithmetic certificate or a bound on nonnormal/complex spectra.
"""
import argparse
from fractions import Fraction as F
from math import comb
from pathlib import Path
import hashlib
import json


def add(a,b):
    return [(a[i] if i<len(a) else F(0))+(b[i] if i<len(b) else F(0)) for i in range(max(len(a),len(b)))]


def mul(a,b):
    c=[F(0)]*(len(a)+len(b)-1)
    for i,x in enumerate(a):
        for j,y in enumerate(b): c[i+j]+=x*y
    return c


def bernstein(c,n):
    return [sum((c[k]*F(comb(i,k),comb(n,k)) for k in range(min(i,len(c)-1)+1)),F(0)) for i in range(n+1)]


def split(c):
    levels=[c]
    while len(levels[-1])>1:
        a=levels[-1];levels.append([(x+y)/2 for x,y in zip(a,a[1:])])
    return [a[0] for a in levels],[a[-1] for a in reversed(levels)]


def prove(snapshot,mode):
    conv=(lambda x:F(float(x))) if mode=='represented_binary64' else F
    g=conv(snapshot['gamma']);a=[[conv(x) for x in row] for row in snapshot['A']];c=[[conv(x) for x in row] for row in snapshot['C']];b=[conv(x) for x in snapshot['b_code']]
    assert all(a[i][j]==0 and c[i][j]==0 for i in range(8) for j in range(i,8))
    # p = b^T(I-T)^-1, T_ij=(gamma*C_ij+A_ij)w-A_ij.
    ps=[[x] for x in b]
    for j in reversed(range(8)):
        for i in range(j+1,8): ps[j]=add(ps[j],mul(ps[i],[-a[i][j],g*c[i][j]+a[i][j]]))
    assert ps[-1]==[F(1)]
    queue=[(F(0),F(1),[bernstein(p,7) for p in ps],0)]
    leaves=[]; failures=[]
    while queue:
        lo,hi,polys,depth=queue.pop()
        upper=[max(map(abs,p)) for p in polys]
        if max(upper)<=F(25,4) and sum(upper)<=F(1167,50):
            leaves.append({'w':[str(lo),str(hi)],'depth':depth,'absolute_upper':[str(x) for x in upper], 'sum_upper':str(sum(upper))})
        elif depth==12:
            failures.append({'w':[str(lo),str(hi)],'max':str(max(upper)),'sum':str(sum(upper))})
        else:
            halves=[split(p) for p in polys]; mid=(lo+hi)/2
            queue.append((mid,hi,[x[1] for x in halves],depth+1))
            queue.append((lo,mid,[x[0] for x in halves],depth+1))
    leaves.sort(key=lambda x:F(x['w'][0]))
    coverage=not failures and leaves[0]['w'][0]=='0' and leaves[-1]['w'][1]=='1' and all(a['w'][1]==b['w'][0] for a,b in zip(leaves,leaves[1:]))
    global_upper=[max(F(leaf['absolute_upper'][j]) for leaf in leaves) for j in range(8)]
    normal_sum=sum(global_upper)
    assert normal_sum<=F(467,20)
    old_sum=sum((sum(map(abs,p)) for p in ps),F(0))
    def uniform_iterations(bound):
        k=0
        while bound*F(1,2)**k > F(1,16): k+=1
        return k
    cost={'scope':'Conditional equal initial preconditioned residual R=1 and contraction q=1/2 in the proved scalar/normal-NSD regime; iteration-cost guarantee, not wall time.','budget':'1/16','old_coefficient_sum_bound':str(old_sum),'new_uniform_sum_bound':'467/20','old_iterations_per_stage':uniform_iterations(old_sum),'new_iterations_per_stage':uniform_iterations(F(467,20)),'stage_count':8}
    cost['old_total_iterations']=8*cost['old_iterations_per_stage'];cost['new_total_iterations']=8*cost['new_iterations_per_stage']
    return {'normal_operator_global_per_stage_upper':[str(x) for x in global_upper],'normal_operator_sum_upper':str(normal_sum),'normal_operator_admission_bound':'467/20','conditional_cost_example':cost,'coefficient_interpretation':mode,'passed':coverage,'claim':'For w in [0,1], each |p_j(w)| <= 25/4 and 1 <= sum_j |p_j(w)| <= 1167/50.','method':'Exact rational power-to-Bernstein conversion, convex-hull upper bounds, dyadic de Casteljau subdivision. Lower bound uses p_8 identically 1.','max_depth':max(x['depth'] for x in leaves),'leaf_count':len(leaves),'polynomials_power_coefficients':[[str(x) for x in p] for p in ps],'leaves':leaves,'failures':failures}


def main():
    ap=argparse.ArgumentParser();ap.add_argument('--out',required=True);args=ap.parse_args()
    src=Path('fixtures/rodas5p_coefficients_snapshot.json');snap=json.loads(src.read_text());cases=[prove(snap,m) for m in ('decimal_literals','represented_binary64')]
    out={'schema':'vigilode.reaudit.sensitivity-proof.v1','input':str(src),'input_sha256':hashlib.sha256(src.read_bytes()).hexdigest(),'script_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'scope':'Exact scalar transformed linear stage problem; lifts by unitary diagonalization to normal Jacobians with nonpositive real spectrum and norm 2. Floating solve/recurrence rounding and nonlinear drift require separate bounds.','cases':cases,'passed':all(c['passed'] for c in cases)}
    Path(args.out).write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'passed':out['passed'],'cases':[{k:c[k] for k in ('coefficient_interpretation','leaf_count','max_depth')} for c in cases]}));return 0 if out['passed'] else 1


if __name__=='__main__':raise SystemExit(main())
