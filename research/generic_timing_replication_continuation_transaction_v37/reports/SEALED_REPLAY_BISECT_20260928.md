# Sealed v3.7 replay bisect, 2026-09-28 (audit F-003)

This note is new. It changes no receipt, no result and no pinned literal.
The test `v37_exhaustion_is_a_charged_abstention_without_endpoint_or_failure_label`
in `crates/rodas5p-integrators/tests/v37_continuation_transaction_contracts.rs`
still pins 18, and it still fails at line 110 with `left: 17, right: 18`.

## Result

The first bad commit is `ab8fbcdb709aa1e87603b1ef6f83c5e610c8cb04`,
"feat: implement scientific validity v2 audit plan".

In that commit, the live RODAS step on the G4/S5B0 atlas trajectory changed.
`run_trajectory` in `crates/rodas5p-integrators/src/g4_s5b0_regime_atlas.rs`
used to call `sequential_matrix_free_step` with the configured GMRES tolerances.
It now calls `g4_live_rodas_step`, which wraps
`sequential_matrix_free_step_with_inner_forcing`. That function sets each
stage's GMRES tolerance from the WRMS inner-forcing rule
(`rodas5p_inner_forcing_target`).

The inner linear solves change, so the adaptive trajectory changes. On the
sealed N=192 semilinear advection-diffusion family, the second frozen
recommendation moves from attempt 18 to attempt 17.

## Method

- Range: good `84a3b0f`, bad `b3e8165`, from `fixes/G6_evidence_process_tests.json`
  in the audit bundle.
- The sealed commit `84a3b0f` was tested first, and it passes.
- The bisect then used `git bisect run`. Each step ran
  `cargo test -p rodas5p-integrators --test v37_continuation_transaction_contracts --locked -- --ignored`.
  `RAYON_NUM_THREADS` was left at its default of 4 cores. A rerun at
  `b3e8165` with `RAYON_NUM_THREADS=1` gave the same failure: exit 101,
  `left: 17, right: 18`.
- Older commits point crates-io at a vendored directory that is not in the
  repository. For those commits, the step replaced `.cargo/config.toml` with an
  empty placeholder. Dependency versions were unchanged, because the lockfile was
  honoured with `--locked`.

| Commit | Exit | Detail |
|---|---|---|
| `84a3b0f` | 0 | sealed commit passes |
| `79f6e31` | 0 | |
| `4223391` | 0 | |
| `17fcd44` | 101 | line 110, `left: 17, right: 18` |
| `93fe348` | 101 | line 110, `left: 17, right: 18` |
| `8d0c791` | 0 | |
| `8419d8e` | 101 | line 110, `left: 17, right: 18` |
| `ab8fbcd` | 101 | line 110, `left: 17, right: 18` (first bad) |

## Cause isolation

`ab8fbcd` changes more than 100 files. Two follow-up experiments located the
cause. In each, one call was reverted: `g4_live_rodas_step` was switched back
to `sequential_matrix_free_step(problem, t, y, h, linear, None, atol, rtol, false, counters)`.
The experiment patches were never committed.

- Experiment A, at `ab8fbcd` with the revert: the ignored test passes (exit 0,
  115 s).
- Experiment B, at `b3e8165` with the same revert: the ignored test passes
  (exit 0, 115 s).

So the switch to WRMS inner forcing on the atlas lane is the whole drift
between `84a3b0f` and `b3e8165`. Other changes in that range do not move this
test.

## Later literals have drifted too

The assertion at line 110 stops the test. A scratch probe ran the same family
at unmodified `b3e8165` and printed every pinned quantity. The probe file was
deleted afterwards.

| Quantity | Pinned | At `b3e8165` |
|---|---|---|
| recommendations, retained level-2 resumptions | 2, 2 | 2, 2 |
| completions, exhaustions, failures | 1, 1, 0 | 1, 1, 0 |
| unsafe recommendations, continuation budget breaches | 0, 0 | 0, 0 |
| RJF parity, hard gates | pass, pass | pass, pass |
| exhausted row: attempt, used JVP vectors | 12, 80 | 12, 80 |
| completed row: attempt | 18 | 17 |
| completed row: continuation JVP vectors | 36 | 52 |
| report continuation JVP vectors | 116 | 132 |
| total speculative = prefix + continuation | prefix + 116 | 120 + 132 = 252 |

## Open, for the owner

There are two ways to reconcile the test with the code:

1. Re-seal the replay under the WRMS forcing rule, with a new receipt and the
   values 17, 52 and 132.
2. Restore fixed tolerances on the atlas lane.

This note does neither, as the handoff requires.

The WU-3 branch (PR #46) changes the inner-forcing target again. Its effect on
this replay has not been measured.

The test is `#[ignore]`d, and no CI job runs it with `--ignored`. That is why
the drift was not caught at `ab8fbcd`.
