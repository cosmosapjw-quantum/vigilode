# R4 HOM contract — fixed before execution

Source: `1c54194123ee6abc6daa512e8574922f510b4e2c`; previous publication `6598abd0db790162ab26f50bc13943f24a0c7e4a`.
Scope: new directed outward stage certificates, inverse witness identity/trust, finite path sum structure, target binding and HOM-06 total cost. No production edit.

Questions and fixed cases:
1. Native 8-stage sequential target, scalar J=-1, y=1, h=1/16, q=0, zero candidate, y_hat=1,e_hat=0, atol=1,rtol=0. Compare constructor-generated witness with (a) copied witness whose upper matrix is zero, (b) empty inner row. Correct behavior: reject corrupted witnesses, or bounds enclose exact first-stage h/(1+h gamma).
2. Declared strict-lower targets with s in {1,2,4,8,9,16}: gamma=0, J=1,y=1,h=1,q=0, alpha=0,L subdiagonal=1,b picks last stage,btilde=0. Exact K_i=i+1. Compare serial/doubling bounds on the zero candidate. Acceptance: every returned bound encloses this exact integer root. More stages than Rodas5P are API generalization probes, not claims about native 8-stage behavior.
3. Minimal standalone structural doubling candidate uses ceil(log2 s) levels; exact rational matrix products prove the finite Neumann identity on the same s fixtures. Refuse nonpositive or nonfinite radius and structurally invalid matrices. This is a mathematical/code candidate, not production performance evidence.
4. HOM-06 published results are inspected as inherited campaign evidence only. No repeat timing campaign. Report actual work, certificate and q1 scope, serial certificate versus research-only doubling, pool creation cost.

Execution gate: no scientific probe before root sends PREREG_COMMITTED. Cargo build uses shared runtime_r4 lock. Exact Python Fraction reference, native JSON results, stdout/stderr/exit retained. At most one compile repair if harness-only failure; source/runtime problems reported separately. No statistical performance conclusion from this probe.

Commands: `cargo run --offline --locked --manifest-path r4/homotopy/probe/Cargo.toml`; `python3 r4/homotopy/exact_checks.py --native r4/homotopy/native.jsonl --output r4/homotopy/RESULTS.json`.
Claim gate: actual native rows + independent exact oracle required for implementation findings; analytical proof and exact implementation checks remain distinct from throughput claims. Owner proposes; independent reviewer decides.

Pre-execution extension: for diagonal J and U the (s n)-square majorant splits, after permutation, into n independent s-square matrices. Exact Fraction fixtures use s=8, n in {1,2,4}, H_(i,u),(j,v)=delta_uv (u+1)/(16(i-j+1)) for j<i, a_(i,u)=(i+u+1)/128. Compare component-factorized adaptive doubling with full-matrix doubling and triangular substitution exactly. Report storage n*s^2 versus (n*s)^2 and conservative dense arithmetic counts; do not infer actual timing. This extension is deterministic and uses no measured result or oracle output in its design.
