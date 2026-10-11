"""Small exact diagnostics for the NEW cached-weight realization only."""
from fractions import Fraction as F
from math import factorial
from pathlib import Path
import json
from causal_weights import CausalWeights, convolve, endpoint_force_coefficients


def main():
    checks = []
    table = CausalWeights.compile(7, 12)
    assert all(table.rows[j][k] == F(factorial(j)*factorial(k), factorial(j+k+1))
               for j in range(8) for k in range(13))
    checks.append({"id": "104_exact_beta_weights", "status": "PASS", "comparisons": 104})
    assert convolve((F(0), F(1)), (F(0), F(1)), table) == (0, 0, 0, F(1, 6))
    checks.append({"id": "causal_not_pointwise_product", "status": "PASS"})
    kernels = [(F(1), F(2), F(3)), (F(2), F(-1), F(4)), (F(3), F(5), F(-2))]
    weights = [F(1, 6), F(1, 3), F(1, 2)]
    forcing = (F(2, 3), F(-3, 7), F(5, 11), F(1, 13))
    mean_kernel = tuple(sum(w*a[j] for w, a in zip(weights, kernels)) for j in range(3))
    individual = [convolve(a, forcing, table) for a in kernels]
    mean_output = tuple(sum(w*a[j] for w, a in zip(weights, individual)) for j in range(7))
    assert convolve(mean_kernel, forcing, table) == mean_output
    checks.append({"id": "new_cached_weight_linear_commutation", "status": "PASS"})
    out = endpoint_force_coefficients(forcing, table)
    assert all(out[j] == sum(forcing[k]*F(factorial(j)*factorial(k), factorial(j+k+1))
                            for k in range(4)) for j in range(8))
    checks.append({"id": "shared_endpoint_weights", "status": "PASS"})
    for args in [(-1, 0), (0, -1), (True, 1), (1.0, 1)]:
        try: CausalWeights.compile(*args)
        except ValueError: pass
        else: raise AssertionError("invalid shape accepted")
    for a,b in [((), (1,)), ((1,), ()), (tuple(range(10)), (1,))]:
        try: convolve(a,b,table)
        except ValueError: pass
        else: raise AssertionError("invalid coefficients accepted")
    checks.append({"id": "invalid_degree_and_shape_rejected", "status": "PASS", "cases": 7})
    result = {"scope": "exact new scalar weight prototype; not native solver validation", "checks": checks,
              "passed": len(checks), "historical_campaign_rerun": False,
              "cost": "O((p+1)*(d+1)) exact scalar operations to compile, no factorial calls in implementation"}
    destination = Path(__file__).with_name('evidence') / 'CAUSAL_WEIGHTS_CHECK.json'
    destination.parent.mkdir(exist_ok=True)
    destination.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({"passed":len(checks),"output":str(destination)}))


if __name__ == '__main__': main()
