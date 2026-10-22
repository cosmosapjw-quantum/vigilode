# Frozen adapter completion contract

Read-only framing role requested gpt-6-astra/xhigh; runtime identity remains
NOT_INDEPENDENTLY_OBSERVED. This framing agent is not the final independent
reviewer. Installation/dependency manifests are explicitly authorized by the
owner for isolated adapters; the native dependency graph remains frozen.

## Source and ownership

Native/problem source45037fc24e8f8101605f309e1fdb3a9a4e3df56d and
treeb43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad remain immutable. Existing native
benchmark SHA25650534f6dc5da3c7c123e74c83c2d6d437474a94282decd274711534292586ac1
is reused, as are the 576 original process records and16 ensemble records.
JSON source_commit identifies this benchmark problem source; it does not
identify diffsol/Julia code. Record a distinct adapter_source_commit/tree,
adapter/runner hashes, dependency lock/manifest and executable/runtime hashes.

Lower workers own adapters/diffsol, adapters/julia, and the shared Python
runner/validator/analyzer respectively. Controller owns contracts, provenance,
audit packaging, reports, source freezes and authorized push. No simultaneous
writers to the same files. Never edit native source or the root Cargo graph.
All new build/run outputs are outside the publication worktree in a fresh run
directory; standalone tracked Cargo.lock and Julia Project/Manifest are inputs.

## Methods and dependencies

Diffsol exact0.16.1/faer in its own [workspace], Tsit45 on nine nonstiff families,
BDF on moderate-stiff/stiff-control. No n-by-n Jacobian allocation for large
explicit vectors; small12-dimensional stiff controls may use analytic JVP and
FaerLU with settings disclosed. Measurement profile opt-level3,LTO=false,
codegen-units16, Rust1.94.1. Resolve a standalone lock, then use locked/offline
builds; authorized dependency fetch is preparation, not solve timing.

Julia runtime1.12.7 was observed. Use isolated minimal Project with exact pinned
JSON3 1.14.3, SciMLBase3.58.0, OrdinaryDiffEqTsit5 2.1.5 and
OrdinaryDiffEqRosenbrock2.7.5, resolving from the existing pinned component
Manifest if useful. Tsit5 is explicit on nonstiff families; Rodas5P on the two
stiff controls. Disclose analytic Jacobian/AD/linear solver options. No automatic
switching. Install/precompile in an isolated depot; preserve old environments.
Actual installed package/tree identities must be verified, not inferred from
the pre-existing Manifest. Package failures are retained outside solver timing.

## Protocol and problem parity

One JSON request on stdin, one vigilode.nonstiff-benchmark-run.v1 result on
stdout, diagnostics on stderr. Request includes case,dimension,rtol,atol,
source_commit,max_steps and optional id/mode. Mode is literally diffsol/julia.
Return request identity,end,success,message,positive finite elapsed_seconds,
requested_times,actual times,all states,analytic reference,three error summaries,
truthful work,adapter method/version/options and timing_scope. Unsupported
counters are null/explained, not fabricated zero.

Successful arrays must be5-by-dimension and finite, at exact [0,T/4,T/2,3T/4,T].
Independently verify expected end/time grid/initial state, not just the submitted
requested_times. On failure keep partial observations plus last accepted state/
time and attempted work. Never fill missing states with analytic references.
Preserve one solve history; describe interpolation/saveat, or genuine clipping.

Initial states,rates,frequencies,RHS and references follow frozen vode_adapter.py
and analyze_bench.exact, byte-identical in this publication base. Registered
case/dimension/end: decay8/4,driven8/4,logistic8/10,harmonic8/12,rapid8/2,
damped8/4,mixed-scales12/4,nonnormal8/4,large-vector4096/4 and16384/4,
moderate-stiff12/2,stiff-control12/1. FULL rtols exactly [1e-3,1e-5,1e-7,1e-9],
atol=rtol*1e-3. Do not use the compact/default three-tolerance prose grid.

## Time and stopping contracts

Fresh initial data/problem/solver/workspace construction,integration,callbacks,
rejections/failures and saved-state collection are timed. References,error
analysis and JSON are outside the solver timer. Compilation,dependency setup,
process wall and native solver seconds are distinct.

Julia uses one fixed same-request method/type/shape JIT preparation solve per
process before the measured solve; record its duration and success separately.
This is additional to the campaign warmup. Measured state,counters and cache
are fresh; no numerical-solution reuse. Preparation failure cannot hide measured
failure. The45-second process cap includes imports,preparation,solve and JSON.
Persistent worker/sysimage needs a separately approved lifecycle/timing contract;
do not silently raise the cap or alter JIT policy after seeing results.

Only missing comparator processes are scheduled:96configs,384planned attempts,
seed20261023,randomized groups/modes each round,one warmup andthree timed runs.
One cumulative budget<=7200s,per process<=45s,sequential execution. Original
run_bench.py unconditionally includes completed native/VODE and must not be
rerun unchanged. Add a narrow external-only runner. Preserve process/numerical/
replay failures,timeout,launch errors and NOT_RUN_BUDGET independently.

## Acceptance and audit

Test all-family initial/RHS/reference parity,16384 shape,representative nonstiff/
oscillatory/nonnormal/stiff/vector saved outputs,unknown family,odd oscillator
dimension,invalid tolerance and max-step exhaustion with honest partial state.
Use frozen checked_record plus stricter expected identity/time/initial checks.
Do not relax reference disagreement5e-14 or reported-error replay tolerances.
Every completed warmup andtimed record must succeed and pass replay before
its configuration enters a cost comparison. Error caps1e-6/1e-8 are independent
of requested tolerance; select fastest eligible registered configuration per
method. Common cap does not mean exactly equal achieved error.

Freeze new code before final tests/measurement. Preserve source/tree,commands,
first failures,raw stdout/stderr,PLAN/RECEIPTS/SUMMARY/REPLAY,archives/manifests,
figures,package locks,adapter identities,hardware/resource/thread settings and
Julia preparation policy. Merged IDs map to original cohort/id/path/hash; old
bytes are not rewritten. Baseline/native/VODE/ensemble retain original identity.
Cross-session ratios are local diagnostic comparisons,not paired timing claims.

A fresh implementation-independent read-only reviewer checks actual diff and
raw outcomes before HTTPS push. One approved bounded repair returns to lower
workers and then review. No tolerance widening,no production admission,no
global/uniform-time certificate,no old proof/campaign re-audit. User authorized
push to origin/research/nonstiff-explicit-20261023; require fast-forward and
verify remote tip. Publishing audit evidence is distinct from scientific admission.
