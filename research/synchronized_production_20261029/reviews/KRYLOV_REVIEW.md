# Independent Krylov reviews before execution

Source baseline: `bc5ddbe42b24b1a6364bf653c3e2d6d938ca4087`.

Three separate reviews (root, state_audit, krylov_theory/proof_audit) read K1–K4 and the actual source. They accepted the scoped proof and K1 implementation contract. The child reviewer independently checked 75 exact-rational polynomial cases and the K4 defect sign. These are theory checks, separate from the subsequently published native execution source.

The theory review clarified three statements before code publication:

1. K4 residual transfer keeps b fixed; changing b adds Δb.
2. A metric epoch change invalidates norm/error certification, while the purely algebraic shifted-image identity still holds if J/U/shift assumptions hold.
3. Existing paired QR and rank-floor code implements a numerical policy; a rigorous outward bound on the transformed image defect remains a separate obligation.

The child then reviewed the production diff and test sources without executing them. K1 uses the proved live-field separation, public reserve remains full, and legacy/augmented prepare still sizes directions. The new tests address cold storage, reserved storage, shape-changing mixed histories, monitored prefixes and actual augmented reuse of poisoned prior directions. Existing `rnext02_gmres_into_contracts` and `reaudit_monitored_progress` supply failure/output/progress regression coverage.

The K3 oracle review found one admission ambiguity: no requested accuracy cap must not return an accuracy PASS. It was corrected before publication to `admitted=None` when `error_cap=None`, with a regression assertion. An explicit cap returns a boolean decision; a rejected cap retains its candidate, residual and error bound. Exact coefficient algebra, original residual, symmetric-part Gershgorin bound, representation-pole refusal and d+1 matrix-action accounting were accepted by source inspection.

Verdict at this point: **ACCEPT_SCOPED_FOR_EXECUTION**. Actual compile/tests, numeric benchmark and final integrated result are recorded by the parent campaign after publication. This review does not qualify native directional-quotient dispatch or a replacement MGS/QR algorithm.
