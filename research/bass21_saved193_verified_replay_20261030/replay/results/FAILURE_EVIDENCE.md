# Prior failure record, 2026-10-30

A first attempt at the **new archive's** `bash VERIFY.sh` returned exit code **1** before any physical source replay, with `FileNotFoundError: [Errno 2] No such file or directory: 'replay/MANIFEST.json'`. This was a packaging-order/verification bootstrap error, not a BASS model/theory failure. No scientific test failed in that attempt; tests did not run because the manifest was not yet generated.

Corrective action: generate source file list and 46 manifest SHA/size entries prior to running `VERIFY.sh`; rerun. Subsequent `VERIFY.sh` returned **exit code 0**, 46/46 manifest rows, original source ZIP3 CRC PASS, fresh 20/20 Python tests, exact recomputation byte identity of JSON/CSV/native FJ NPZ.

Previous PR118 source-verification `bash new_physics/VERIFY.sh` separately returned **exit code 0** and 13/13 tests; this does **not** fix the already-failing research-node preregistration/ledger CI of PR118. R6/C9 distinct-author independent validation still NOT_RUN. This file is a faithful summary of executed error output, not an original raw stderr capture of the first failed attempt.
