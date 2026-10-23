# Reproduce without overwriting historical evidence

The complete S3 ZIP in FINAL_DELIVERY.json contains66payloads plus ARCHIVE_MANIFEST.json. Verify its SHA256 and extract to a NEW directory. All raw inputs/outputs, the independent Fraction oracle, Python arbitrary-integer reviewer, original failed attempts, first integer implementation, original benchmark baseline, full cost rows and closed budget are included. Compiler archives/executables/.so/.ll are regenerable and omitted. This Git subtree is a reviewable selected-source copy, not all raw data.

For stored-result verification use a COPY of the archive and `python review_saved.py`. It recomputes681integer certificates and byte parity, checks budget log hashes, and rewrites REVIEW.json only in the COPY. It invokes no Rust or new timing. Do not point it at historical source artifacts.

For a separately authorized new experiment, provide Rust1.94.1 and a C compiler; no Cargo dependencies are needed. Use new directories and a new cumulative budget, not the original closed BUDGET.json.

```bash
rustc +1.94.1 --edition=2021 --test source/tests.rs -o tests
./tests --test-threads=1
rustc +1.94.1 --edition=2021 -C opt-level=3 source/probe.rs -o probe
python oracle_new.py
```

The complete ZIP provides intake data for the six old ambiguous inputs. oracle_new.py runs the NEW681cases, not all previous741. It writes new native outputs under evidence/ and must run only in a fresh copy.

x86 mode test is child-only. To avoid the original joint-emit basename issue, explicitly name the library (this split is documented adaptation, not a new executed receipt):

```bash
rustc +1.94.1 --edition=2021 -C opt-level=3 --crate-type=cdylib --crate-name integergate source/ffi.rs -o libintegergate.so
cc -O2 source/mode_probe.c -L. -lintegergate -Wl,-rpath,'$ORIGIN' -o mode_probe
./mode_probe < evidence/NEW_INPUT.txt
```

Do not change the parent process or user's FP environment. This mode test concerns integer input/certificate operations, not authorization to run the rest of an ODE solver under altered modes. FFI pointers have fixed readable/writable/alignment/nonoverlap requirements; normal Rust evaluate uses safe fixed-size references.

Archive source/bench.rs builds the original FP baseline, saved integer long-divider and optimized integer code into a single timing binary. All include decision/checksum; conversions are outside timers. Compile it with Rust1.94.1 opt-level3 and save new output to a NEW log. Old COST_FINAL.json must not be overwritten or new time samples attributed to this run. Timing does not claim a solver speedup.
