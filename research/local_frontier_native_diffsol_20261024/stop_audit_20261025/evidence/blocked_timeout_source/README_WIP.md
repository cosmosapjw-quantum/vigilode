# Timeout diagnostic runner v2 contract

This directory contains synthetic runner tests. No solver execution or build is
authorized by these files. Real materialization requires a clean source commit,
a frozen built binary, and a separate independent review of the final plan.

`vigilode.local-continuation-runner.v2` is opt in through the plan schema; v1
continues to use its original campaign binding and retry rules. V2 selects exactly
one entry from the fixed catalog template (path and SHA are runner constants).
It requires the unchanged original command arguments and original receipt/task
identities. Only the executable path is substituted with the frozen binary.

The plan requires `source`, `runner`, `catalog`, `build_receipt`, `output`,
`watchdog`, `tasks`, `predecessor`, `cost_root`, `deadline_utc`, `new_cap_seconds`,
and `approval`. File bindings are `{path, sha256}` and paths are canonical absolute
paths. `runner.path` is the executing runner file. Sources contain exact clean
HEAD/tree identities. The task includes `original_task_sha256`, its exact receipt
and SHA, `binary`/`binary_sha256`, CPU `[0]`, one thread, null
`physical_progress_target`, and the catalog `wave`/`depends_on_case`.

The exact watchdog is poll 5s, idle 120s, hard stop 300s, at most 180 samples,
aggregate process group RSS 12 GiB, reservation 960s, extension disabled. CPU ticks
and output bytes establish activity only. Every observed live group member must
remain on CPU0 with one OS thread. A Linux subreaper kills the group and reaps
adopted descendants on every terminal path, including controller termination;
`CLEANUP.json` and the receipt retain group resource and reaping evidence.
SIGKILL or machine failure requires the existing interrupted-attempt reconciliation
before further cumulative cost admission.

The build receipt schema is `vigilode.timeout-diagnostic-build.v1`, with
`status: COMPLETED`, integer `exit_code: 0`, the exact `source`, `binary`,
`binary_sha256`, and a nonempty build `command`. This receipt records an already
performed build; the runner never builds.

The approval schema is `vigilode.timeout-diagnostic-approval.v1`, with
`decision: APPROVED_ONE_DIAGNOSTIC`, distinct nonempty `approver` and `implementer`,
`independent_review: true`, and `plan_core_sha256`. The core is the whole plan
excluding only `approval`, serialized using the runner's canonical SHA function.
The approval also repeats exact `original_task_sha256`, `original_receipt_sha256`,
`source`, `binary`, `binary_sha256`, `runner`, `catalog`, `build_receipt`, `output`,
`deadline_utc`, and `new_cap_seconds`. A profile or approval field is not evidence
of an observed reviewer runtime; the owner must obtain the separate review.

Wave 1 requires `predecessor: null`. Wave 2 requires three file bindings named
`binding`, `receipt`, and `review`. The binding must be the consumed wave 1 task
binding for `depends_on_case`; the receipt must belong to its output and retain
successful group cleanup. The review schema is
`vigilode.timeout-diagnostic-predecessor-review.v1`, with
`decision: APPROVED_NEXT_DIAGNOSTIC`, `independent_review: true`, distinct
`approver`/`implementer`, exact `binding_sha256`, `receipt_sha256`,
`next_original_task_sha256`, `source`, and `binary_sha256`.

Under the existing shared mutex the runner checks recursive attempts, controller
charges and external cost reservations, then durably creates
`DIAGNOSTIC_OUTPUT_BINDINGS/<original_task_sha256>.json` before START or spawn.
The binding permanently consumes that original task and its unique output.
Replay, renamed tasks, alternative outputs, retries and reused output paths are
refused. An interrupted or failed attempt does not reset that identity or budget.
Only one new attempt can start in any invocation. V2 preserves the legacy global
campaign binding without using it for new diagnostic outputs.

Run the synthetic suite from the checkout root:

```sh
python3 -m unittest discover -s research/local_frontier_native_diffsol_20261024/timeout_runner_followup -p 'test_*.py' -v
```

Set `TIMEOUT_RUNNER_TEST_EVIDENCE` to a new directory outside the checkout to retain
test fixtures, attempts and cleanup receipts. All runtime and timing evidence
remains `diagnostic_only`; scientific admission remains `HOLD`.
