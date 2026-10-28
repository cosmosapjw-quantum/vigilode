# R4 statistics preregistered contract

Source: `1c54194123ee6abc6daa512e8574922f510b4e2c`; R3 source/output are immutable evidence.
No scientific execution before root confirms PREREG_COMMITTED. Production code is read-only.

## Audit questions and gates

1. Does R3 raw admission close its five original malformed variants? Public `admit` and fresh assessment must reject each.
2. Does `PairedTimingEvidence::verified_decision` enforce the documented `STATISTICAL_AUTHORITY_HOLD` from both failed coverage studies? A structurally valid synthetic six-session, one-case 1.3-speedup receipt with exact A/A=1 is the control. An unchanged `Promote` is evidence that the hold is documentation-only, not a new observation about coverage.
3. Does receipt admission enforce complete per-session raw data? Mutations after the first session: empty warmups, one warmup, and remove one candidate/reference sample from session 1 while adding it to session 2 (merged vectors remain unchanged because constant samples). Also test end-before-start provenance, wrong batch, and missing session case as controls. Recompute the evidence from changed raw records; mutation acceptance is source/implementation evidence, not a demonstration of a malicious real campaign.
4. Replay published POLY03 and HOM06 evidence through current public verifier; preserve serialization/replay failures. Inspect published coverage/campaign files without rerunning timing studies. Do not reinterpret acknowledged negative results as new defects.

Command (after prereg commit): `cargo run --manifest-path r4/statistics/probe/Cargo.toml --offline --locked -- <repo-root>`; shared build lock is mandatory. Native stdout JSONL, stderr and exit status retained. Compile-only harness corrections must be disclosed.

## Mathematical/coding advancement

Candidate: exact finite-sample simultaneous median intervals from independent sessions for a fixed declared case corpus. Explicit candidate estimand is median over cases of each case's population median **session-cell median log speedup**. It equals the existing symmetric additive design's theta, but need not equal the current pooled-pair estimand in arbitrary distributions; no silent production substitution.

For each of C complete cases and S independent sessions choose the largest k>=1 with 2*P[Binomial(S,1/2)<=k-1] <= alpha/C. The interval is [Z_(k), Z_(S-k+1)]. If no such k exists return (-infinity,+infinity) and Inconclusive. Take medians of component lower/upper endpoints. Union bound permits arbitrary dependence across cases, but requires independent identically distributed session vectors and non-informative inclusion of complete sessions. Missing cells/failures reject, not impute/drop. Ties are conservative for a specified median. No bootstrap/Monte-Carlo gate is needed for this finite combinatorial coverage statement.

Run `python3 r4/statistics/exact_session_interval.py --output r4/statistics/EXACT_SESSION_INTERVAL.json`. Exact Fraction/binomial checks S=1..16, C in {1,5}, alpha=1/20; enumerate all 2^S signs for S=6,8,12 and compare exact endpoint coverage. Test matched control, all-above/all-below and threshold ties; duplicated session ID, missing case, NaN are rejected. Acceptance: every exact combinatorial equality holds, claimed simultaneous lower bound >=0.95 or interval explicitly unbounded; S=6,C=1 finite and S=6,C=5 unbounded, S=8,C=5 finite. No population speed or performance claim.

Stop after one native probe and one exact enumeration run, except one bounded build/runtime repair or an actual code defect correction. An independent reviewer owns final research claim admission. No production edits, long wall-time campaign or reseeding.
