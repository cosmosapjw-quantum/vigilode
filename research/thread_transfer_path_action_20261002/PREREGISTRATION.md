# Preregistration: action-first path-sum certificate (thread-transfer DAG node P1-PATH-ACTION)

## Question

The doubling certificates (`doubling_certificate_with_execution`, `blocked_doubling_certificate_with_execution`)
build the sum matrix `S = (I + H^(2^(L-1))) ... (I + H)` by `S <- S + Q S; Q <- Q^2` and only then form `E = S a`.
Only `E` is needed. The review (section 4.4) proposes the action-first order

```
e = a; Q = H
for level in 0..L:  e <- e + Q e;  if level + 1 < L: Q <- Q Q
```

which in exact arithmetic gives `e = sum_{j < 2^L} H^j a = sum_{j < s} H^j a` for a strictly lower `s x s` block
(`L = ceil(log2 s)`). With nonnegative `H` and every operation rounded upward, it is a separate valid upper bound,
not bit-identical to the matrix order.

Is the action-first certificate correct, and does it cost fewer counted operations on the component blocks?

## Implementation (written after this commit)

- New research module `crates/rodas5p-integrators/src/causal_majorant.rs`:
  - `upper_path_sum_action`, `upper_path_sum_matrix` (the blocked certificate's matrix order) and
    `upper_causal_solve` (`e_i = a_i + sum_{j<i} H_ij e_j`) on one strictly lower nonnegative block, with
    `PathSumWork` counts (directed operations, matrix-matrix and matrix-vector products, allocated and peak live
    f64 slots);
  - every input checked: square, strictly lower (a nonzero diagonal or upper entry is
    `CERTIFICATE_STRUCTURE_UNSUPPORTED`), finite, nonnegative, matching seed length. Structural zeros stay exact
    zeros.
- `blocked_action_doubling_certificate_with_execution` in `outward_certificate.rs`: the same arguments and result
  type as the blocked certificate, the same `H` entries, only the path-sum order changed. The existing functions,
  the default q2 routing and the R4 study are unchanged.

## Tests and command

`crates/rodas5p-integrators/tests/thread_transfer_path_action.rs`:

`THREAD_TRANSFER_PATH_ACTION_OUTPUT=research/thread_transfer_path_action_20261002/RESULTS.json cargo test --release -p rodas5p-integrators --locked --test thread_transfer_path_action -- --nocapture`

## Gate

**PASS** if all hold:

1. **Exact identity.** On integer-valued strictly lower blocks of size s = 1, 3, 8, 9, 16 (entries 0..3, seeds 0..3,
   every intermediate an integer below 2^53, so no rounding occurs), action, matrix order and causal solve agree
   bit for bit.
2. **Invalid structure rejected.** A nonzero diagonal entry, an upper entry, a negative entry, NaN, infinity, a
   ragged or non-square block and a seed of the wrong length are all errors, for all three functions.
3. **Enclosure.** On the five R4 fixtures (n = 1, 2, 4, 8, 16; see `research/thread_transfer_radius_replay_20261002`)
   with 8 radius attempts, the action certificate closes at the same attempt as the blocked one, and its stage
   bound encloses the exact stage root: `|K_hat - K*| <= E` for every point of an interval enclosure of `K*`
   computed by the causal stage recurrence in directed interval arithmetic (the quadratic target is explicit in
   `K_i` given `K_j`, j < i).
4. **Fewer operations.** On every one of the five fixtures, the action certificate counts strictly fewer directed
   operations and strictly fewer allocated f64 slots than the blocked certificate, over the same attempts.
5. **Deterministic.** Bit-identical action bounds for 1, 2, 4 and 8 Rayon workers.

Otherwise **FAIL**. Reported, not gated: the largest relative difference between the action and matrix-order
bounds; output-bound widths against the serial certificate; the ratio of counted operations.

The claim ceiling is certificate-action correctness and counted cost; there is no wall-clock or admission claim.
Timing authority stays on HOLD.

## Prior information

- The review's `path_action_probe.py` (exact rationals, widths 1, 3, 8, 9, 16, four invalid inputs) passed in Python.
- The blocked certificate counted 3689, 9186 and 21988 operations at n = 1, 2, 4 (L-0024); the serial certificate is
  cheaper at every n (425-26000).
- No Rust code of this node exists before this commit.
