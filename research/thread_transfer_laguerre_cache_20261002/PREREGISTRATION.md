# Preregistration: Laguerre adjoint envelope cache and cost (thread-transfer DAG node P2-LAGUERRE-CACHE)

## Question

L-0039 added the signed output-adjoint recurrence bound. Its envelopes `beta_j` depend only on the stored
coefficients, the extent `L'`, the subdivision depth and the proof version, not on the operator or the vectors. Is a
cache keyed by exactly those inputs safe (every change of a key part misses), and what do cold setup and warm reuse
cost against the action itself? For a same-vector input (`w_k = s_k v`), does one adjoint for the combined
coefficients `C_n = sum_k s_k c_(n,k)` (exact reals) bound the fused recurrence error more tightly than the sum of
the five column bounds?

## Implementation (written after this commit)

In `crates/rodas5p-core/src/laguerre_adjoint.rs`:

- `laguerre_adjoint_envelopes_counted`: the envelopes with the number of interval operations of their setup;
- `laguerre_adjoint_envelopes_interval`: the same for interval coefficients (each `c_n` enclosed, the Bernstein
  constant term an interval), so an exact-real combination can be enclosed without rounding it to one binary64 value;
- `LaguerreEnvelopeCache`: key = degree, extent bits, depth, SHA-256 of the coefficient interval bits, proof version;
  hit/miss counters; a hit returns the stored envelopes and costs no interval operations.

`joint_phi_action` and its reported components are unchanged.

## Test and command

`THREAD_TRANSFER_LAGUERRE_CACHE_OUTPUT=research/thread_transfer_laguerre_cache_20261002/RESULTS.json cargo test --release -p rodas5p-core --locked --test thread_transfer_laguerre_cache -- --nocapture`

## Gate

**PASS** if all hold:

1. **Invalidation.** An identical key hits and returns bit-identical envelopes; changing the degree, the extent (one
   ulp), the depth, any coefficient (one ulp; including the coefficients that a changed `h` produces) or the proof
   version misses.
2. **Cold and warm separated.** A miss reports its interval operation count (positive); a hit reports zero; cache
   statistics count hits and misses separately.
3. **Combination valid.** For the same-vector case of a Laguerre table (`laguerre_coefficient_enclosures`, five
   phi columns, scales `s_k = 1/(k+1)`), the combined bound `sum_j beta_j(C) eps_j` is at least the fused recurrence
   error computed in exact rationals from the native recurrence vectors (checked in the test with an exact
   rational evaluation of the recurrence vectors, which are dyadic), on a diagonal 3-D operator at degrees 16 and 32.

Otherwise **FAIL**. Reported, not gated: setup interval operations for m = 16, 32, 64, 128 (and the fitted exponent),
against the action's floating-point operations `m (n^2 + 8n)` for n = 8 and 64, with the number of actions that
amortize one setup; the ratio of the combined bound to the sum of column bounds.

The claim ceiling is an amortized cost study and cache safety. There is no support claim above degree 128 and no
change of `TotalErrorStatus`.

## Prior information

L-0039 (envelopes valid, cubic-looking setup in development, not measured). No code of this node exists before this
commit.

## Amendment before any code (exact check of item 3)

Rust has no exact rational arithmetic here, so the exact part of gate item 3 runs, as for L-0039, in Python: the
test's ignored writer stores the same-vector cases (operator, source, column coefficient bits, scales, native
recurrence vectors, native combined and per-column bounds) in `fixtures/thread_transfer_laguerre_cache_cases.json`,
and

`python3 tools/thread_transfer_laguerre_cache_check.py --cases fixtures/thread_transfer_laguerre_cache_cases.json --output research/thread_transfer_laguerre_cache_20261002/EXACT_CHECK.json`

computes the fused recurrence error `sum_k s_k sum_n c_(n,k) (t_hat_n - t_n)` exactly and checks it against the
native combined bound. The threshold is unchanged.

---

## Results (appended after the run at `94c3f8f`)

Outputs: `RESULTS.json`, `EXACT_CHECK.json`. Ledger row L-0042.

**Gate: PASS.**

| Gate item | Outcome |
|---|---|
| 1. Invalidation | holds: the identical key hits with bit-identical envelopes; the degree, a one-ulp extent, the depth, a one-ulp coefficient, the coefficients of `h` one ulp larger and the proof version each miss |
| 2. Cold and warm separated | holds: the cold setup at m = 32 costs 145,454 interval operations, a hit 0; 1 hit and 6 misses counted |
| 3. Combination valid | holds: exact fused recurrence errors 3.87e-17 (m = 16) and 3.82e-17 (m = 32) against combined bounds 4.79e-16 and 5.10e-16 |

Setup cost (interval operations; the number of actions of cost `m (n^2 + 8n)` flops one setup equals):

| m | setup | n = 8 | n = 64 |
|---|---|---|---|
| 16 | 23,342 | 11 | 0.3 |
| 32 | 145,454 | 36 | 1.0 |
| 64 | 1,013,806 | 124 | 3.4 |
| 128 | 7,540,782 | 460 | 12.8 |

The fitted exponent over m = 16..128 is 2.78, close to the cubic count of Bernstein subdivision (an interval
operation is several binary64 operations, so these counts understate the setup in flops). For small operators a
setup costs tens to hundreds of actions, so the bound pays only with reuse of identical keys (same `h`, scale, degree
and depth). Large degrees should not be enabled without that reuse.

The same-vector combination gave no gain here: the combined bound equals the sum of the column bounds to 1e-11,
because the Laguerre phi coefficients of all five columns have one sign, so nothing cancels. It would matter only for
coefficient sets with mixed signs, which this node did not test.

Development disclosure: the test and the checker ran once before the recorded run (same outcomes). Claim ceiling: an
amortized cost study and cache safety; no support above degree 128; `TotalErrorStatus` unchanged.
