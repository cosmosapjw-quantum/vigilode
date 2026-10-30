# C9 F01 opt-in source-trial binding prototype: preregistration (2026-10-30 KST)

## Frozen source and non-claims
Parent Draft PR126 (unmerged) exact HEAD `2688a59e0b8ab88120105a6949db68d61a378e71`; original C8 source blob `e6a33b66ba80055cae9439cf683a1715ca77c5a2`, C9 test blob `b6a0f06aba6d5d48d0de056ec9eadde1467904af`. Prior C9 actual native GREEN [40973472003](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/40973472003). C9 separate-author independent `0/1 NOT_RUN`; production HOLD. Current available host contains **no native delegation tool**. This branch is an **internal C9 minimal risk-repair prototype**, NOT an independent review receipt and NOT C10.

## Narrow repair hypothesis and acceptance
F01: C8 research-only `certified_raw_stage_bound(case.k,y,h,stage_k,y_out)` has free source coefficients and no `N01MassActionTrial`. A valid certificate for the wrong model is not authority for the live model. Reuse the existing `N01MassActionOwner::freeze()` opaque trial and `value_only_stage_inputs(trial)`; add an opt-in method that checks represented `time` and `y` against the **same actual owner** at the beginning and end (read-only, no `advance()`); use only the owner-supplied rates/carrier in a **new test-only wrapper** that invokes the UNCHANGED C8 analytical stage residual/bound function. Never accept a free caller `case.k` in this wrapper.

Falsifiers:
1. All-eight-zero trial from fake k=0 gives old research checker `(max_residual=0, Some(0), PROVED...)`; new source-checked true k=(0.5,10,200) returns nonzero original raw-U residual or refuses the domain. Never silently claim fake certificate for live true source.
2. Stale owner epoch, mismatched time (including bit-identity of ±0.0 when useful), shifted initial y, trial from a different owner are rejected; no caller-output mutation on error.
3. Source and source model remain unchanged for exact in-scope positive trials; no Newton/JVP/finite difference introduced in algorithm; do not modify default solver or old C8 tests.
4. Run true native Rust 1.94.1 C8+C9 test plus focused C6/C7/PR91 regressions and strict clippy/fmt on an isolated GitHub-hosted workflow. No false PASS if 0 tests selected. Preserve failed logs and at most one repaired rerun.

**Remaining scientific limits even after positive test:** This wrapper binds one N01 owner, not a generic `OdeProblem` or BASS physical33 callbacks, and `certified_raw_stage_bound` remains test-only. No native 8-stage derivative-free production integration, verified global-IVP error, independent-author C9, or C10.

## Status
DAG VigilODE 7/23, REI 28/88, successor admitted 0/12; C6/C7/C8 each 1/1 scoped, C9 same-assistant technical 1/1 scoped, C9 distinct author 0/1, F01 prototype 0/1 at prereg. Preserve derivative-free causal RHS-only Newton-free homotopy as primary; JVP optional fallback. No main merge, force push, tolerance or source-rate mutation. Stage evidence per Git commit and final archive Google Drive + permitted Dropbox only with actual ACKs.
