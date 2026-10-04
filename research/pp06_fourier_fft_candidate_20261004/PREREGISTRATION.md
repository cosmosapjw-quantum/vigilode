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
