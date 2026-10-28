# Runner and local-plan review closure

**Disposition: pass_scoped.** The concrete cancellation and provenance
findings in `DEVELOPMENT_PLAN_REVIEW.md` are closed for the documented
POSIX/WSL command contract. No remaining blocking finding was identified in
this reviewed dependency set. Exact reviewed hashes and each closure are in
`RUNNER_CLOSURE_REVIEW.json`.

This review read the fixed code, generated plan, seven actual parent-executed
helper-test receipts and their test source. It did not modify any runtime or
solver source and did not execute a native solver, build, benchmark or profile.
The two review files are the only new artifacts written by this reviewer.

## Process ownership and cancellation

The new `record_command.py` launches a command in a dedicated process group.
Timeout and SIGINT/SIGTERM cancellation preserve the actual command exit code
separately from wrapper qualification, send SIGTERM then SIGKILL to that owned
group, and retain a final receipt. Surviving group members are also addressed
after a direct command exits, before the suite releases its lock.

The recorder's signal handler stores a pending signal rather than raising
during `Popen`, so the child handle is available before cleanup. The outer
`run_local.py` forwards any pending signal after its own child handle is
assigned. Its nonblocking flock retry loop checks cancellation rather than
blocking indefinitely while another native command holds the lock.

The instruction profiler now launches native/Valgrind children in the outer
owned group. Its per-child timeout terminates the direct Valgrind/native
process and exits the helper; remaining same-group descendants are then
covered by the outer recorder. A new detached session inside the profiler
would have escaped that ownership and has been removed.

The supported command contract is explicit: helpers remain in this owned
group and do not daemonize or create detached native sessions. These fixes
concern execution and evidence capture; completed scientific profile metrics
remain historical results with their original source/binary receipts.

## Source and input identity

The recorder rescans the source/input roots before and after execution,
detecting added or removed files as well as modifications. It records actual
RAYON and Rustdoc environment values, the resolved executable content, and
the plan supplied through `--input`. A child exit0 with changed input now
produces wrapper exit2 and status `stale_source`.

The profile helper rejects duplicate arm identifiers that would overwrite
the same filenames, checks finite positive timeouts, rescans native sources,
and returns failure when source or binary identity changes. No solver rerun
was needed to inspect these operational corrections.

The standalone smoke's narrower facade/config source list remains supplemented
by the outer recorder in the prescribed suite. Deployment CPU, compiler,
libm and affinity evidence still belongs to the future local L-RELEASE and
L-HARDWARE gates; this code review does not invent a completed host campaign.

## Actual focused execution evidence

The parent's `evidence/runner_contracts/SUMMARY.json` names seven successful
contract checks. Its runner hash exactly equals the reviewed file:

`c9d60c39301d218ee38f20a65fca0b5834741a24df75826c944ff66915ad996a`.

| Contract | Observed wrapper result | Reviewed obligation |
|---|---:|---|
| Successful command |0|Actual environment keys and unchanged source retained|
| Existing input mutation |2|Child exit0 retained; qualification is stale|
| Child failure |7|Original nonzero status preserved|
| Newly discovered configuration |2|Input addition detected by rescan|
| Timeout with resistant grandchild |124|Owned descendant no longer executes; SIGKILL recorded|
| SIGTERM with resistant grandchild |143|Same descendant cleanup after cancellation|
| Missing executable |127|Launch error retained|

The tests use owned toy processes and check that the resistant descendants
are absent or terminated/zombie rather than claiming every PID was reaped
by the recorder. These tests do not establish solver convergence or CPU
performance. The narrow spawn/lock race fixes were separately inspected in
code; the test count is not expanded to claim exhaustive scheduler coverage.

## Machine plan and executable suites

The revised canonical DAG has16 unique nodes and8 suites, with no cycle or
missing dependency. The structural benchmark argument positions match its
native CLI and now cover n1, n64 and n2048, including one explicit two-thread
cell. The adapter suite calls its paired helper, and a separate profiling
suite contains the arm/repetition templates consumed by the native programs.
Receipt outputs now name `SUITE_RECEIPT.json` and the actual `command_XX`
receipt/source/stdout/stderr paths.

PR-N06 explicitly states the temporal variation/dwell assumptions and
residence/exploration-loss obligations previously present only in the adapter
lane. Its dependencies on target-hardware evidence and PR-N01 cost evidence
are coherent. Existing SPD04 workspace reuse and the nonnormal Givens
counterexample remain correctly incorporated.

The package verifier now requires `RAW_MANIFEST.json` for full evidence
verification. Omitting archive checks requires explicit `--source-only`,
which cannot be combined with extraction. Final archive generation and
verification remain the parent's packaging task; this review does not mark
unexecuted artifact checks complete.
