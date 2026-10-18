# A04 arithmetic research 01 — T2 tail lemma and next execution

Scope: exact Fraction verification of the historical T2 componentwise-tail
claim, its corrected infinity-norm implication, and a current-source proof map.
This is a bounded mathematical research computation, not a Leja measurement or
Rust acceptance run. No Leja code, tolerance, manifest, historical record or
authority is changed. Total Leja authority stays `EstimateOnly`.

Frozen relevant source is the byte content recorded in
`exact_tail_counterexample.py::SOURCE_SHA256`. The observed repository HEAD at
freeze was `6b722a918deb0ab178b8e77090cac1280dbb3051`; unrelated A01 and A05
work may advance that HEAD without changing these bound blobs.

The exact witness is the 4 by 4 lower shift matrix L with subdiagonal 1,
zero diagonal, initial vector e0, and N=2. Compute every term in the exponential
series until L^4=0. Require all of the following without a numerical tolerance:

1. The historical norm inequality holds.
2. The historical componentwise inequality fails at component 3.
3. The uniform infinity-norm componentwise bound holds at all components.
4. The witness, script hash and input source hashes are serialized losslessly.

The failure of the historical lemma does not assert failure of the current
native implementation: its tail is already a uniform scalar bound. This witness
is not a claim that the native stopping rule selects N=2. A new independent
total-output/coefficient oracle and scope-matched independent review remain
required before an authority change.

## Contract-only revision: frozen A04-PROOF-ORACLE-01 execution

This appended contract does not retroactively change the earlier exact-tail
run. Its original preregistration hash and raw execution record remain in
`first_run.stdout.json` and `RUN_RECORD.json`; bytes preceding this appended
section reconstruct that original preregistration. No computation or Rust test
was executed for this contract-only update.

The machine-readable frozen input manifest is
`RESEARCH_DECISION.json::next_action.frozen_oracle_contract`. Every float input
is an explicit IEEE binary64 hex bit pattern, directly or via the named shared
array. The fixed budgets are the existing 1e-8 and 1e-12 values. A fixture is
not replaced after observing a failure. There are 12 action fixtures and four
standalone coefficient fixtures:

| ID | Expected native outcome | Fixed cap/budget |
|---|---|---|
| A04-F01-ZERO-DISTINCT | Ok, zero-input, converged true | 16 / 1e-8 |
| A04-F02-SCALAR-DISTINCT | Ok, scalar, converged true | 16 / 1e-8 |
| A04-F03-NEWTON-DISTINCT | Ok, newton, converged true | 16 / 1e-8 |
| A04-F04-NEWTON-SAME-SIGNED | Ok, newton, converged true | 16 / 1e-8 |
| A04-F05-SUBNORMAL-INPUT | Ok, newton, converged true | 8 / 1e-8 |
| A04-F06-SUBNORMAL-OPERATOR | Ok, newton, converged true | 8 / 1e-8 |
| A04-F07-CANCELLATION-SCALAR | Ok, scalar, converged false | 16 / 1e-8 |
| A04-F08-DEGREE-CAP | Ok, newton, converged false | 1 / 1e-12 |
| A04-F09-MATVEC-OVERFLOW | Err, NonFinite | 8 / 1e-8 |
| A04-F10-NAN-INPUT | Err, NonFinite | 8 / 1e-8 |
| A04-F11-INF-INPUT | Err, NonFinite | 8 / 1e-8 |
| A04-F12-INF-SCALE | Err, NonFinite | 8 / 1e-8 |
| A04-C01-NORMAL | finite coefficient/radius table | degree 8 |
| A04-C02-SUBNORMAL | finite coefficient/radius table | degree 2 |
| A04-C03-CONFLUENT | finite coefficient/radius table | degree 3 |
| A04-C04-ZERO-SCALE | finite coefficient/radius table | degree 0 |

These expectations are frozen falsifiable hypotheses, not execution results.
Unexpected Ok/Err, branch or convergence status is preserved as a contract
failure and is not converted into oracle INCONCLUSIVE. A degree-cap report is
`Ok(report)` with `converged=false`; nonfinite/overflow refusal is typed `Err`.
Every Err attempt is preceded by a successful report on the same workspace so
report preservation and attempted-work accounting can be inspected.

For coefficients, reconstruct c, gamma, and each xi from received binary64
bits as exact Fraction values, then compute `x_i=Q(c)+Q(gamma)*Q(xi_i)` exactly.
There is no intermediate f64 multiplication/addition or re-rounding of x_i.
Action-derived c/gamma/actual xi bits are exported from the native path and
bound to the same run; these derived values are not retuned fixture inputs.

Both coefficient and fused-output reference intervals use rational lower and
upper squared distances from the actual returned binary64 center. For a
reference box [l_i,u_i] and native bound b, compute

    U=sum_i max(abs(l_i-y_i),abs(u_i-y_i))^2,
    L=sum_i max(0,l_i-y_i,y_i-u_i)^2.

The triage is `U<=b^2: PASS`, `L>b^2: FALSIFIED`, otherwise `INCONCLUSIVE`.
Nonfinite/negative b is `INVALID_REPORT`. The scalar coefficient test uses the
same triage in one dimension. An upper-bound failure alone does not prove
under-enclosure. FALSIFIED is the only numerical trigger for code repair;
INCONCLUSIVE only refines the independent oracle.

The initial Fraction order is M=64; refinement orders are 128,256,512,1024,
with hard cap 1024. All comparisons/squares are exact rational. For each
coefficient remainder, assert `M+2>R`. For every active fused k, assert
`M+2+k>B`, where `B=||Q(h)Q(A)||_infty` and active means
`abs(Q(s_k))*||Q(w_k)||_infty>0`. The fused tail is

    sum_active_k |s_k| W_k B^(M+1)/(M+1+k)! /(1-B/(M+2+k)).

This follows because each following term ratio is at most
`B/(M+2+k)<1`. For B=0 the omitted tail is exactly zero. An unmet remainder
domain is `ORACLE_DOMAIN_UNMET`; advance only to the next frozen order. An
INCONCLUSIVE or unmet domain at the cap stays HOLD. Inputs, native budget,
degree cap, native outputs/radii and authority never change during refinement.

The allocator calibration clones three nonempty independent Vec values in the
same thread/allocator measurement machinery and must detect at least three
allocations before any warmed-zero assertion. Calibration failure yields
`ALLOCATION_UNAVAILABLE`, never zero-allocation PASS. After positive control,
prewarmed successful F02/F03/F04 at identical capacity/cache must each observe
zero allocations; refusal/error-string paths are not covered by this claim.
Setup/warmup/calibration work is retained separately. A detected warmed
allocation is `ALLOCATION_NONZERO`; only the calibrated zero observation is
`ZERO_ALLOCATION_PASS_SCOPED`.

This revision leaves Leja source, thresholds and total authority unchanged.
The full A04 decision remains `EstimateOnly/HOLD`; the next step is the
bounded oracle implementation and its recorded execution, followed by an
independent scope-matched decision.


## Recorded A04-PROOF-ORACLE-01 result

Execution commit (`EXEC_SHA`): `9e35f12c85abff34d85185f866180b616a8ba208`. The contract and harness were committed before this rerun. The first post-EXEC attempt used a relative output path and failed with exit 101 because the Rust test process resolves it from the crate root; the raw log is `research/production_audit_20261018/evidence/continuation_a04/proof_oracle_01/post_exec_9e35f12/rust_test.log`. The same frozen command with an absolute output path passed (exit 0, 1 Rust test), followed by the Fraction oracle (exit 0, 218 exact comparisons PASS at M=64).

The allocation positive control observed 3 allocations before the warmed F02/F03/F04 observations of 0 each. This is scoped only to those successful calls. Total A04 authority remains `EstimateOnly/HOLD`.

The frozen `inputs_sha256` at EXEC_SHA and recorded `outputs_sha256` are in `research/production_audit_20261018/evidence/continuation_a04/proof_oracle_01/post_exec_9e35f12/EXECUTION_IDENTITY.json`.


Independent read-only receipt review returned `READY_SCOPED` for this post-EXEC run and L-0136 only. It confirmed 11 EXEC_SHA input hashes, 9 output hashes, 218 exact comparisons, and allocation 3 then warmed 0/0/0. The reviewer recorded the initial relative-output-path literal as unavailable in its first record and did not infer it; the first failure classification and successful absolute-path retry remain preserved. Full A04 remains EstimateOnly/HOLD. Review artifact: `evidence/continuation_a04/proof_oracle_01/post_exec_9e35f12/INDEPENDENT_REVIEW.json`.
