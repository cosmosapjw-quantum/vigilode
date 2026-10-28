# A01 identity continuation: attempt history

Base: `6c067105f0b9e16cbfd397b2d5316673e698e00e` on `audit/production-readiness-20261018`.

1. Initial targeted compile failed with `E0425` because the new integration regression omitted the public `reconstruct_modes` import. Added the import; no production source change was needed for that compiler error.
2. First targeted execution after identity binding found that `evaluate_mode_at` intentionally restricts `P(tau*s)` and changes the linear kernel horizon. The restricted candidate still carried the old descriptor identity and returned `InvalidInput("candidate was generated for a different compiled packet")` at the output check. The restriction path now explicitly transfers identity to the generated shortened packet after scaling every polynomial coefficient by `tau^k`.
3. The read-only independent reviewer called out the `tau == 0` cross-packet case. The implementation already validates candidate identity before the zero-time return; the regression now checks that exact edge explicitly.

The initial E0425 and stale identity failure were productive diagnostics. The unchanged retry of the targeted set passed after the identified fixes. Complete command output is retained in `targeted_tests.log`.
