$physmath-research-loop $scispace $exa $wolfram

# VigilODE CHEM6-MU01 -> actual nonlinear physical-client handoff

PROJECT=cosmosapjw-quantum/vigilode
DRAFT_PR=https://github.com/cosmosapjw-quantum/vigilode/pull/103
HEAD_BRANCH=research/chem6-signed-lognorm-20261029
BASE_BRANCH=research/sym02-native-affine-20261029
BASE_COMMIT=8357faa5b42724a8be24609d4d2f16aec8a6eb3a
VERIFIED_PREDECESSOR_CODE=0d722b4ed71413926c76bc0037f02f3a6bbdd362
FROZEN_BASS_SOURCE=cc602e209e7deeb511ab2a1f3b2268d55206c43c
FROZEN_SOURCE_BINDING=8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16
UNIT=VIGILODE-CHEM6-MU01
UNIT_STATUS=PASS_SCOPED_ALGORITHMIC_PENDING_INDEPENDENT_REVIEW
STAGES=18/20; R6 independent-author and C9 independent-code reviewer NOT_RUN
ORIGINAL_CORE_DAG=7/23 inherited scoped, delta0, 34 edges
CENTRAL_REI=28/88 delta0; working33/88 is not canonical
PRODUCTION=HOLD

## Read in this order

1. Latest GitHub PR/branch/AGENTS and any parallel changes; do not assume PR103 is latest forever.
2. `research/chem6_signed_lognorm_20261029/THEORY_KO.md`, `REPORT_KO.md`, `STAGE_LEDGER.json`, `RESULTS.json`, `DELIVERY_RECEIPTS.json`, `MANIFEST.json` and `code/`.
3. Prior PR102 `research/sym02_native_affine_20261029/{THEORY.md,ADMISSION.json,NEXT_THREAD_HANDOFF_KO.md}` and exact verified code source `0d722b4e`.
4. PR95 source chemical `research/v8_source_chemical_jacobian_remainder_20261028/theory/THEORY_KO.md`, BASS PR233 chemical-geometry crossblock, PR234 tau row. Keep PR95/99 separate from the PR90–102 cumulative ancestry; do not double cherry-pick.
5. Full original 23-node/34-edge DAG and current-evidence statuses, including Q-LEDGER authority FAIL, Q-API breaking change, Q-GIVENS profile discrepancy, 10 interaction gates, negative-cost evidence, SUNDIALS/optional feature/release limits.

## Current result and exact validation

The source chemical6 Jacobian `J=P+(e^Tz)Q+(Qz)e^T` has a sharp weighted-infinity signed lognorm maximum over a 24-vertex H/He/Li product simplex and independent 12-rate box. For each of 6 rows, 32 offdiagonal sign patterns and 12 rate coefficients, the endpoint maximizing rule gives an exact attainable value. It is a source-topology theorem, not an owner rate/clock admission.

Synthetic dyadic example: exact `mu_sharp=378302368699121663/2305843009213693952`, naive independent-entry `49/256`; reduction of certificate bound ~14.2857%, not solver speedup. Exact Fraction witness row4/vertex23. Python pytest8/8, SymPy original-flux Jacobian and independent four-variable full-corner, 300 random interior samples PASS. Rust1.94.1 standalone8/8 tests, strict Clippy PASS, seven Fraction-from-bits cases PASS, max upper excess8.326672684688674e-16. Saved-data plot5 boxes; actual image inspected. No current BASS30 source execution.

Do not repeat completed campaigns without changed source/assumptions. For local independent reproduction:

```bash
set -euo pipefail
cd /path/to/vigilode
# Obtain the exact current branch HEAD and isolate from dirty WIP.
git fetch origin --prune
HEAD=$(git rev-parse origin/research/chem6-signed-lognorm-20261029)
git worktree add -b validation/chem6-mu01-local-$(date -u +%Y%m%dT%H%M%SZ) ../vigilode-chem6-mu01 "$HEAD"
cd ../vigilode-chem6-mu01/research/chem6_signed_lognorm_20261029/code
# This is a standalone candidate package; its Cargo.lock is not the root workspace lock.
export CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1
cargo +1.94.1 test --offline --all-targets -- --test-threads=1
cargo +1.94.1 clippy --offline --all-targets -- -D warnings
python3 -m pytest -q test_chem6_exact.py
cargo +1.94.1 run --offline --bin chem6_probe > RUST_PROBE.csv
python3 verify_rust.py
```

The historical code execution source is distinguished from the final report/backup commit. A fresh local run must pin its actual code/tree/lock/toolchain and preserve failures. If root workspace auto-membership interferes, use an isolated copy of the standalone code and retain exact hashes; do not mutate root Cargo.lock or silently widen workspace support.

## Next genuine science node

Acquire an actual owner-accepted reduced30/physical33 source capsule and effective photo/recombination rate intervals on a common source-clock slab, with isotope scale binary64 bits, positive H/nb, full geometry corridor, source/evaluation/query rounding and incoming global error. This input is not presently certified. Keep missing fields null, not zero. Before applying the chemical6 mu bound to actual BASS, verify exact source/clock/rate/epoch/physical-domain identity and norm transport from this weighted infinity norm to the existing physical33 Gram norm. The geometry/tau crossblocks in BASS PR233/234 do not complete the full30 Jacobian.

Then derive original-equation vector quartic residual `p'-F(x,p)` (degree8 for quadratic chemical RHS) using Bernstein coefficient intervals with coefficient/time/source rounding. Verify p stays in the product simplex; combine mu and residual via Dini, then test electron event signs and `e^T F` transversality before unique crossing. Actual FastDenseSegments integration is a new, separate native qualification. DOPRI piecewise and singular branches remain distinct.

## Required independent review and return

A different author/context should perform R6/C9 against the actual frozen source equations, all 12 rate-basis matrices, proof of finite signed extrema, the exact Fraction witness, Rust interval arithmetic (including subnormal/overflow/FP modes), and figure raw data. This review is pending, not a new science blocker for exploratory continuation. Record actual reviewer ID and findings. Do not claim `20/20` until these two independent reviews actually run and pass.

On completion return: source/tree SHA; owner capsule identity; exact rate/clock/unit/norm definitions; all input hashes; native test commands and raw exits; independent reviewer findings; first failing/negative results; plot/data hashes; DAG counts (`7/23` inherited vs new scoped nodes); actual Drive/Dropbox ACKs and restore status; branch/PR URL; next executable node. Never force push, merge main, reset correction budgets, reactivate NCP, or promote production without the existing full gates.
