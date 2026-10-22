# Complete raw benchmark records

`RAW_OUTPUTS.tar.gz` contains every numbered stdout/stderr file, including failed configurations, warm-up records, and all three timed repeats. `RAW_OUTPUTS_MANIFEST.json` records each uncompressed SHA256 and the archive SHA256. Originals are retained locally and ignored in Git to avoid duplicate storage.

From this directory, recover the exact files with:

```sh
tar -xzf RAW_OUTPUTS.tar.gz
```

The filenames in `RECEIPTS.json` and `SUMMARY.json` then resolve directly. Re-run `../../analyze_local_bench.py` with this directory as its argument to regenerate the numeric summary and figures. No benchmark rerun is required to inspect evidence.
