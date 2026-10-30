# Offline reproduction from the source-inclusive package

Python 3.11+ with NumPy, SciPy, Matplotlib. This does not require GitHub, NCP, nor a new IVP.

```bash
cd VIGILODE_BASS21_LRS_POL_SOURCE_20261030
bash VERIFY.sh
```

`VERIFY.sh` authenticates the original 23,770,654-byte BASS21 ZIP; verifies source code syntax; runs model-data replay into a temporary directory; compares its numerical outputs with packaged exact saved scientific arrays; runs 13 focused tests and the 50-case mesh-refinement diagnostics. All generated Q is **first gain only** with frozen unpolarized BASS hierarchy and normal-comoving cold electrons. `results/` contains compressed 50×193×17 physics arrays, endpoint and paired CSV, exact algebraic attribution CSV, original-source plots, and precise summary/evidence. PR127's own `VERIFY.sh` remains inside `source/`'s original ZIP and can be run separately without modifying this package.

Package reference file hashes reside in `FILE_MANIFEST.json`. `STAGE_LEDGER.json` records stage-scoped provenance; it is not 20 independent compute campaigns or separate external backups.
