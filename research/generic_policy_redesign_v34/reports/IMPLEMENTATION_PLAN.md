# v3.4 Sealed Holdout Plan

1. Commit this contract before any N384 output.
2. Run the existing frozen `stage-growth-holdout-384` safety-audit profile family-by-family.
3. Apply the committed tau exactly; no selector or feature fitting is allowed.
4. Compute pooled and groupwise unsafe/recommendation counts and check every hard gate.
5. If any hard gate fails: reject policy and keep N2048/active switching sealed.
6. If every hard gate passes: commit raw holdout hashes and verdict before opening any economics/order node.
