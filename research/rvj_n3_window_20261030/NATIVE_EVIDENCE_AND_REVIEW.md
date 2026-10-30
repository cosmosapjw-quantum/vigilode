# N3 native Rust1.99.1, model, tests, negative evidence and actual work

Date 2026-10-30 KST. This report covers the **standalone N3 research crate**, not current VigilODE PR131 native Rust source.

## Toolchain and offline input

User-provided standalone \`rust-1.99.1-x86_64-unknown-linux-gnu.tar(1).xz\`: 210,515,736 bytes SHA-256 \`231fcae47ee950621374725b5fc3b5441d2fbda2955ac45dd7dc816d8876cfe7\`; rustc 1.99.1 commit \`58cf8f9c35b355199727627d07ae34a4e2846d67\`, cargo 1.99.1, rustfmt 1.10.0, Clippy 0.1.99. Installed in isolated prefix; no system modification. Detached PGP .asc file provided, but \`gpg --verify\` yielded **No public key**, so authenticity status is **NOT_VERIFIED** (hash is independently recorded, not officially authenticated).

Provided offline VigilODE ZIP: 70,055,503 bytes, SHA-256 \`f8caea8ac3435dfde6636947bd2618f659ef9930e3a6910148972a96fc2e6045\`, ZIP CRC PASS, 11,585 member entries, 118 vendor crate dirs, five workspace packages. Cargo.lock SHA-256 \`04c7d6c147bf19ebb1a627463705d9dcf27337a3ff228dd1169f19eb73c71400\`. This is an **older source snapshot**, not the immutable exact native PR131 HEAD. Rust1.99.1 \`cargo check --locked --offline -p rodas5p-core\` PASS, \`cargo test --locked --offline -p rodas5p-core --lib\` **27/27 PASS**; initial long dependency compile timed out and was subsequently resumed by incremental build; do not call initial timeout a physics failure.

## New standalone N3 source-owned model

Rust standalone Cargo 2024, no external crate dependency. Owns an immutable source \`Arc<Source>\`, represented complex \`a_j\`, two damping clusters (\`kappa_s=0.25, kappa_f=1000\`), 2×2 collective rank basis, two accepting windows \`h=0.25\`, original independent initial complex data, source token, trial-id, accepted epoch and clock bits. One-Picard exact damping is evaluated with polynomial cubic triple-cluster convolution and integer-degree exponential moments.

Adaptive experimental width selector halves \`h\` after **research** refusal, without losing prior accepted state or attempted work. \`n<=8\` chooses a protected baseline without speculative work. Detector \`kappa*h>=10\` distinguishes window fast modes; not a default production dispatcher. This path does NOT call native original RODAS8 sequential stages, cannot represent source-attested PR131 C9 eight-stage outward certificate, and does not replace RHS-only homotopy with an unrelated JVP-first scheme.

## Direct execution

* New Rust1.99.1: \`cargo test --offline\` **20/20 PASS**, \`cargo fmt --check\` PASS, \`cargo clippy --offline --all-targets -- -D warnings\` PASS, after recorded initial parser-RED and strict Clippy RED repairs.
* Research source and transaction negatives: wrong source, stale epoch, abort pending, bad tolerance, negative/malformed h, exhaustion fail-closed, attempted=accepted+rejected+pending; energy cancellation; exact zero-coupling decay; sign involution; odd 65-mode state; fast/slow detector.
* Repro script \`REPLAY.sh\` and restored independent N2 Python source with an external Rust1.99.1 prefix: all Rust and Python checks completed; deterministic native_windows.csv **byte-identical** with prior result. Timings are not expected to be byte-identical.
* Native model n=12,24,65,257, two width-1/4 windows, analytical q=21/400. Second-window displayed float bound maxima n12 \`4.21618e-10\`, n24 \`4.32715e-10\`, n65 \`4.41614e-10\`, n257 \`4.42930e-10\`; fast n24 max \`6.16e-13\` versus slow \`4.33e-10\`.
* Independent mpmath first-Picard recomputation max discrepancy <=1.74e-17 at n=12/24/65/257. SciPy BDF only n12/n24 sampled comparisons max 3.85e-13 and 2.26e-13, respectively. Neither reference is an outward interval.
* Candidate-only 30-sample median wall ns, including proposal/transaction n12 \`98257\`, n24 \`194512\`, n65 \`559539\`, n257 \`2497838\`; n6/n8 protected skip. No original native RODAS5P matched-accuracy comparator or universal speedup.

## Required adversarial carry-forward

Exact Fraction oracle reconstructed the exact-real bound. **544/716** native float component-window bound values are fractionally below the rational source-model bound by a few ULPs; max recorded absolute difference ~1.04e-25. This confirms **no outward numerical certification**. The Rust public decision string says \`RESEARCH_ACCEPTED_IDEAL_BOUND_ONLY_NOT_PRODUCTION\`, never a production PASS. A real source-attested production gate must enclose source rational-to-float conversion, exponential kernel moments, all candidate arithmetic and inherited propagation, then audit the **actual original raw-U eight-stage residuals**. Do not merely add a heuristic epsilon and promote.

**Stage checkpoint:** R6-R9 and C5-C9 same-author scoped test/review only. Real different-author C9 **0/1**, official DAG 7/23 unchanged, RVJ scoped 3/8, C10 NOT_STARTED, production HOLD. Executable source is archived as full ZIP and must be independently retrieved for source-level evaluation.