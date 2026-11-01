"""WP7 source-independent exact Bernstein degree-elevation lemma witness.
Do not equate finite executable tests with HOL-kernel proof or full WP2 source replay.
"""
from fractions import Fraction as Q
from math import comb, factorial
import random

def degree_elevate(bs):
    m=len(bs)-1
    if m<0: raise ValueError("empty")
    return [bs[0]]+[Q(k,m+1)*bs[k-1]+Q(m+1-k,m+1)*bs[k] for k in range(1,m+1)]+[bs[-1]]

def eval_basis(bs,u):
    m=len(bs)-1
    if not Q(0)<=u<=Q(1):raise ValueError("outside basis domain")
    return sum((bs[k]*comb(m,k)*u**k*(1-u)**(m-k) for k in range(m+1)),Q(0))

def envelope(bs,u):return eval_basis([abs(v) for v in bs],u)

def beta_moment(m,l,j,k):
    if not (0<=l<=m and j>=0 and k>=0): raise ValueError("bad beta indices")
    return Q(comb(m,l)*factorial(l)*factorial(m-l+j+k),factorial(m+j+k+1))

def test_rational_identities():
    rand=random.Random(20261101)
    for _ in range(9):
        b=[Q(rand.randint(-7,7),rand.choice((1,2,3,5))) for k in range(10)]
        e=degree_elevate(b)
        for t in (Q(j,8) for j in range(9)):
            assert eval_basis(e,t)==eval_basis(b,t)
            assert envelope(e,t)<=envelope(b,t)
    assert beta_moment(9,4,2,3)==Q(6,715)
    assert envelope([Q(1)]*10,Q(3,7))==envelope([Q(1)]*11,Q(3,7))==Q(1)
    print("WP7_STANDALONE_DEGREE_ELEVATION_SCOPED_TESTS_PASS")

if __name__=="__main__":test_rational_identities()
