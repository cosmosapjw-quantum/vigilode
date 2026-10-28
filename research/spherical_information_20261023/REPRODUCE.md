# Reproduce the lightweight checks

Use a copy of the extracted package if the supplied raw results must remain byte-identical.
Run from the package root. No Rust compilation or NCP connection is required.
Python 3.12.14, NumPy 2.3.5 and SciPy 1.17.0 were used. Plotting additionally
requires Matplotlib. A missing dependency is an environment failure, not a failed theorem.

```bash
export OPENBLAS_NUM_THREADS=1
export OMP_NUM_THREADS=1
export MKL_NUM_THREADS=1
python3 -m unittest discover -s src -p 'test_*.py' -v
python3 src/spherical_mesh.py
python3 src/fisher_demo.py
python3 src/thomson_demo.py
python3 src/remap_budget_demo.py
python3 src/spectral_tools_demo.py
python3 analysis/check_paper_i_algebra.py
python3 analysis/review_spotchecks.py
python3 src/make_figures.py
```

The Thomson script uses the bundled frozen oracle by default. An explicitly selected checkout can be supplied with
`--repo /absolute/path/to/vigilode`; its oracle hash must match the frozen
reference. The current checkout does not need to be reset or switched.
Source identity is the oracle SHA256, not a claim that another branch has been tested.

The frozen reference is from VigilODE commit
`14c74978b93339a79d7228a324fa77552dae548e`, file
`research/bassrei_p06_full_sphere_scoped_20261022/FULL_SPHERE_FVM_REFERENCE.py`,
SHA256 `773552acc6c42de5f65972453a32e9f9dff7ac96bde33b358b05e18edd8de891`.

Expected scientific checks:

| Component | Acceptance contract |
|---|---|
| Mesh | 20 configurations; both methods within the same allowed discrete entropy budget; positive reconstruction; energy conservation; actual observer error below computed bound |
| Joint moments | number and energy conserved; positive MB Hessian; nonnegative Jensen gap |
| Fisher | 13 cases; reference whitening; finite-family membership; eigenvalues inside susceptibility interval; original-coordinate residual <=1e-9 |
| Thomson | agreement with independent small dense exponential, rank-deficient oracle case, mass/transversality/PSD checks at recorded float tolerances |
| Remap | 8 configurations /320 steps; Hellinger and bounded-observable accumulated bounds;30 stochastic maps |
| Spectral | Laguerre Gram, L2 cutoff, L-infinity positivity margin, v=0 and v=1 endpoints |

Consult the source assertions for exact numerical tolerances. Timing is diagnostic:
the host is shared, setup costs and multiple raw timing samples are recorded, and no
fixed wall-time threshold is an acceptance condition. Several small cases favor the baseline.
Repeated runs may change timing while preserving the scientific conclusions.

`ARTIFACT_MANIFEST.json` checks bytes of the delivered package; it excludes itself.
It is a package identity check, not an additional mathematical proof.
