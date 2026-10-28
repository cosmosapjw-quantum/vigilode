# MATRIX_FREE_COMMON_W_SUBSTRATE claim ledger

## Authority

Remote stacked base intended for publication:

```text
PR #31 head  c5fbd6d5703fc396bdf30eb3acfacb6c6bd2b921
tree         a0fec46f857f00054d674fb417812065aeca8a31
```

Local implementation/verification used an exact reconstruction of:

```text
head         e77ec86376ca89850e18e99963992aeeb01055c2
tree         44e5d68dcb91c8a167c987fa9c92a8134f866f87
```

PR #31 changes seven paths. This work changes none of those paths; therefore
patch conflict risk is structurally absent, but exact-base compilation and
execution are still mandatory before publication.

## Host exact-c5 addendum (2026-08-30)

Host Codex applied the supplied patch directly to the declared c5 head after
both the remote-head and tree gates passed. The exact-c5 matrix-free suite ran
6/6 tests and the unchanged PR #31 bridge suite ran 15/15 tests. The controlling
prompt expected 16 bridge tests, but c5 contains 15; this is retained as
`EXPECTED_16_OBSERVED_15` rather than repaired by changing a protected PR #31
path. The readiness workflow ran 64 Rust tests and 20 Python tests, all passing.

The numerical and structural rules were fixed independently of these outcomes.
No tolerance, condition-aware ceiling, state point, problem size, or mutation
was widened or selected after inspecting the exact-c5 values. In particular,
the fixed coupling-sign mutant failed with exit 101 and relative difference
`0.4899133405529948`; the byte-restored source passed with relative difference
`4.760950091765468e-16` against the existing explicit common-W reference.

The complete host command logs are retained outside the repository and bound by
SHA-256 in `evidence/HOST_EXACT_C5_VERIFICATION.json`. The compact record keeps
the raw numerical observations needed for external re-audit without pretending
that temporary host paths are portable publication artifacts.

The single fresh review found one P1: the projected operator used uncounted
application metadata, and constructor failures discarded their attempted setup
snapshot. The one permitted repair switched that reconstructed operator to the
existing counted-JVP constructor and returns a typed, boxed setup failure. The
final correction record has `linear_matvecs=136`, `diagnostic_matvecs=8`, and
`jvp_calls=jvp_vectors=144`; invalid configuration retains
`setup_attempts=1`, `setup_completed=0`, and zero solve attempts. Final scoped
tests/readiness/clippy/format/diff checks pass. No second fresh review was run.

The same review recorded two nonblocking follow-ups: duplicated nonlinear
snapshot/RHS preparation (P2) and a stale historical handoff count (P3). They
are preserved as deferred history, not silently repaired or promoted into the
current claim.

## Supported claims

| Claim | Evidence | Disposition |
|---|---|---|
| A matrix-free shifted-W solve can be executed without an explicit shifted matrix or direct factorization in the declared probes. | Matrix-free contexts expose no explicit W; all candidate direct-factorization/direct-solve counters are zero. | `SUPPORTED_IN_DECLARED_PROBES` |
| One Audit-2 session retains its operator identity, identity-preconditioner setup, and GMRES workspace across multiple RHS solves and two batches. | `setup_attempts=1`, `workspace_initializations=1`, `solve_completed=16`, capacity growth after first solve `0`. | `SUPPORTED_AS_ALLOCATION_AND_SETUP_REUSE` |
| The actual eight-stage matrix-free block-forward correction agrees with the existing explicit-W research reference in the fixed n=16/h=0.01 case. | Relative correction difference `4.760950091765468e-16`; condition-aware ceiling `1.2660497088853964e-7`; reapplied linear residual `3.3065204906367934e-20`. | `SUPPORTED_IN_ONE_DECLARED_PROJECTED_CASE` |
| A late malformed RHS preserves the completed first row and spent work. | Typed failure retains one solution, one solve report, workspace capacity, and all counters. | `SUPPORTED_BY_FAILURE_CONTRACT` |
| The stage-coupling sign is protected by a meaningful regression. | `+= h J_i p_i -> -= h J_i p_i` mutant raises relative difference to `0.4899133405529948` and the test exits 101; restored code passes. | `SUPPORTED_BY_MUTATION` |

## Forbidden or unestablished claims

| Claim | Reason |
|---|---|
| Production/default activation | No production dispatcher, controller, acceptance route, or default feature changed. |
| Scalable performance or speedup | No wall timing and no large-client campaign were run; the validation oracle is explicit and small. |
| Krylov-basis/subspace reuse | Workspace allocation is reused, but every RHS performs a fresh GMRES solve. All recycle counters remain zero. |
| A useful production preconditioner | The candidate uses the identity preconditioner only. |
| Original-target, nonlinear, or output accuracy admission | This node tests the projected correction substrate and supplies no observable budget/reference-uncertainty authority. |
| Transactional whole-step correctness | Rollback, rejection, controller, restart, dense output, and accepted-state integration were not wired to this entry. |
| BDF/CVODE ranking, holdout, freeze, PM-7/K0 closure, merge, tag, release | Not performed. |

## Validity separation

- `RESULT_VALIDITY`: only the declared n=48 session probe, n=16/h=0.01
  block-forward case, malformed-input failure, and coupling-sign mutation.
- `PROVENANCE_VALIDITY`: exact supplied-patch application and host replay on
  c5/tree a0fec; remote publication identity is supplied by the PR readback and
  cannot be embedded in its own pre-commit bytes.
- `PACKAGING_VALIDITY`: the original package evidence remains historical; this
  addendum and `evidence/HOST_EXACT_C5_VERIFICATION.json` are the current host
  replay receipt. Remote branch/PR status is recorded after publication rather
  than predicted here.

Claim ceiling:

> `EXPLORATORY_NONAUTHORITATIVE_MATRIX_FREE_COMMON_W_SUBSTRATE` — a reusable
> matrix-free shifted-W workspace/session and one projected block-forward
> compatibility result. No production, scalability, speed, basis-reuse,
> original-target accuracy, comparator, or release claim.
