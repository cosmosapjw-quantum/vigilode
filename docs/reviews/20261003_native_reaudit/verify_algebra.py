"""Exact symbolic checks; not a native solver or ODE accuracy certificate."""
import json
import sympy as s
x, dx, w, dw, k, a = s.symbols("x dx w dw k a", real=True)
cn, dn, rc, rd = s.symbols("cn dn rc rd", real=True)
checks = {
 "physical_reconstruction_identity": s.expand((x+dx)**2*(a+dw)-x*x*a-((2*x*dx+dx*dx)*a+(x+dx)**2*dw)) == 0,
 "cofactor_quotient_identity": s.cancel((((-k+a)*cn+rc)*dn-cn*(a*dn+rd))/dn**2 - (-k*cn/dn+(rc-cn/dn*rd)/dn)) == 0,
 "positive_radius_needed": (s.Matrix([[0,0],[1,0]])*s.Matrix([1,0]))[1] == 1,
 "nonzero_diagonal_breaks_nilpotency": s.Matrix([[s.Rational(1,2),0],[1,0]])**2 != s.zeros(2),
}
print(json.dumps({"scope":"exact algebra only", "checks":checks, "passed":sum(checks.values()), "total":len(checks)}, indent=2))
assert all(checks.values())
