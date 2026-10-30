# Reproduce source-frozen BASS21 even-spin-2 E high-rank hierarchy and flux

* Extract the FULL_SOURCE ZIP into a clean **empty** directory. No external network download is required. Nested original BASS and all original source data are carried in `source/PARENT_PR143_FULL_SOURCE.zip` and recursively validated.
* Expected exact source parent ZIP SHA256: `a2fa5e7cbae98cb7a28df19314b1c6da8b90ad5277c08831384290171f4231ee`. Owner archive SHA256: `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`.
* On Linux with CPython 3.13, NumPy 2.3.5, SciPy 1.17.0, SymPy 1.14.0, run:

```bash
export OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 PYTHONDONTWRITEBYTECODE=1
bash VERIFY.sh
python3 code/plot_science.py
```

The verifier checks payload SHA, original archive byte identity, original 50 cases SHA, executes 10 theory/science tests plus source-free Git hash checks, re-evaluates the **original pre-existing Volterra `deltaI/Q` response** at all 50 x 193 epochs and compares the 6 new output files by exact bytes. It does NOT integrate a new Einstein/Boltzmann/chemistry IVP or certify original BDF time accuracy. Exact original dependency versions outside this recorded environment are not guaranteed byte-equivalent.

`code/highrank_atlas.py` produces the 12 even E+ mode array, 11-mode local shear/Thomson RHS atlas, and four upper-neighbor truncation-boundary flux atlases. The nested original BASS owner commit is `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; never bind this SHA to vigilode repository. Owner physical signature is (-,+,+,+) after convention translation.
