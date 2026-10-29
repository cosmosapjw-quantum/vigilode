# C7 preregistration: source-specific scientific correspondence, not production admission

Created 2026-10-30 KST, before the native C7 experiment. Parent PR120 HEAD `f022c4c5593b8d3f1f4ee7ba3efa56fd800dd413`. Research DAG official Original VigilODE **7/23**, central REI **28/88**, successor **0/12 admitted**; N01 C5 **1/1 scoped** and C6 **1/1 native-scoped**; C7 **0/1** before verification. Production **HOLD**. C8/9/10, full-IVP, BASS physical33 and N02 NOT_RUN/DEFERRED.

## Protected objective
Causal, parallelized *derivative-free first* Rosenbrock/Runge–Kutta augmented solver with Newton-free homotopy, secondary JVP. PR117 source-specific Robertson scalar diagonal implicit RK/BE stage is NOT the protected eight-stage RODAS5P raw-U target; PR120 C6 35/35 actual native tests only validate the auxiliary C5 path and prior homotopy/causal regressions, not the scientific eight-stage equivalence.

## Registered exact mathematical discriminant
For Robertson mass-action f=(-k1 a+k2 b c, k1 a-k2 b c-k3 b^2, k3 b^2), `d=z_BE-y`, `tau=h gamma>=0`, `J0=Df(y)`, and an *exact* BE root `d=tau f(y+d)`, the ORIGINAL first RODAS5P raw-U equation requires
  `(I-tau J0)U0=tau f(y)`.
Substituting `U0=d` gives
  `r_rawU0=tau[f(y+d)-f(y)-J0 d]=tau(k2 db dc,-k2 db dc-k3 db²,k3 db²)`.
Thus an exact value-only BE root generally has **nonzero** original RODAS first-stage defect. For dyadic rates k=(1/2,1,1/2), carrier y=(35/64,27/128,31/128), tau=1/4, root z=(1/2,1/4,1/4):
  `r_rawU0=(5/65536,-35/131072,25/131072)`, max |r|=35/131072. This is a falsifier of any direct promotion of the auxiliary certificate. Exact Python Fraction and Wolfram CAS must agree.

## Positive stage-structure claim to test
For identity mass, autonomous f with `1^T f=0`, `1^T J0=0`, and exact strictly lower snapshot `A,C`, if original eight-stage raw-U equations hold, `1^T U_i=0` by induction (since each RHS is mass-neutral). Hence y_new=y+sum_i b_code_i U_i preserves total mass in exact arithmetic. Binary64 raw and native-K coefficient transfer has nonzero registered `D_Gamma,D_alpha,D_output,D_embedded` allowances and roundoff. Do not assert exact byte-identical U vs K or physical true error.

## C7 experiment design
1. Independently verify exact dyadic identity and 0-vs-nonzero discriminator. Sample varied dyadic rates and BE candidate b with exact Fraction reference and negative false-admission counterexamples; archive all failures.
2. Add opt-in **native test only**, no solver behavioral changes: frozen source owner with positive initial state, native sequential direct small-dimension 8-stage step as **oracle**, reconstruct raw-U from the frozen coefficient snapshot and compute ORIGINAL full-space raw-U residual per stage. Record max residual, mass defects, stage count, JVP/direct-work provenance and actual returned raw-vs-K allowance. Confirm the BE root is not silently reused as RODAS stage.
3. Focused GitHub-hosted Rust 1.94.1 actions: compile and execute new C7 native test, C6 six native tests, PR91 29 regression tests, strict clippy+focused rustfmt. Actual CI job and logs are scientific proof of implementation-scope results. If native stage cannot run, report partial C7; no promotion by synthetic standalone.
4. Publish staged GitHub commits, local source+evidence ZIP with SHA/CRC, Google Drive backup and Dropbox backup only with actual provider ACK. If Dropbox connector prohibits binary upload, create Dropbox Paper source/handoff receipt as text backup, clearly marking binary dual backup incomplete.

## Protected boundaries
All N01 main runtime and default facade untouched; no production release, no main merge/force-push, no tolerance relaxation. No unregistered speedups/whole-slab/global bounds. Existing high-entropy PR121 (BASS21 source replay) is separate and not silently imported. Native subagents if exposed, otherwise native_subagent NOT_RUN.
