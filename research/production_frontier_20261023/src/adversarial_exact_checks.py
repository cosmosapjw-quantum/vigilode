"""Independent exact-arithmetic counterexamples; no performance measurement."""
import json
from fractions import Fraction as F
from pathlib import Path

def run():
    # A2: RKL2 degree3 at h nu=5/2 on a periodic5x5 nearest-neighbor graph.
    n=5
    def p_action(x):
        y=[F(0)]*(n*n)
        for i in range(n):
            for j in range(n):
                y[i*n+j]=sum((x[((i+di)%n)*n+(j+dj)%n] for di,dj in [(1,0),(-1,0),(0,1),(0,-1)]),F(0))/4
        return y
    delta=[F(0)]*(n*n);delta[0]=F(1)
    p1=p_action(delta);p2=p_action(p1);p3=p_action(p2)
    result=[F(7,12)*x-F(5,8)*u+F(25,24)*v for x,u,v in zip(delta,p1,p3)]
    assert result[1]==-F(5,512)
    assert sum(result)==1
    # B2: finite triangular propagation can amplify initial residual M^(s-1).
    m=10;s=8
    z=[F(1)]
    for _ in range(1,s):z.append(m*z[-1])
    assert z[-1]==10_000_000
    return {'schema':'vigilode.adversarial-exact-checks.v1','arithmetic':'Python fractions.Fraction exact rational',
            'rkl2_degree3':{'grid':'periodic5x5','initial_state':'unit delta','h_nu':'5/2','polynomial':'7/12 I - 5/8 P + 25/24 P^3','mass':str(sum(result)),'nearest_neighbor_value':str(result[1]),'minimum':str(min(result)),'nonnegative':False,'implication':'Spectral diffusion stability alone does not prove positivity.'},
            'nilpotent_propagation':{'stages':s,'subdiagonal':m,'bottom_left_inverse':str(z[-1]),'implication':'Finite termination alone does not bound residual or rounding amplification.'}}

if __name__=='__main__':
    out=run()
    target=Path(__file__).resolve().parents[1]/'results'/'ADVERSARIAL_EXACT_CHECKS.json'
    target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(out,indent=2)+'\n')
    print(json.dumps(out,indent=2))
