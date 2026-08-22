# v3.7 Timing Authority Validator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the already-sealed whole-campaign timing authority validator without producing a new paired-wall campaign.

**Architecture:** A Python standard-library module captures immutable source/toolchain/binary/host/thread attestations and validates complete five-profile campaign directories. It rejects or accepts only whole campaigns, retains every pair and failed attempt, and summarizes up to five attempts into the sealed three-passing-campaign decision. Existing v3.6 economics JSON remains read-only input; no solver path is modified.

**Tech Stack:** Python 3 standard library, Bash, committed v3.7 contract JSON, existing v3.6 economics JSON.

**Spec:** `docs/superpowers/specs/2026-08-23-v38d-high-entropy-performance-tournament-design.md`

## Global Constraints

- Contract: `research/generic_timing_replication_continuation_transaction_v37/contracts/V37_TIMING_REPLICATION_CONTINUATION_TRANSACTION_CONTRACT.json`.
- Exact profiles: 96, 192, 256, 320, 384; each has one warm-up pair and seven measured alternating-order pairs.
- Require exact Git/toolchain/binary/contract identity, clean tree, affinity, host fingerprint, and thread environment.
- Require idle fraction `>=0.90`, steal fraction `<=0.001`, swap deltas `=0`, exposed thermal-throttle deltas `=0`, arm spans `<=1.5`, order-median gap `<=0.1`.
- No individual pair/profile exclusion; verdict cannot depend on favorable ratio direction.
- Require three passing campaigns within at most five attempts.
- No new wall campaign, speedup claim, active switching, policy retuning, or N=2048.

---

### Task 1: Contract loader and CPU-stat arithmetic

**Files:**
- Create: `research/generic_timing_replication_continuation_transaction_v37/scripts/timing_authority_validator.py`
- Create: `research/generic_timing_replication_continuation_transaction_v37/scripts/test_timing_authority_validator.py`

**Interfaces:**
- Produces: `ValidationError`, `sha256_path`, `load_json`, `load_contract`, `parse_proc_stat_cpu`, `cpu_idle_steal_fractions`.

- [ ] **Step 1: Write the failing tests**

```python
class ContractAndCpuStatTests(unittest.TestCase):
    def test_contract_thresholds_are_exact(self):
        timing = load_contract(CONTRACT)["timing_replication"]
        self.assertEqual(timing["profiles"], [96, 192, 256, 320, 384])
        self.assertEqual(timing["required_passing_campaigns"], 3)
        self.assertEqual(timing["maximum_campaign_attempts"], 5)
        self.assertFalse(timing["quality_rules_reference_ratio_direction"])

    def test_proc_stat_uses_sealed_eight_fields(self):
        before = parse_proc_stat_cpu("cpu 10 1 4 80 2 1 1 0 0 0\n")
        after = parse_proc_stat_cpu("cpu 12 1 5 170 2 1 1 0 0 0\n")
        idle, steal = cpu_idle_steal_fractions(before, after)
        self.assertAlmostEqual(idle, 90 / 93)
        self.assertEqual(steal, 0.0)
```

- [ ] **Step 2: Verify RED**

```bash
cd research/generic_timing_replication_continuation_transaction_v37/scripts
python -m unittest test_timing_authority_validator.ContractAndCpuStatTests -v
```

Expected: module import failure.

- [ ] **Step 3: Implement the minimal arithmetic**

```python
class ValidationError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValidationError(message)


def parse_proc_stat_cpu(text: str) -> tuple[int, ...]:
    line = next((line for line in text.splitlines() if line.startswith("cpu ")), None)
    require(line is not None, "aggregate cpu line missing")
    values = tuple(int(value) for value in line.split()[1:9])
    require(len(values) == 8, "sealed cpu field count mismatch")
    return values


def cpu_idle_steal_fractions(before, after):
    delta = tuple(b - a for a, b in zip(before, after))
    require(len(delta) == 8 and all(value >= 0 for value in delta), "invalid cpu delta")
    total = sum(delta)
    require(total > 0, "cpu total delta is zero")
    return delta[3] / total, delta[7] / total
```

`load_contract` must fail unless the exact schema and every sealed timing threshold match.

- [ ] **Step 4: Verify GREEN and commit**

```bash
python -m unittest test_timing_authority_validator.ContractAndCpuStatTests -v
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "test: establish v3.7 timing authority validator core"
```

### Task 2: Deterministic host and execution attestation

**Files:** Modify the Task 1 module and tests.

**Interfaces:**
- Produces: `capture_attestation(repo_root, binary, contract, preflight_seconds)`, `capture_host_fingerprint`, `capture_preflight`, `thread_environment`, `allowed_affinity`.

- [ ] **Step 1: Write failing mocked capture tests**

Assert a stable `vigilode-v37-timing-host-attestation-v1` object containing Git HEAD/tree/clean, Rust/Cargo text, binary and contract SHA-256, measurement profile, host fields, affinity, all seven thread variables including unset values, CPU before/after, idle/steal, swap, and thermal maps.

- [ ] **Step 2: Verify RED**

```bash
python -m unittest test_timing_authority_validator.AttestationTests -v
```

- [ ] **Step 3: Implement capture fail-closed**

Stable output keys:

```python
{
  "schema": "vigilode-v37-timing-host-attestation-v1",
  "git": {"head": str, "tree": str, "clean": bool},
  "rust": {"rustc_vv": str, "cargo_version": str},
  "contract_sha256": str,
  "binary_sha256": str,
  "measurement_profile": "measurement",
  "host": {
    "kernel": str, "cpu_model": str, "logical_cpu_count": int,
    "physical_core_count": int | None, "microcode": str | None,
    "numa_node_count": int | None, "frequency_governor": str | None,
    "boost_or_turbo_state": str | None
  },
  "cpu_affinity": list[int],
  "thread_environment": dict[str, str | None],
  "preflight": dict[str, object]
}
```

Unavailable thermal counters are `{}` and do not fail alone. Dirty tree, missing binary, missing aggregate CPU stat, or nonfinite fractions fail capture.

- [ ] **Step 4: Verify GREEN and commit**

```bash
python -m unittest test_timing_authority_validator.AttestationTests -v
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "feat: capture sealed timing host attestation"
```

### Task 3: Exact campaign layout and pair validation

**Files:** Modify validator and tests.

**Campaign layout:**

```text
attempt-01/
  ATTESTATION.json
  profiles/
    calibration96.json
    calibration192.json
    calibration256.json
    holdout320.json
    holdout384.json
```

**Interfaces:** Produces `validate_campaign(contract, campaign_root) -> dict`.

- [ ] **Step 1: Write failing valid-layout and missing-pair tests**

Synthetic fixtures must build the exact five profiles and pair rows. Tests assert 35 retained measured pairs and five warm-ups; removing one measured row rejects the entire campaign but still reports five retained profile files.

- [ ] **Step 2: Verify RED**

```bash
python -m unittest test_timing_authority_validator.CampaignLayoutTests -v
```

- [ ] **Step 3: Implement structural validation**

For every arm and pair validate mode, repetition count, six-family identity, finite positive wall/interval/Gamma, exact `Gamma=wall/interval`, alternating order, exact paired denominator, and exact file set. Preserve hashes and every offending row in the result.

- [ ] **Step 4: Run compatibility tests and commit**

```bash
python -m unittest test_timing_authority_validator.CampaignLayoutTests -v
python -m unittest ../../../generic_frozen_full_e_shadow_v36/scripts/test_analyze_shadow_economics.py -v
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "feat: validate complete timing campaign layout"
```

### Task 4: Host-quality gates and ratio-independent whole-campaign verdict

**Files:** Modify validator and tests.

- [ ] **Step 1: Write failing threshold tests**

Test each sealed threshold and a mutation where the wall ratio changes from 0.6 to 1.4 while every quality field stays fixed; quality verdict must be identical. Inflate one N=384 R-JF wall by 2x and assert whole-campaign failure with all 35 pairs retained.

- [ ] **Step 2: Verify RED**

- [ ] **Step 3: Implement named failures**

```text
git-identity, rust-toolchain, measurement-binary, contract-hash, clean-tree,
host-fingerprint, cpu-affinity, thread-environment, cpu-idle, cpu-steal,
swap-in, swap-out, thermal-throttle, profile-pair-cardinality,
proposed-interval, rjf-arm-span, shadow-arm-span, order-median-gap
```

The decision path must not read ratio direction except to preserve raw evidence.

- [ ] **Step 4: Verify GREEN and commit**

```bash
python -m unittest test_timing_authority_validator.HostQualityTests -v
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "feat: enforce whole-campaign timing host quality"
```

### Task 5: Three-passing-within-five summary

**Files:** Modify validator and tests.

**Interfaces:** Produces `summarize_attempts(contract, attempt_results) -> dict`.

- [ ] **Step 1: Write failing tests**

Assert `[pass, fail, pass, pass]` yields `PASS_HOST_QUALIFIED_DESCRIPTIVE_TIMING`; five attempts with only two passes yields `HOST_UNSUITABLE_NO_TIMING_PROMOTION`; six attempts raises `ValidationError`.

- [ ] **Step 2: Implement minimal summary**

Retain every attempt path/hash/verdict and explicitly emit:

```json
{
  "speedup_claim_authorized": false,
  "active_switching_authorized": false,
  "individual_pair_exclusion_used": false,
  "individual_profile_exclusion_used": false
}
```

- [ ] **Step 3: Verify and commit**

```bash
python -m unittest test_timing_authority_validator.AttemptSummaryTests -v
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "feat: summarize sealed timing campaign attempts"
```

### Task 6: CLI and atomic output

**Files:**
- Modify validator module/tests.
- Create `run_timing_authority_validator_selftest.sh`.

**CLI:**

```text
capture --repo ROOT --binary BIN --contract CONTRACT --output ATTESTATION.json
validate-campaign --contract CONTRACT --campaign-root ATTEMPT --output DECISION.json
summarize --contract CONTRACT --attempt-result RESULT.json ... --output SUMMARY.json
```

- [ ] **Step 1: Write subprocess CLI tests**

Pass must return 0 and stable JSON. Malformed campaign returns 1 with no partial authority output. Argument error returns 2.

- [ ] **Step 2: Implement `argparse` and atomic replace**

Write to `suffix + ".tmp"`, flush, `os.fsync`, then `os.replace`; remove temporary output on exception.

- [ ] **Step 3: Verify and commit**

```bash
python -m unittest test_timing_authority_validator -v
bash run_timing_authority_validator_selftest.sh
git add research/generic_timing_replication_continuation_transaction_v37/scripts
git commit -m "feat: expose timing authority validator CLI"
```

### Task 7: Retrospective v3.6 diagnostic

**Files:**
- Create `results/V36_RETROSPECTIVE_TIMING_QUALITY_DIAGNOSTIC.json`.
- Create `reports/TIMING_AUTHORITY_VALIDATOR_RESULT.md`.
- Modify tests.

- [ ] **Step 1: Write failing diagnostic test**

Load all five committed v3.6 economics JSON files and assert 35 retained pairs, N=384 R-JF/shadow span failures, and `historical_verdict_rewritten=false`.

- [ ] **Step 2: Generate deterministic diagnostic**

Historical host counters are `null` with status `NOT_RECORDED`; never invent pass/fail. N=384 profile failures classify the whole historical campaign non-authority under the new contract while preserving historical `PASS_DESCRIPTIVE_ECONOMICS`.

- [ ] **Step 3: Run twice, compare SHA-256, and commit**

```bash
python -m unittest test_timing_authority_validator.RetrospectiveDiagnosticTests -v
git add research/generic_timing_replication_continuation_transaction_v37
git commit -m "docs: seal v3.7 timing validator result"
```

### Task 8: Focused verification, one independent review, and PR checkpoint

- [ ] **Step 1: Run focused verification**

```bash
source /mnt/data/rust_1_94_1_env.sh
cd research/generic_timing_replication_continuation_transaction_v37/scripts
python -m unittest test_timing_authority_validator -v
cd ../../../../
python research/generic_frozen_full_e_shadow_v36/scripts/test_analyze_shadow_economics.py
cargo fmt --all -- --check
```

- [ ] **Step 2: Independent review questions**

Can favorable ratios alter verdict? Can one bad pair be omitted? Are unavailable host fields distinct from pass? Can fewer than three attempts promote? Does any command generate new wall output?

- [ ] **Step 3: Checkpoint and open one implementation PR**

Record branch/HEAD/tree, exact commands/exits, hashes, unresolved P2/P3 debt, and next action. Claim ceiling: validator implementation only.
