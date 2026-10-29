# Tool and source provenance

## Execution policy

Construction policy was `HOST_CODEX_ONLY`. No local LLM was used for code
generation, scientific classification, review, claim admission, or evidence
interpretation. The Bateman six-case candidate suite was not executed.

## Supplied harnesses

| Artifact | SHA-256 | Observation |
|---|---|---|
| `physmath-research-harness-gpt56(20260827-091541).zip` | `9adde688f8020e7feb2c1c0304b3204dbe70dd01e2d87e64a5c4eb357c019934` | Byte-identical to the supplied untimestamped research-harness copy; versioned research/evidence/stop-rule materials inventoried |
| `physmath-coding-harness-gpt56(20260827-091536).zip` | `6e67e999a0c19f6ed9de7c339067cc11691d5cf5cb662a11756d8fc393c849b4` | Byte-identical to the supplied untimestamped coding-harness copy; contract/TDD/handoff materials inventoried |
| `VIGILODE_THREAD_ARCHIVE_RECOVERED_20260830(1).zip` | `c112309cab3e431ca563dd11dc1f67d95df0bfa85c8081251c33bea16ca44cfb` | Metadata and handoff-continuity documents only; recovery snapshot of 183 files, not a complete conversation export or scientific authority |

The harnesses govern process and packaging. They are not numerical evidence
for the Bateman candidate.

## Connected sources and computations

| ID | Source/tool | Use | Authority limit |
|---|---|---|---|
| P-GITHUB | GitHub read-only PR/commit/workflow readback | Bound PR #38 to head `f954e391...`, tree `4314da2...`, OPEN/DRAFT/UNMERGED state, and five successful workflow runs | Prior-node provenance only; no Bateman candidate result |
| P-GITHUB-GIT-DATA | Connected GitHub Git-data publication | Uploaded 15 byte-identical implementation blobs, produced tree `c23abbee...`, and created commit `cac7d1b7...` directly on parent `f954e391...`; final docs/PR/check state is recorded in the post-commit receipt | Publication provenance only; no candidate execution or scientific result |
| P-SCISPACE | SciSpace semantic paper search | Located Bateman analytic/stiffness and validated-stiff-IVP literature, including DOI `10.1063/1.2715785` and DOI `10.1119/1.5064446` | Supporting literature; not exact repository coupling, bits, budgets, or proof |
| P-WOLFRAM-CONTEXT-1 | Wolfram Context | First request returned MCP internal error `-32603` without scientific output | No evidence derived |
| P-WOLFRAM-CONTEXT-2 | Wolfram Context retry | Located NDSolve/stiffness documentation relevant to method selection | Background only |
| P-WOLFRAM-EVAL | Isolated Wolfram Language evaluation | Cross-checked analytic Bateman values, exact mass conservation, and Taylor bracket construction | Cross-check only; exact-binary Python proof remains authoritative |
| P-PYTHON-FRACTION | Repository stdlib-only verifier | Validates the frozen manifest schema/admission predicates and recomputes rational reference enclosures, W digests, PC bits, and scenario identities; the exact manifest byte hash and canonical Rust equality bind the frozen numerical fields | Authority construction only; explicitly reports zero candidate executions |
| P-RUST-CONTRACTS | Feature-gated Rust contracts | Check canonical manifest equality, opaque authority construction, candidate-free live-context W/PC rebinding, tamper rejection, and uncertainty-gate semantics | Non-executing construction contracts; local scientific runner remains pending |

The exact authority byte bundle currently has these SHA-256 identities:

```text
authority_manifest.json
673045bf6b9e723fceb6a3b8df8e9e9e9075c942cf1c438f0ebd03574dbac360

verify_authority_manifest.py
542715ca749efbf2060d608f2089ee8457e32f9c61fd0d35f613d5ecec26487d

evidence/AUTHORITY_VERIFICATION_RECEIPT.json
057cceba92fed0d707db1d586b53adebee5aed00583b224811d091f1d453ab12
```

The verification receipt reports two operator cases, six planned scenarios,
zero candidate executions, and `NOT_RUN_DURING_AUTHORITY_CONSTRUCTION`. The
local example checks all three exact byte sequences before it can construct
the opaque authority token.

## Literature fingerprints

- Yuan and Kernan (2007), DOI `10.1063/1.2715785`.
- Levy (2018), DOI `10.1119/1.5064446`.
- Hykes and Ferrer (2013), SciSpace ID `26w8vbcla7`, supporting the
  observation that Bateman systems can be numerically stiff.
- Yu (2004), SciSpace ID `4yjs5q7hbx`, supporting validated global-error-bound
  methodology for stiff IVPs.

These citations motivate the physical model and validation discipline. The
checked-in manifest and exact verifier, not search metadata, determine this
node's exact numbers.

## Boundary deviation

A delegated repository inventory briefly crossed into a prohibited
mixed-corpus section and was stopped. No path-specific content or finding from
that section was requested, propagated, inspected by the root executor, or
used in this package. Every retained inventory conclusion was independently
supported by allowed non-holdout paths. No Oregonator holdout file, fixture,
or test case was opened or executed.
