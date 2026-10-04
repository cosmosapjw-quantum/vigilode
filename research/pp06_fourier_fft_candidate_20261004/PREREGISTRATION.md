# PP06 — native coefficient FFT candidate for the Fourier client (prospective registration)

RVJ DAG node `PP06` (depends on PP05). Base commit: the commit that adds
this file. Source: `code/fft_bridge.py` (`required_shape`, `fft_rhs`,
`build_fft`) in the Loop 11 ZIP.

## Implementation

- A radix-2 complex FFT (no FFT crate is in the lockfile) and a 2-D
  transform over (harmonic, degree); the grid is padded to powers of two at
  least `N_k >= 2(3K + S) + 1`, `N_j >= 3P + Q + 1` (full product of the
  cubic right side with the phase), refused otherwise.
- Conjugation is applied to coefficients, `(k, j) -> (-k, j)`, before
  transforming, never to grid values (the degree axis is not physical time).
- The FFT right side replaces only the predictor's convolution; every
  frozen FFT candidate is certified by the unchanged PP05 certificate, which
  recomputes the noncyclic lambda = 1 right side. The primitive and the
  start enforcement are the PP05 ones.

## Runs (fixed now)

omega 40 and 1e4, T = 1/2, tol 1e-8, the PP05 driver with the FFT
predictor; plus a deliberately under-padded grid (`N_k = 2K + 1`) as an
alias negative control.

## Gate

G1 The FFT predictor's right-side coefficients equal the direct noncyclic
   ones within `1e-12` relative (max over coefficients, per call), on every
   predictor call of the runs.
G2 Every FFT-built committed state passes the PP05 enclosure check (same
   reference as PP05) and the PP05 certificate; no acceptance uses an FFT
   estimate.
G3 The under-padded grid is refused by the builder; when forced (test-only
   override) its candidate is rejected by the certificate or its bound is
   not met.
G4 A test where a roundtrip `ifft(fft(x)) = x` passes but conjugation on
   grid values (the wrong order) gives a different right side, showing the
   roundtrip alone is not a correctness argument.
Reported: predictor calls, transforms, padded grid points, certified steps
and rejections next to the direct predictor. No timing.

## Results (append only after the recorded run)

### Executed result — 2026-10-04 (source `fb52d1f`)

Commands: `PP06_CASES=research/pp06_fourier_fft_candidate_20261004/cases.json
cargo test --release -p rodas5p-integrators --locked --test pp06_fourier_fft
-- --ignored --nocapture`, then `python3 tools/pp06_fft_check.py --cases
.../cases.json --output .../RESULTS.json`. Unit test
`fourier_path_candidate::tests::roundtrip_and_conjugation_order` passes
(G4): a transform roundtrip reproduces the coefficients to 1e-15 while
conjugating grid values gives a different right side (relative
difference > 0.5).

**Verdict: FAIL (G3 as registered); G1, G2 and G4 PASS.**
- G1: on all 48 predictor calls of the padded runs (omega 40 and 1e4) the
  FFT right side equals the direct noncyclic one within 5.3e-16 relative.
- G2: both FFT runs reach T = 1/2 in 4 steps; every state and interior
  point encloses the reference (worst 0.108 and 0.069); 120 forward and 48
  inverse 2-D transforms per run on grids of at most 1024 points.
- G3 FAIL: the under-padded harmonic axis (`N_k = 8`, the power of two
  above `2K + 1 = 7`; the registered `2K + 1` is not a power of two) was
  refused without the override in 16 of 24 calls (the other 8 are first
  sweeps whose constant paths fit in 8 rows). With the override forced, the
  aliased right side differs from the direct one by at most 3.6e-12
  relative, all 4 steps were committed, and every committed bound encloses
  the reference (worst 0.083). As in PP05, the folded harmonics of this
  client are below the tolerance, so the certificate correctly accepted
  accurate candidates; the registered expectation of rejection did not
  occur and the control did not exercise harmful aliasing.
- No bound is below the actual error in any run.
