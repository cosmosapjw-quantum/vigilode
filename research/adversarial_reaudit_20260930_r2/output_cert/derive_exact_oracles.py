from fractions import Fraction as F
import json
from pathlib import Path
u=F(1,2**52); p=1+u; w=1-u
true_epsilon=1-p*w
true_budget=F(1e-320)*(F(1.0)/F(1e-160))**2
result={"oracle":"exact Fraction over supplied binary64 coefficients","m08_true_epsilon":str(true_epsilon),"m08_true_inverse_exceeds_one":1/(p*w)>1,"mixed_true_budget":str(true_budget),"mixed_true_budget_rounded":float(true_budget),"output_five_must_reject":F(5)>true_budget}
print(json.dumps(result,indent=2))
