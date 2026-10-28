"""New tiny exact-algebra diagnostics; no native or prior campaign execution."""
from fractions import Fraction as F
from math import factorial
from pathlib import Path
import json

B,A,C,T,E,eta = F(3,4),F(9,32),F(1,8),F(1,2),F(5,3),F(4)
mapb = A*E + eta*C*E*T*B**3
qeta = 3*eta*C*E*T*B**2
q1 = qeta/eta
p,q = 8,10
tau = F(1,2)**(p+1)/factorial(p+1)
tail = mapb/eta**(q+1)
D = (1+q1)*tail
H = A*tau+C*B**3*T*tau/(p+2)+D
alpha = 3*C*B**2*T
expupper = 1/(1-alpha)
mode = tau*(F(11,32)+C*B**3*T/F(p+2))+(expupper-1)*H
assert mapb==F(165,256) and qeta==F(45,64) and mapb<B and qeta<1
assert mode<F(1,40_000_000)
pi = [F(1,2),F(1,3),F(1,6)]
def mat(a,d):
    return [[d*(F(i==j)+(a-1)*pi[j]) for j in range(3)] for i in range(3)]
def mul(A,B):
    return [[sum(A[i][k]*B[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
assert mul(mat(F(5,4),F(2,3)),mat(F(7,5),F(3,4))) == mat(F(7,4),F(1,2))
for r in range(5):
    for d in range(5):
        val = sum(F((-1)**j*factorial(r+j),factorial(r+j+1))*F(factorial(d),factorial(j)*factorial(d-j)) for j in range(d+1))
        assert val == F(factorial(r)*factorial(d),factorial(r+d+1))
out = {'scope':'new tiny exact-rational derivation diagnostics, no imported scientific campaign rerun',
       'checks':{'admissible_map':str(mapb),'q_eta':str(qeta),'q_one':str(q1),'rounding_margin':str(B-mapb),
                 'kernel_order':p,'homotopy_order':q,'uniform_kappa_mode_bound':str(mode),
                 'uniform_kappa_mode_bound_decimal':float(mode),'strict_bound':'< 1/40000000',
                 'rank_one_transport_product':'PASS','mixed_phi_zero_damping_limits':'25 exact identities PASS'},
       'claim_ceiling':'diagnostic algebra, not native implementation or wall time'}
Path(__file__).with_name('DAMPING_EXACT_CHECKS.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
