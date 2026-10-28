# Compatible external adapters: bounded completion of the nonstiff comparison

Owner authorization: install/develop compatible diffsol/Julia adapters, finish
the missing external comparison, and push for external audit. No NCP or SSH.

Frozen native/problem source: 45037fc24e8f8101605f309e1fdb3a9a4e3df56d,
tree b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad. Publication base:
8c80985e30778df4ba44b2b7085753fa345cf0e2. The already completed local native,
VODE and independent-ensemble evidence is immutable and will be reused under
its original identity and hashes, without rerunning those solves.

Use separate standalone Rust and Julia dependency graphs. Do not modify the
VigilODE root Cargo.toml/Cargo.lock, native implementation, or historical
research. Installation and compilation precede timed execution. Preserve
failures, exact commands, actual package/runtime versions and hashes.

The frozen scientific/execution contract is SCIENTIFIC_CONTRACT.md. Diffsol
0.16.1/faer uses Tsit45 for the nine nonstiff families and BDF for the two
stiff controls. Julia 1.12.7 uses pinned Tsit5 and Rodas5P. Method selection is
explicit, with no automatic switching. Each adapter implements the same
JSON-stdin/stdout model/state protocol and truthful partial-failure behavior.

Run only the missing two comparators: 12 case-dimension groups, four tolerances,
one campaign warmup plus three timed repetitions = 96 configurations and
384 planned process attempts. Random seed20261023; cumulative campaign budget
7200 seconds maximum, 45 seconds per process maximum, sequential measurements.
Julia imports/JIT preparation remain subject to the process timeout; its fixed
internal preparation solve is separate from the campaign warmup and disclosed.

Admit only completed, independently replay-verified configurations to the
predefined sampled normalized-error thresholds1e-6/1e-8. Preserve all failures,
timeouts, budget exclusions, accuracy misses and cost losses. Merged baseline
IDs retain a mapping to original bytes and their execution cohort. Cross-session
cost ratios are diagnostic observations, not paired statistical evidence.

Freeze adapter/runner source commit before final measurements. Retain the native
45037fc2 binary identity separately. A fresh read-only independent review must
precede final publication. One approved focused repair is allowed; no tolerance
widening or new algorithm research. Publication supports external auditing,
not general production admission or global/uniform-time error certification.
