# Scientific Contract

- State norm: dimensionless WRMS with per-component `atol + rtol*max(|old|,|new|)`.
- Step size has units of time; ratios and controller factors are dimensionless.
- Fixed-step public behavior is protected.
- Rejected trial state/history/recycle/output is rolled back; work remains counted.
- BDF2 uses nonuniform coefficients when variable mode is requested.
- Cross-family authority is external global error and total work, not equal internal tolerance.
- Validity range: binary64 regular ODEs with nonsingular dense mass matrices.
