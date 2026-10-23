# Saved evidence and new-run reproduction are different

## Restore original S2, no Rust and no new experiment

Original ZIP: `VIGILODE_V1_PORTABLE_S2_20261024.zip`,802047bytes.
SHA256: `ffe50c8f246e38fef472eb69dd7528b88d96d31c2a95f2ec1b356349e965a661`.
Drive: https://drive.google.com/file/d/1zcYiuNO5JVVyKOwI_eiAGzRvvVWpSIaA/view
Dropbox: `/bianchi/research_backups/rei_bianchi/VIGILODE_V1_PORTABLE_S2_20261024.zip`.

Download into a new directory. Verify the ZIP SHA256 and extract it into `$SAVED`.
Do not run `oracle.py` there: that script runs the native probe and overwrites evidence.
Use the committed `readback_saved.py` instead:

```bash
python research/v1_portable_norm_20261024/readback_saved.py "$SAVED" /tmp/v1p_readback_new.json
```

Expected scope:37 original payload hashes and9 original budget-log hashes;741 saved
exact arithmetic cases;36 timing cells and3 paired medians. No compiler, native
solver or benchmark is invoked by that readback. It writes only the specified
new result path. Do not point the output at an old result.

The final S3 companion has `payload/` (the exact S2 members) and `closeout/` (new
recovery report and scripts). Its readback invocation is:

```bash
python closeout/readback_saved.py payload /tmp/v1p_readback_new.json
```

## A separately authorized fresh experiment

The public kernel/test/probe/bench sources use only Rust std. The observed compiler
was Rust1.94.1. C99 benchmark comparison additionally needs a C compiler and libm.
No original compiler binary or benchmark executable is bundled.

Copy source/,oracle.py,intake/lifted_error_norm.{c,h} into a NEW run directory,
make evidence/, then use that directory as cwd:

```bash
rustc +1.94.1 --edition=2021 --test source/tests.rs -o tests
./tests --test-threads=1
rustc +1.94.1 --edition=2021 -C opt-level=2 source/probe.rs -o probe
python oracle.py
cc -std=c99 -O2 -c intake/lifted_error_norm.c -o norm_c.o
rustc +1.94.1 --edition=2021 -C opt-level=3 -C codegen-units=1 \
  source/bench.rs -C link-arg=norm_c.o -C link-arg=-lm -o bench
./bench > evidence/KERNEL_COST_NEW.log
```

These follow the original logged invocations, with a rustup toolchain selector
instead of the historical absolute compiler path. This recovery did not rerun them.
Measure compiler/host/flags/FP-mode and a new budget; do not edit the closed historical
BUDGET or label new wall-time samples as the original ones.

The current kernel assumes gradual underflow and no FTZ/DAZ or fast-math; it does
not test/change those settings. `Indeterminate` is not an accepted native step.
Allocation0 applies only to128 kernel calls in the serial test, not the IO parser.
C99 and Rust timing obligations differ. This is not a complete ODE integrator.
