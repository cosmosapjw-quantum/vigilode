# v3.6 Next Node — Frozen Full-E Shadow Work/Economics Preflight

Parent: final v3.5 closeout commit.

## Objective
With R-JF still the only committed trajectory, quantify the incremental work and economics of **completing E-K only after** the already-frozen chain:

`k=3 R-side event -> transactional level1+2 prefix cap -> zeta34 <= 13.39706618860016`.

No threshold, persistence, or prefix-budget retuning is allowed.

## Exact first action — no new solver run
Use the already-durable `prefix_work` and `audit_full_e_work` fields from v3.5 rows to reconstruct

\[
W_{\rm cont}=W_{\rm audit\,fullE}-W_{\rm prefix}
\]

for every frozen recommendation on N=96/192/256/320/384. Join each event to the target R-JF attempt work and report per-event and cumulative continuation/JVP ratios. This is a ledger analysis, not a new solver campaign.

## If the ledger preflight survives
Implement a read-only runtime shadow that resumes the retained level-2 prefix (no prefix recomputation), completes E-K only for frozen recommendations, records incremental continuation work as speculative, and leaves R-JF state/controller/requested outputs exactly unchanged.

## Hard requirements
- unsafe E recommendations = 0 on any new shadow holdout;
- no hidden/discarded work;
- prefix exhausted events remain immediate R-JF abstentions;
- R-JF committed trace exact parity;
- full-E shadow work is explicitly counted, not labeled as free audit work;
- no active switching;
- N=2048 remains sealed.

## Decision point
If full continuation tail work recreates the old P1-01 overhead pathology, stop and design a separate continuation transaction/cap before forced-switch studies. Only if shadow economics and safety survive may the DAG open forced-switch fifth-order recovery.
