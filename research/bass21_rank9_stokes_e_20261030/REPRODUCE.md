# Source-contained reproducibility: BASS21 rank-9 coherency + E_l0 (+)

1. Extract the delivered FULL_SOURCE ZIP into an empty directory; do not substitute the owner archive or any other BASS KM2F/2G event.
2. Host used: Linux x86_64, CPython 3.13.5, NumPy 2.3.5, SciPy 1.17.0, SymPy 1.14.0, Matplotlib (figures only). Other versions NOT_CHECKED.
3. Execute `bash VERIFY.sh` with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1`.
4. Verifier hashes all tracked payload, reads the unchanged PR138 complete ZIP, recursively reconstructs original BASS owner ZIP under exact SHA, verifies original 50/50 source NPZ hashes, executes 9/9 exact/math/science tests, then recomputes 50x193 original source fixed Volterra (NOT new IVP), and requires all six output hashes identical to packaged science arrays/CSV/JSON.
5. `python3 code/plot_science.py` reproduces plots from saved NPZ. Figures are not a substitute for angular-domain validation.

Full original BASS source owner archive SHA-256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, parent VigilODE PR138 canonical ZIP SHA-256 `ab7044f604fdd7badb7eb21bae60f7900dd47e1e81b0ed1cce3091283162309f`. All source data and methods frozen. The new E harmonic coefficients have explicitly labelled screen and + sign; no owner-native full dynamical E_l equality is claimed.
