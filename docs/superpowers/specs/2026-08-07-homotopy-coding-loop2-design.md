# Homotopy Coding Research Loop 2 — Design Specification

Date: 2026-08-07
Base tag: `rust-pre-homotopy-v0.2.0-alpha1`
Branch: `homotopy-coding-loop2-v0.4`

## Goal

Turn the promoted mathematical survivor family into a small, auditable Rust research kernel that can support the next numerical-experiment branch without yet implementing an adaptive homotopy integrator or making performance claims.

## Scientific object

The protected RODAS5P stage problem is

\[
\mathcal R(K)=(D-C)K-g-hN(K)=0,
\]

\[
D=I_8\otimes W,\qquad W=M-h\gamma J_n,\qquad C=hL\otimes J_n.
\]

The promoted partial-coupling homotopy is

\[
H_\theta(K,\lambda)
=(D-\theta C)K-g
-\lambda[(1-\theta)CK+hN(K)].
\]

This coding loop implements only the affine/reference structure, exact and truncated nilpotent inverse actions, endpoint/original-residual certification, and deterministic design screens.

## In scope

1. A research-only `homotopy` module in `rodas5p-integrators`.
2. Explicit affine block oracle for noncommuting mass and Jacobian matrices.
3. Start, path, and target block operators for arbitrary finite `theta, lambda in [0,1]`.
4. Exact finite nilpotent inverse action and truncations `q=0..7`.
5. Original-stage residual and output-weighted defect certificate for the affine oracle.
6. Deterministic structural screens:
   - endpoint and start identities;
   - `theta=1` zero-continuation affine limit;
   - `q=7` exactness and low-`q` truncation error;
   - official RODAS5P `L^m` decay;
   - determinant-one/non-normal condition-growth counterexample.
7. A `homotopy-design-check` CLI command emitting canonical JSON.
8. Coding-harness Phase 0–10 documentation, review, closeout, and experiment-branch handoff.

## Out of scope

- adaptive continuation in `lambda` or `theta`;
- nonlinear remainder continuation;
- Anderson acceleration;
- Chebyshev or Padé path interpolation;
- actual performance benchmark or solver ranking;
- modification of sequential RODAS5P or SABR5P behavior;
- BDF/Radau comparison;
- new dependencies;
- production API stability claim.

## Architecture

### `homotopy.rs`

Defines:

- `PartialCouplingParameters { theta, lambda }` with validation;
- `AffinePartialCouplingOracle` containing `D`, `C`, `g`, dimensions, and RODAS weights;
- exact path solve using the existing `faer`-backed LU;
- normalized coupling action `Q_eta = eta D^{-1} C`;
- truncated inverse action `sum_{m=0}^q Q_eta^m D^{-1} r`;
- target residual and output-defect certificate;
- deterministic screen helpers and serializable reports.

The oracle is deliberately explicit and small-scale. It is a correctness oracle, not the future matrix-free production solver.

### CLI

`rodas5p homotopy-design-check --output <json>` runs only deterministic structural checks. It does not integrate an IVP or time a solver.

## Failure semantics

The following return explicit errors and never become success:

- non-square or inconsistent matrix dimensions;
- non-finite `theta`, `lambda`, matrices, stages, or residuals;
- `theta` or `lambda` outside `[0,1]`;
- `q >= stages`;
- singular `W`, `D`, path operator, or target operator;
- mismatched RODAS weight length or scaling vector;
- non-finite certificate.

## Scientific acceptance

1. `H_theta(K,1) == R(K)` to binary64-scaled tolerance.
2. `H_theta(K,0) == (D-theta C)K-g`.
3. For affine-in-state problems and `theta=1`, the path solution is independent of `lambda`.
4. For any `theta`, the `lambda=1` solution agrees with the target block solve.
5. The full truncation `q=s-1` agrees with the direct inverse for noncommuting `M,J`.
6. Low-depth truncations expose nonzero, monotonically nonincreasing error on the canonical chain screen; monotonicity is a test-case property, not a general theorem.
7. Output-defect certificate equals the weighted output error induced by the exact affine residual correction.
8. The nonnormal screen demonstrates growing `cond_1` despite unit determinant.
9. Existing workspace tests remain unchanged and pass.

## Promotion boundary

The module may be promoted only as a **research oracle and numerical-experiment preparation layer**. No homotopy fast-path, order, stability, DAE, or wall-time claim is authorized.
