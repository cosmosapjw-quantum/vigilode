# R6 / C9 self-adversarial and stage progression | 2026-10-31

**Review type:** same-context adversarial, not independent distinct-author blind referee or HOL theorem. Original source and existing branches read-only. All tests were executed in the scientific ZIP, not represented as remote GitHub CI success.

## Reproducible scientific evidence
- Git immutable parent PR162 HEAD: `02f16fac23090cd4b8bfc5981e5b196c9e47c791`.
- Source original BASS owner ZIP 50×193 SHA256: `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`; source PR145 original E24 NPZ SHA256: `b087a54bcd3bdbfaef5d91165634bf5c7590e6c43417112e4de0168327b3366b`.
- Original existing coupled-Volterra source re-evaluated as 50 source-frozen histories, 96 GL angular nodes. Generated I/Q source NPZ SHA256 `59190f5ce6ffc7b7927c41c6fba91889d4332ebfab420ad721b790b91e1504ab`.
- 193000 genuinely calculated local collision rows, 38600 electron-frame radiation coherency moments; finite-grid M direct quadrature defect 4.4408920985e-15, parent E_l collision parity 2.3592239273e-16, positive minimum owner seed I−|Q|=0.2181855348. New owner IVPs=0.

## Falsification-driven audit
1. The naive positive intensity claim for I=J0+5J2 P2 was refuted: 1476 previously saved states yielded negative intensity. An exponential maximum entropy closure was investigated but discarded for the accepted physical result. Only original source-frozen Volterra I,Q are used.
2. Rest and z-axis β=.05 preserve LRS screen U≈0 (max 2.615e-17, 3.922e-17), transverse x-boost gives 7.35161124e-4 and xz45 tilt gives 5.70169737e-4. Reflection parity for β_x: I/Q even and U odd under φ→−φ at selected original owner source states. No all-sky physical B claim.
3. All 193000 rows checked proper density normalization g'_e γ=g_normal at fixed normal density. Original source and cloned unit test with forged hash reject modified bytes; no tolerance adjustment.
4. General kernel can accept source-independent U,V and arbitrary Hermitian coherency; original owner only provides I,Q. Synthetic circular-polarization controls DO NOT establish original source-owned circular signal.
5. Fresh 32/32 Python tests PASS; original raw-source zip CRC PASS. Standalone source-full ZIP cold restore: 40 payload SHA+size matches, 32/32 tests PASS, 14/14 source replay CSV/NPZ/diagnostic/figure byte-exact PASS. Prior first cold restore failed only SVG random clip IDs; fixed by deterministic SVG hashsalt and Date=None, rerun passed. All scientific tests and thresholds unchanged.
6. Original whole-repository Git authority historical FAIL_41181443124 remains. This new branch's CI is NOT_RUN until checked; distinct-author C9 NOT_RUN; native subagent not exposed, HOL kernel NOT_RUN. No production promotion, main merge, force push, owner source switch.

## Self-contained binary preservation
`VIGILODE_BASS21_OWNER_FROZEN_FULL_STOKES_THOMSON_20261031_SOURCE_FULL.zip`; **67,043,971 bytes**, SHA256 **af80130c93f52e2552e2c47c8b6d9a973042ce4d0b4bdc22bf4e44ffaa69c38b**. ZIP 41 entries with original 50-case source 23,770,654-byte ZIP, original parent scientific NPZ, source-frozen Volterra replay NPZ, 193000-row CSV, 38600 moments NPZ, code, tests, manifest, figures, handoff, `VERIFY.sh`. Clean separate directory run `bash VERIFY.sh --recompute` exit 0. The new full binary ZIP is a ChatGPT file artifact, **not independently uploaded and read back from two remote providers**; a Git theory/CSV/code PR is not an adequate binary backup.

## Per stage: actual progress, preservation, official canonical DAG

| Scope | Objective output and local evidence | Git checkpoint | VIGILODE DAG |
|---|---|---|---|
| R2–R3/C2 | PR155–162 current source and original immutable BASS source verified | source lock + ZIP | 7/23 |
| R4/C3 | Naive closure exact numerical refutation, original Volterra selected | negative findings retained in review | 7/23 |
| R5 | Exact boost, coherency tensor gain/loss, 3×3 angular map and eigen sectors | R5 theory commit `617fda6373adf9a5ad3f220771e640d53678201f` | 7/23 |
| C5 | 193000 original source frozen rows and 38600 moments | diagnostics code `02b777667d2380db0f87494069ceaf46fcaa26a2`, witness CSV `88ea9e42595d04b6a13d69f4b7356664a5d765ad`, kernel `51232f1ba48bd8a29e088f961a99fffd41d5dffd`, full ZIP | 7/23 |
| C6–C8/R7 | 32 tests, 14 byte-exact outputs, source parent E and M parity, screen symmetry | full ZIP clean restore | 7/23 |
| R6/C9 | This bounded hostile self-audit, no external referee | Git self-audit note | 7/23 |
| R10/C10 | Executable handoff, protected source pointers, production HOLD | Git handoff + full ZIP | 7/23 |

**Decision:** DERIVED/MODEL_SOURCE_AUTHENTIC/NUMERICALLY_CHECKED and local implementation VERIFIED_SCOPED. Nothing has been admitted to the formal VigilODE research DAG. VIGILODE 7/23, REI central 28/88 working 33/88, BASS CE1 9/15, new original IVPs 0. PRODUCTION=HOLD; NO_MERGE, AUTO_NEXT_NODE=false.