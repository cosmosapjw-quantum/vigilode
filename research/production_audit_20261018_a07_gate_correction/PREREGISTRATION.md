# A07 Robertson gate-report correction

## Frozen contract

- Parent campaign source identity: `01c5b93357ec4318e06e36d938deedf1b2db4a3f`.
- New correction execution source is recorded in `SOURCE_IDENTITY.json` after the preregistration commit is published.
- Problem and physical tolerances are unchanged: Robertson on `[0,40]`, `rtol=1e-8`, `atol=[1e-11;3]`, 101 requested outputs.
- Reuse the already validated immutable campaign at `research/production_audit_20261018/evidence/continuation_a07/validated.json`; do not rerun solver arms.
- This correction may repair threshold descriptions and derive reporting fields from the retained validator output. It must not change thresholds, arm outputs, reference data, campaign rows, or timing authority.

## Question and acceptance

The published follow-up described `0.05` as the solver-arm accuracy threshold. Verify the validator contract and correct the reports/schema to state separately:

1. reference-spread gate: every empirical reference spread `<=0.05`;
2. per-arm accuracy gate: `max_grid_wrms_distance + empirical_reference_uncertainty <=1.0`, with all rows successful;
3. cross-arm empirical-interval overlap gate: required in addition to the previous checks for the combined accuracy comparison.

Acceptance requires a reproducible extraction from the retained `validated.json`, exact agreement of corrected report values and gate booleans, no campaign rerun, and continued `HOLD` for matched-accuracy speed because Diffsol fails the per-arm gate and intervals do not overlap. The empirical reference remains non-rigorous.

## Bounded validation

After publishing this preregistration as an execution commit, hash the preregistration and immutable campaign/validator inputs. Run only a local report-contract verifier that reads retained outputs and checks the stated gates; preserve stdout, stderr, exit code, and output hashes. Then append results and a ledger row. No numerical solver execution is registered or authorized by this correction.
