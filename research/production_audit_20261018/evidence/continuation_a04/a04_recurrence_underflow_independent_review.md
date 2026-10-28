# Independent review: A04 additive-underflow recurrence slice

Requested profile: gpt-6-astra/xhigh read-only reviewer; observed runtime identity is not independently verified.

**Scoped PASS for recurrence interval replay; full A04 remains HOLD / EstimateOnly.** At source `0179e3e7750cbb053d95f8f519a28037b3494195`, `recurrence_interval_next` encloses the exact expression `((h * Q - center * old) / scale) - node * old` by outward interval composition. For the witness `h=2^-1074`, `Q=old=1`, `scale=2`, and other terms zero, the binary64 recurrence rounds to zero while the real update is `2^-1075`; the exact value lies in the interval whose endpoints straddle zero and the minimum subnormal. The relative-only bound is zero and is falsified. The helper charges 48 operation proxies before fallible arithmetic; error-path transaction behavior remains unchanged.

Post-EXEC checks passed: focused unit 1/1 and Leja contract 16/16. Earlier retained validation also reports clippy, fmt, diff-check exit 0. Exact raw logs are gzip archived and their decompressed SHA-256 values are recorded in the receipt. A too-strong intermediate assertion requiring a strictly negative lower endpoint was preserved and corrected because outward rounding may produce zero; this did not change a numerical threshold.

No tolerance, coefficient formula, or authority changed. This extracts an existing interval expression and adds a falsifier; it does not close total additive-underflow/operator/coefficient/output arithmetic. `EstimateOnly` remains mandatory pending those obligations and independent total-path proof.
