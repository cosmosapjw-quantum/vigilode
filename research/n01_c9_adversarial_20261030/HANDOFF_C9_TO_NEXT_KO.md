# VigilODE C9 adversarial scoped review → next independent reviewer / C10 conditional

$physmath-research-loop $scispace $exa $wolfram

PROJECT=cosmosapjw-quantum/vigilode
STAGE_EXECUTED=C9_ADVERSARIAL_SAME_ASSISTANT_SCOPED
NEXT_REQUIRED_STAGE=C9_DISTINCT_AUTHOR_INDEPENDENT_REVIEW
FOLLOWING_STAGE_ONLY_AFTER_ACCEPT=C10_RELEASE_DECISION_AND_HANDOFF
AUTO_NEXT_NODE=false
PR_PARENT=125
PR_PARENT_HEAD=8afb3cef7f237b0c7ac26ec7b647694df23a7789
PARENT_C8_NATIVE_SOURCE_BLOB=e6a33b66ba80055cae9439cf683a1715ca77c5a2

ORIGINAL_DAG_VIGILODE=7/23
CENTRAL_REI_DAG=28/88
SUCCESSOR_ADMITTED=0/12
N01_C6=1/1_SCOPED
N01_C7=1/1_SCOPED
N01_C8=1/1_SCOPED
N01_C9_INTERNAL_TECHNICAL_AUDIT=1/1_SCOPED
N01_C9_DISTINCT_AUTHOR=0/1_NOT_RUN
PRODUCTION=HOLD

PRIMARY_ARCHITECTURE:
- causal parallelized RHS-only derivative-free Newton-free accelerated homotopy
- matrix-free/JVP only optional secondary fallback
- dense 3x3 only independent small correctness oracle

C9_FINDINGS:
F01_SOURCE_BINDING: general source admission BLOCK_NOW. The original test-only bound takes free `case.k,y,stage_k,y_out`, no opaque owner/trial/epoch. Exact forged fake k=(0,0,0) gives residual0, true owner k=(1/2,10,200) gives first raw-U residual (7/2048,93/2048,-25/512), max25/512 at tau1/64. Validity for fake equation != authority for true equation.
F02_GLOBAL_IVP: C8 moderate n8 true endpoint IVP ℓ1 error ~9.63953e-10 vs max one-step algebraic bound ~3.30584e-12 (291.5908x). Not a contradiction. Need truncation and error growth before whole-IVP claims.
F03_WORK_PRECISION: RODAS and BE achieved-error ranges do not overlap in tested mild/moderate cases. No matched-achieved-error cross-method work ratio. Within-BE scalar map calls are comparable.
F04_REFERENCE: new independent mpmath 40/75 dps Taylor reference mild/moderate, stable ~1e-35/1e-34; Radau vs mp75 ~7e-17/2.3e-16; BDF vs mp75 ~2.46e-13/1.73e-12. NOT interval certified; stiff source reference not revalidated.
F05_FORMULA: exact Fraction 512 source/J norm cases, 504 positive Neumann cases, 128 exact nilpotent causal transfers pass; conditional formulas retained.
F06_BACKLOG: C8 test-only verifier contains unchecked unwrap on interval failures, no valid-input crash reproduced.

C9_NEXT_REVIEWER:
- Receive this exact source/ZIP with byte hash, read the C8 original code and the C9 exact/MP scripts.
- Independently reproduce F01 with ACTUAL native test and check source-owner identity/epoch/clock gating; attack C8 κ, positive tube, raw U/K coefficient allowance, rounded output and parallel work.
- Check that F01 is a promotion blocker, not a refutation of source-matched C8 measurements. Find and fix only concrete source defects, bounded once.
- If satisfied, return independent actual reviewer identity, evidence hashes, red/green commands and promote C9 distinct-author gate; only then proceed to C10.
- Do not broaden solver defaults, alter frozen RODAS coefficients, merge to main, weaken tolerances, or claim production release.

RETURN=PR_HEAD, reviewer ID, actual source and check identity, failures, exact physics, test exits, official DAG counters, Drive and Dropbox ACKs, exact next handoff.
