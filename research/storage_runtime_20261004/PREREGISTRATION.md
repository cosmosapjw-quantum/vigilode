# STORAGE-RUNTIME — measured allocator bytes of the shared shift jet (prospective registration)

RVJ DAG node `STORAGE-RUNTIME` (depends on PP02). Base commit: the commit
that adds this file. No timing.

## Measurement

A counting global allocator (test binary, one thread) records live and
peak heap bytes. For `shared_shift_jet` on a sweep n in {4, 16, 64, 128},
r in {1, 4}, degree p in {0, 8, 24}, targets m in {1, 17, 65} (72
configurations, seeded dissipative J) it records: peak bytes above the
live bytes at entry (`P_total`); the same for `LuFactorization::new` of the
n x n center plus one `solve_rows` of r columns alone (`P_lu`); and
`8 * explicit_storage_upper_scalars_excluding_lu` (`S_explicit`).

## Gate

G1 Explicit accounting covers the non-LU part: `P_total - P_lu <=
   S_explicit + 64 * (vector headers allocated)` in every configuration,
   with the header count taken from the allocation count. A failure means
   the explicit slot formula misses a buffer and is a FAIL.
G2 The budget refusal allocates nothing proportional to the problem: with
   `max_stored_scalars` or `max_work_units` one below the requirement, the
   call errors with at most one allocation during the call, of at most 256
   bytes (the error message string).
G3 Reported, not gated: `P_lu / (8 n^2)` and `P_total / S_explicit`; the
   documentation states that the explicit cap is not an RSS cap.

Amendment before any run (2026-10-04): G2 originally said "zero
allocations"; every refusal builds a `CoreError::InvalidInput` message
`String`, so it was changed to "at most one allocation of at most 256
bytes". No measurement had been made when this was changed.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `108a0d5`)

Command: `STORAGE_RUNTIME_OUTPUT=research/storage_runtime_20261004/RESULTS.json
cargo test --release -p rodas5p-core --locked --test storage_runtime --
--ignored --nocapture --test-threads=1` (the test writes the gate and the
verdict into RESULTS.json).

**Verdict: FAIL (G2, as registered).**
- G1 PASS: in all 72 configurations the non-LU peak `P_total - P_lu` is
  within `S_explicit + 64 x allocations`. The explicit f64-slot bound alone
  does not cover it in 22 configurations (n = 4 everywhere, and n = 16 with
  m = 65): there the non-LU peak is 1.0x to 2.0x `S_explicit`, the excess
  being struct and `Vec` header bytes of the per-target candidates and
  certificates, which are not f64 slots. The slot bound is therefore not a
  byte bound for small n, which the module documentation already states
  (it excludes metadata and faer workspace).
- G2 FAIL: every refusal (both budgets, 72 configurations) made 2
  allocator events of at most 72 bytes, where the amended gate allowed at
  most one. From the code, the refusal path allocates only the error
  message: `format!` reserves an estimated capacity for
  `"shared shift jet: {message}"` and grows it once (the tracking allocator
  counts the growth `realloc` as an event). No buffer proportional to the
  problem is allocated before the budget check, which is what the gate was
  meant to show; the registered count limit was nevertheless exceeded and
  the verdict stays FAIL. The realloc reading is from the code and the
  72-byte size, not from a separate measurement.
- G3 (reported): `P_lu / (8 n^2)` is 3.0 to 3.1 for n >= 16 (6.5 and 10.5
  at n = 4; 131 for the first n = 64, r = 1 measurement, an unexplained
  outlier, possibly first-use workspace in faer, not investigated);
  `P_total / S_explicit` is 1.14 to 2.24.
