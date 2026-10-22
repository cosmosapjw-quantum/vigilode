# Handoff: BASS/rei DAG closing steps (NCP C1 campaign) — 2026-10-22

You are taking over the end of a long, governed research run on branch
`research/bassrei-port-20261021` of `github.com/cosmosapjw-quantum/vigilode`. Read `AGENTS.md`
(repo root) first: its rules are binding. The rules that matter most here:

- the five-step ledger order (§2.7);
- no tolerance widening, and no re-scoring of a computed verdict;
- timing is `diagnostic_only` (STATISTICAL_AUTHORITY_HOLD);
- no `Cargo.toml`/`Cargo.lock` edits;
- never write external 40-hex SHAs into documents (bind by sha256).

## Where things are

**Checkout.**

- Worktree: `/home/cosmosapjw/vigilode-bassrei` (branch `research/bassrei-port-20261021`).
- Cargo target `/home/cosmosapjw/vigilode-bassrei-target`; toolchain `RUSTUP_TOOLCHAIN=1.94.1`.

**Push.** Only this exact command, never force:
`git -C /home/cosmosapjw/vigilode-bassrei push origin research/bassrei-port-20261021`.
After pushing, check that `git ls-remote origin research/bassrei-port-20261021` equals local `HEAD`.

**NCP access.**

- Command: `ssh -i /home/cosmosapjw/.ssh/ncp_key -o IdentitiesOnly=yes -o BatchMode=yes -p 22 root@49.50.142.2`.
- Never print or copy the key, and keep host-key verification on (no `StrictHostKeyChecking=no`).
- Read the host state and preserve other runs.
- Delete only inside directories this campaign created.
- Each campaign has a 120-minute hard cap, setup included.
- Failures and not-run items are preserved, never dropped.

**Checkers.** All must pass before any results push:

- `python3 tools/check-research-node.py --base origin/research/ncp-followup-fvm-20261021` → PASS;
- `python3 tools/check-authority-refs.py` → "missing=14" (the baseline; it must not grow);
- `python3 tools/check_ignored_tests_in_ci.py`;
- `python3 tools/test_e4_build_modes.py`;
- `git diff --cached --check`;
- Cargo.toml and Cargo.lock unchanged against `origin/research/ncp-followup-fvm-20261021`.

**Ledger.** `research/LEDGER.jsonl`, one 10-key row per node:
`id, date, commit, command, profile, inputs_sha256, outputs_sha256, claim, verdict, supersedes`.

- The `claim` is one line of at most 600 characters.
- The next ids are **L-0154** (attempt 2) and **L-0155** (attempt 3).
- Copy the format of row L-0153.

## Done (do not redo)

| node | ledger | verdict | notes |
|---|---|---|---|
| P02 / P02b | L-0148 / L-0149 | FAIL / PASS | L-0149 supersedes L-0148 |
| P03 | L-0150 | PASS | structured stage solvers (L2) |
| P04 | L-0151 | PASS | 3 deviations for user sign-off (below) |
| P05 | L-0152 | PASS | accepted-step remap (L3) |
| P06 (scoped) | L-0153 | PASS | full sphere, unpolarized; polarization etc. HOLD |
| L4 | commit `451452dc` | n/a | accept-control `h_next` capped at `max_step` + golden-digest test (final library audit) |

## NCP C1 campaign: state at handoff (2026-10-22, about 15:50 KST)

**The node.** `research/bassrei_ncp_wp_c1_20261022/`. Its `PREREGISTRATION.md` holds the plan, the gates
G1–G7 and two amendments. The harness is `external_audits/vigilode_bassrei_ncp_c1_20261022/` (its
`README.md` has the details).

**Attempt 1 (EXEC `6ed8f526`).** INVALID in setup after 9 s: `cargo` was not on the detached shell's
PATH.

- Its evidence is in `attempts/attempt1_20261022T063359Z/`.
- Fixed by `79a39b8e` and disclosed in Amendment 1.
- No ledger row.

**Attempt 2 (EXEC `e3341309`).** Running, started 06:36:27Z.

- NCP directory: `/root/vigilode-bassrei-ncp-c1-20261022T063627Z`.
- Local receipts: `~/vigilode-campaign/bassrei-ncp-c1-20261022T063627Z`.
- Setup OK, diffsol OK, parity OK. **Julia HOLD**: its depot lived on tmpfs and a power cycle cleared it.
- `inputs_sha256` is frozen in `handoff/attempt2_inputs_sha256.json`.

**Julia install.** User-authorized and online. Running on NCP cores 0-7, at
`/root/vigilode-julia-c1-20261022/`. It is done when `INSTALL_DONE` exists. It succeeded if `install.log`
contains `JULIA_INSTALL_OK` and ends with `exit=0`.

**Attempt 3 (EXEC `2064366e`).** Not launched yet. It is the full preregistered campaign with the Julia
arms (Amendment 2). Its `inputs_sha256` comes from `python3 research/bassrei_ncp_wp_c1_20261022/handoff/inputs_sha256.py 2064366e`.

Status probe (read-only):

    ssh -i /home/cosmosapjw/.ssh/ncp_key -o IdentitiesOnly=yes -o BatchMode=yes -p 22 root@49.50.142.2 \
      'C=/root/vigilode-bassrei-ncp-c1-20261022T063627Z; [ -f $C/CAMPAIGN_DONE ] && echo DONE || echo RUNNING; tail -3 $C/logs/PHASES.log;
       ls /root/vigilode-julia-c1-20261022; tail -5 /root/vigilode-julia-c1-20261022/install.log'

## Your steps

**1. Attempt 2.** When `CAMPAIGN_DONE` exists:

    cd /home/cosmosapjw/vigilode-bassrei
    bash external_audits/vigilode_bassrei_ncp_c1_20261022/fetch_results.sh \
      /root/vigilode-bassrei-ncp-c1-20261022T063627Z ~/vigilode-campaign/bassrei-ncp-c1-20261022T063627Z
    python3 -s external_audits/vigilode_bassrei_ncp_c1_20261022/harness/import_results.py \
      ~/vigilode-campaign/bassrei-ncp-c1-20261022T063627Z/ncp research/bassrei_ncp_wp_c1_20261022/attempt2

- The fetch verifies sha256.
- The import writes `attempt2/results.json` (VERDICT), `attempt2/campaign/*` and
  `attempt2/IMPORT_MANIFEST.json`.
- Do not edit any imported file. The verdict is whatever `analyze.py` computed.
- If the verdict is INVALID (incomplete), do not write a ledger row for it. Record it as an attempt
  instead, as Amendment 1 did.

**2. Julia install evidence.** Copy `install.log`, `ENV_SHA256SUMS.txt`, `INSTALL_START_UTC.txt` and
`INSTALL_END_UTC.txt` from `/root/vigilode-julia-c1-20261022/` into
`research/bassrei_ncp_wp_c1_20261022/julia_install/` with `scp` (same options as ssh, `-P 22`).

- If the install failed, attempt 3 cannot add Julia. Do not launch it.
- In that case, record the failure in the Results and keep attempt 2 as the C1 result.

**3. Attempt 3.** Launch only after step 1 is complete and the install reports success:

    bash external_audits/vigilode_bassrei_ncp_c1_20261022/launch_from_local.sh \
      "$(git rev-parse 2064366e)" --detach

- The full sha appears only on the command line, never in a document.
- The preflight refuses to launch while a C1 run is alive or a process uses ≥ 25% CPU. Wait and retry,
  and never override.
- Poll every 5–10 min with the status probe above, pointed at the new directory the launcher prints.
- When it is done, fetch and import it into `research/bassrei_ncp_wp_c1_20261022/attempt3`.

**4. Ledger rows.** Follow AGENTS.md 2.7 order. The `PREREGISTRATION.md` at each EXEC was already pushed
before its run (step 1), and the inputs are frozen (step 2).

- **Outputs:** take `outputs_sha256` from each attempt's `IMPORT_MANIFEST.json` (prefix each path with
  the node path), plus `DAG_STATUS.json`.
- **Results:** append a `## Results` section to `PREREGISTRATION.md` covering both attempts. Include:
  - the verdict and gate table G1–G7 of each attempt;
  - the matched rungs;
  - the VigilODE G3 margins;
  - comparator coverage per arm (OK / HOLD / typed failure / DEADLINE);
  - the ABBA cost ratios, labelled `diagnostic_only`.
- **Rows:**
  - L-0154 = attempt 2, `commit` = the full sha of `e3341309`;
  - L-0155 = attempt 3, `commit` = the full sha of `2064366e`, `supersedes: null`, because both stand.
- **Command field:** the exact launch command plus the import command.
- **DAG_STATUS.json:** write it in the node, with schema `bassrei-dag-status-v1` like the other nodes and
  `production: "HOLD"`.
- Then run the checkers, commit and push.

**5. Final report to the user.** In Korean, terse.

- **Verdict table:** P02 to P06 and C1 (L-0148 to L-0155).
- **What C1 shows:** accuracy-matched cost ratios are diagnostic only; no speed claim.
- **HOLD list:**
  - polarized transport, tilt, recoil, class B, the sphere solver and sphere AMR;
  - live BASS/rei integration;
  - broad production;
  - wall-time claims.
- **Deviations that need the user's sign-off:**
  - P04: the C2 invariant bound is 1e-14·Σ|terms| (the P02 precedent) instead of 1e-15; the D/E error is
    depletion-weighted; Thomson is weak in every scenario.
  - C1 (prereg "Deviations"):
    - 16×8 auxiliary cases were added for dense VODE;
    - Julia's sparse Jacobian is exact AD of a bitwise port, and CVODE Band uses its own
      difference-quotient Jacobian;
    - the allocation and cross-core checks ran during the ladder window;
    - DOP853 ran only 2 rungs on stiff cases;
    - diffsol is dense-only, with no G4;
    - S3-G3 was dropped, and S4 does not run on G2/G3;
    - ABBA pairs are budget-matched.
  - C1 Amendment 2: the online Julia install, user-authorized.

**6. Optional cleanup.** Ask the user before deleting anything on NCP. The campaign directories and the
Julia depot are kept as evidence by default.

## Known facts that save time

- **Toolchain on NCP:** rustup lives at `/root/.cargo/bin` with the 1.94.1 toolchain installed.
  Julia 1.12.7 is at `/root/VIGILODE_COMPONENT_20261018T134235Z/env/julia-1.12.7+0.x64.linux.gnu/bin/julia`.
- **Python:** the system scipy is 1.11.4. Always use `python3 -s`, because `~/.local` scipy 1.17 breaks
  VODE with user Jacobians.
- **Runners:** they integrate twice per call (warm + measured), so references cost two integrations.
- **Golden test:** the L4 golden-digest test, `crates/rodas5p-integrators/tests/fast_driver_golden_digest_contracts.rs`,
  proves the default driver paths bit-identical to `5fd6e459`.
- **Memory note:** `/home/cosmosapjw/.claude/projects/-home-cosmosapjw-vigilode/memory/bassrei-dag-execution-20261021.md`
  (Claude Code memory) has the same history in short form.

## Note

- `harness/py_worker.py` line 16 still names the rei C15 source by its git blob id (in a comment).
  It is a frozen input of attempts 2 and 3, so it was not edited. The preregistration binds the same file by
  sha256 `a920d0e8…c1c4b1e`.
