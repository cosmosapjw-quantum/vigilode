# Reproduce without overwriting historical evidence

Restore the S3 ZIP from FINAL_DELIVERY.json into a NEW directory after verifying its SHA256. It contains all trial/state bits, the exact scale checker and Fraction/MP80 saved reviewer, original RED/warning logs and closed budgets. Git's Rust/C sources suffice to build the consumer; full historical readback requires the companion raw files.

Saved-data-only verification: in a COPY of the restored archive run `python review_saved.py` with mpmath installed. This performs no solver/compiler/timing run, but writes REVIEW.json in that COPY and checks budget-log hashes. Do not replace the original.

For a separately authorized fresh experiment, use a new source copy and budget. Observed rustc1.94.1, LLVM21.1.8, x86_64 Linux; no Cargo dependencies. From the fresh unit directory:

```bash
mkdir -p bin evidence
rustc --edition=2021 --test source/tests.rs -o bin/tests
bin/tests --test-threads=1
rustc --edition=2021 -C opt-level=3 source/campaign.rs -o bin/campaign
bin/campaign
rustc --edition=2021 -C opt-level=2 source/scale_probe.rs -o bin/scale_probe
python scale_check.py
rustc --edition=2021 -C opt-level=2 --crate-type=cdylib source/envffi.rs -o bin/libenvguard.so
cc -O2 source/env_probe.c -Lbin -lenvguard '-Wl,-rpath,$ORIGIN' -o bin/env_probe
bin/env_probe
python review_saved.py
```

The Python scripts are in the full companion. Commands overwrite their named evidence, so never run them in the historical directory. The mode probe alters only its own child process and restores MXCSR. No fast-math/FTZ/DAZ; non-x86_64 is explicitly unsupported by this guard. Generated table.rs is committed; generate_table.py in the companion recreates it from the pinned JSON. Compiler binaries/executables/shared libraries are regenerable and excluded.

RUNS.csv timings include unequal evidence logging and cannot rank solver speed. This is a research RODAS5P realization, not a native production workspace build or actual BASS model.
