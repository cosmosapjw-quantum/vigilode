# Decision log

- Selected bounded fixed-schedule experiment kernel; adaptive schedules and acceleration remain prohibited in cycle 1.
- Added correction=0 as the genuinely low-depth comparator; correction=1 remains the accuracy lane.
- Added a noncommuting mass-matrix manufactured family before scientific closeout.
- Added a global fixed-step order screen after the one-step certificate screen passed.
- Fixed `E_out <= 0.1` is retained only as the cycle-1 test policy; it is rejected as a sufficient global-order policy.
- Deferred adaptive/LTE-scaled defect policy to the next research cycle under a new contract.

- Renamed the cycle-1 acceptance parameter to `output_wrms_budget`; it is an absolute normalized
  output-correction threshold, not a fraction of embedded LTE.
- Classified `direct_solve_calls` as batch-call depth only and based equal-work rejection on RHS,
  JVP, and common-`W` vector work.
- Independent diff review passed with scientific/performance HOLD; q<7 is explicitly described as
  approximate propagation rather than exact-start continuation.
