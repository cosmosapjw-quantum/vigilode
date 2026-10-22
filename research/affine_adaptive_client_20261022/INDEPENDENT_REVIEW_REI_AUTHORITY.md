# Independent N1 review

Reviewer: /root/rei_authority, independent of N1 theory author and implementing
coder. Date2026-10-22UTC /2026-10-23KST. Decision:
**PASS_SCOPED_AFFINE30_BDF_METRIC_HOOK_AND_FINITE_PAIR**.
The work-count portion of the original reporting contract remains incomplete
because unflushed counters were lost; it is not silently promoted to complete.

I inspected CONTRACT.md, AUTHORITY_INTAKE.json, affine_system.py,
lifted_bdf.py, C norm source/header, run_pair.py, recover_receipt.py,
inherited_gates.py, the pinned SciPy/owner sources, preserved failures,
EXECUTION_LEDGER.json, transactional test and recovered raw pair receipt.
No new trajectory or old diagnostic was run for this review.

The affine map retains the prescribed conserved totals. The reduced RHS
is R F(d+Lu), and the Jacobian is R J L. The C implementation reconstructs
the three omitted tangent errors as negative charged-state sums and divides
the squared length by33. In the BDF subclass, Newton corrections use lifted
predictor scales; LTE and order-minus/order-plus candidates use the full33
endpoint scales. The driver selects the initial step on the full33 owner and
passes it to both solvers. There is no native30 norm substitution or global
monkeypatch. The runtime explicitly binds the SciPy1.17.0 source hash.

The private D copy prevents partial history publication. On NormFailure,
t,y,D,h_abs,J,LU,order,n_equal_steps are restored. In particular, the error
handler covers the order-adaptation call after temporary accepted-state
assignments. The retained injected status1 test observes rollback and zero
accepted steps at this late call. Counters are observations of attempted
work, so retaining their increments does not violate accepted-state rollback.

I independently read the original saved NPZ and recomputed the nine full33
WRMS differences using numpy.linalg.norm divided by sqrt(33), with the
frozen endpoint scales. The maximum is7.222972526677901e-8, below1; the
largest receipt difference from summation association is6.617444900424222e-24.
The raw sample shape is9x33, x exactly equals linspace(0,.01,9), retained
coordinates equal the reduced state exactly, and reconstructed elemental
totals have zero observed drift. Saved full62 final species match the receipt.
The raw file SHA256 is
`000901168907f753b73e08b8a740d6147c68fcd9a850046d6843dd3b5338c287`.
This read-only array check exited0 in0.833353s; it used no solver callbacks.

All13 reported inherited gates are true for each trajectory. I compared their
expressions and thresholds against compact_run.py: positivity, isotope and
baryon conservation, electrons, monopoles, optical depth, Einstein constraint,
tensor sum, angular positivity, trace energy, initial axes, and endpoint/grid
conditions remain inherited. The diagnostic maximum absolute full33 difference
is6.661338147750939e-16. Physical HOLD and the prescribed-T/UV test-fluid
assumptions remain explicit. No speed or broad physical accuracy claim follows.

The first real pair saved its raw arrays and then failed JSON serialization
on NumPy scalar metadata. Recovery uses the original NPZ, a separately
persisted manufactured transaction check, and the unchanged owner metadata
converter. The failures are retained. Exactly2 short cosmological IVPs form
one matched pair; recovery adds0. The first-step numeric value and detailed
RHS/Jacobian/LU/step/rejection/norm counts are UNKNOWN, as explicitly reported.
Their absence limits work/performance reporting, not the preserved pair's
finite numerical agreement. Reintegrating the same pair solely to replace
missing counters is unnecessary for this scoped decision.

The retained execution ledger records15.712870685wall seconds and15.57707CPU
seconds across compile, failures, the one pair, and recovery; every recorded
command is below30seconds. The source-read review itself is separate. The
pair process duration includes fixture, both trajectories, diagnostics and
failed serialization, so it is not an isolated timing measurement.

Reviewed hashes: lifted_bdf.py
`80345e9b35321988d293e20108a65f2107e2a91c4a0e77689e0a0f66d235d1ba`;
run_pair.py
`b22373dce1875f1825316be6e30a808b10ddbb879769a2f91ef96d4f320706fe`;
recovered RESULT.json
`3af2f42de97422392259550e9c5639f20bc643407ce2bd130a5f000e03bbb957`;
TRANSACTION_TEST.json
`b4d5b5ede8865ae954104aff20ba07c87025b66642384d77c87d3698c3ee85e5`.

No blocking correctness finding remains in this bounded implementation and
saved-pair scope. The next action is additive publication/backup using actual
root receipts. No further optional test or scientific replay is requested.
