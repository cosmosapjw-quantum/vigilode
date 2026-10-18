# Production readiness continuation follow-up preregistration

## Frozen identity and scope

- Project: VigilODE.
- Continuation: `production-audit-20261018-followup`.
- Branch: `audit/production-readiness-20261018`.
- Frozen starting HEAD: `c3036c63da02ca9b04cd39ccf580cd67d00ccb0c`.
- Governing contract: `research/production_audit_20261018/NEXT_STEPS.json`, the current `CONTINUATION_DAG.json`, and the accepted conditional theorems in `THEOREMS.json` / `docs/reviews/20261018_production_audit/THEORY_KO.md`.
- Preserve user work, `Cargo.toml`, `Cargo.lock`, prior audit inputs and failed evidence. Do not widen tolerances or convert diagnostic/empirical evidence into a certificate.

## Frozen node obligations

- **A01:** outward local packet centers/radii only where arithmetic and descriptor identity are enclosed. Keep `roundoff_enclosed=false` unless every path is independently admitted. Reject unrepresentable sizes and nonfinite requests with typed refusal. Include exact one-mode and high-precision multi-mode cases at intermediate and endpoint times; record actual work.
- **A02:** use T1 only with the actual reconstructed curve, a continuous residual enclosure and a bound on its reconstructed mean. Never substitute candidate mean for reconstructed mean without a proved enclosure. A linear-only sub-scope may be implemented only with explicit typed limits and no broader authority. Preserve state on rejection and retain attempted work.
- **A03:** full packet obligation is to connect damped moments, `exp(-2*kappa*t)` forcing, frame/carrier convention and `kappa -> 0`; do not promote scalar moments alone. An independent bounded sub-scope is authorized: add a forcing-moment consumer for `Q_{r,d}(a,h)=I_{d,r}(a,h)` with finite-domain/order checks, typed refusal when an outward absolute-width gate fails, and attempted-work retention. It must not change packet routing or authority. Falsifiers include unequal indices with positive rate, zero-rate beta limit, rounded product branch boundary, subnormal rate-time product, large-rate width refusal, invalid inputs and overflow.
- **A04:** close interval arithmetic paths only under the declared domain and coefficient-tail premises. Preserve `EstimateOnly` until independent review admits total arithmetic authority and preserve warmed zero-allocation behavior.
- **A05:** certify represented dense curves separately from ODE residual/global error. A global claim requires a continuous interval RHS residual and full-tube one-sided gain. Preserve the witness `y'=-10^6(y-sin(t))+cos(t)+10(y-sin(t))^2`, `y(0)=0`, `[0,1]`, `rtol=1e-8`, `atol=1e-11`, with 101 requested outputs; do not change its acceptance threshold.
- **A06:** retain the reviewed carrier-zero rotating-frame Woodbury scope; do not infer full trajectory timing from algebraic complexity.
- **A07:** matched Robertson campaign uses `rtol=1e-8`, `atol=[1e-11,1e-11,1e-11]`, 101 outputs on `[0,40]`, paired timing and allocator receipts. Accuracy failure remains a failed gate; no speed claim absent same physical accuracy.
- **A08/A09:** remain blocked unless both A04 total arithmetic and A07 comparison conditions pass.
- **A10:** use T2 Bernstein subdivision only after coefficient/residual intervals exist; preserve outward transforms, stopping conditions and refinement cost.
- **A11:** retain only the supplied-box Gray-Scott logarithmic-norm scope; no trajectory-box implication.
- **A12:** issue independent scope-specific decisions, with HOLD retained for every unmet dependency.

## Verification and execution order

Preserve the raw first failure before repair. Run only dependency-relevant focused targets, compiler checks, formatting and required clippy checks. Bind source, input, command, environment and output identities. After bounded implementation and independent review are complete, first publish the source execution commit while this file contains no result section. Then compute frozen input hashes, run the final registered checks, store numeric results and raw logs, append one ledger row, and run `check-research-node.py` and `check-authority-refs.py`. No full workspace test is implied by focused results.

## Stop and claim rules

Classify proof, arithmetic, implementation, input and environment failures separately. Typed HOLD nodes block descendants; independent READY nodes continue. Component PASS closes only its reviewed scope. Overall production promotion, global all-time accuracy and same-accuracy speed remain separate decisions.

## Registered results (appended after EXEC and frozen inputs)

See `INPUTS_SHA256.json` and `RESULTS.json` for source/input identity, exact commands, environment, scope decisions, and numeric campaign outcomes. The registered final runs produced 28/28 core, 76/76 integrator, and 1/1 fair-ab focused tests; workspace all-target check, fmt, and clippy each exited 0. The retained A07 Robertson campaign uses rtol=1e-8, atol=[1e-11,1e-11,1e-11], 101 outputs on [0,40]. Diffsol max-grid WRMS 3.3026992500504733 exceeds the frozen 0.05 accuracy gate; timing remains diagnostic_only and no speed claim is made. Broad production promotion, nonlinear packet trajectory certification, generic all-output accuracy, total Leja arithmetic authority, and matched-accuracy speed remain HOLD.
