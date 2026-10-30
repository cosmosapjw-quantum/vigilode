# VigilODE × BASS21 | Coupled Thomson (δI,Q) source-frozen 실제 데이터 결과

- 2026-10-30 KST, `research/bass21-lrs-coupled-thomson-response-20261030`라는 독립 브랜치. 계산 전 Git prospective prereg commit `138072bcea820c8a8c583b098c8244fc69ac02e6`.
- Original full source ZIP (nested) 23,770,654 B, SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`; wrapper original PR130 ZIP 28,120,080 B, SHA256 `d6aaaf75aa82f083eddd577f9d0f44d54fe5fdfdd2978fe4882a6d986edc3e2d` independently read back.
- Original 50 same-event BASS21 62-state cases, each 193 saved epochs with source b,J₂,g,τ, H,ne. No replacement by CE1 six-cell or REI17/P19/Grackle event.
- New physical `Q_coupled` 50×193×17 = **164,050**, new `deltaI` same = **164,050**, 25 paired `deltaQ` 25×193×17 = **82,025**, matched endpoint pair rows **425**.
- Mean equator matched ΔQ old first-gain = **0.010318432952964909**; coupled = **0.010520628797866527**; incremental = **+0.00020219584490161854** (**~1.96% relative to first gain**).
- Matched equatorial ΔQ full range **[0.010426230728885707, 0.010615118061701517]**; maximum sampled |Q_full/I_full| **0.011991577849511477**; maximum |δI/I_base| **0.0006384116803021321**; min I_full **0.22494695316114469**.
- Coupled fixed-operator max absolute moment residual **4.632579042596063×10^-15**; max first-gain exact replay error **1.9081958235744878×10^-17**; GL24/48 max Q diff **3.469446951953614×10^-18**, δI diff **5.421010862427522×10^-20**. Picard <=7 iterations. All axis Q poles exactly 0 and x0 Q initial exactly 0; no I_full<=0 or |Q|>I.
- 193→97/49 stored-grid sampled quadrature maximum Q difference **3.2522494184886045×10^-6** (selected original four cases). **Not** BDF global temporal-convergence proof or full polarized-solver error bound.
- Local unit tests: **10/10 PASS** (includes independent Cartesian 48×96 Thomson tensor quadrature, zero source, manufactured 2×2 exact matrix exponential, first-gain limit, source guard, sample-grid guard). Baseline PR130 clean `VERIFY.sh` exit0, 13/13 PASS. Wolfram verified `∫(1−μ²)² dμ=16/15` and polarization gain eigen 3/5; confirmed local constant-rate matrix.
- Full source-inclusive ZIP to be verified with package `VERIFY.sh`, source original kept binary exact; Git mirrored text and endpoint CSV, not binary matrices.

## Scope and pending

Physics result **model-conditional**: self-consistent linear `(δI,Q)` Thomson gain on unpolarized frozen BASS21 baseline, not fully backreacting polarized Einstein–Boltzmann chemistry or observer E/B; no new IVP. BASS PR274 KM2D time-dependent electron boost has distinct noncommuting geometry/source and cannot be spliced into these saved 193 epochs. Original PR130 research-ledger failure and PR127 failure preserved (retrospective absence of preregistration and ledger row); this *new* node was independently preregistered before its numerical execution. Native distinct-author R6/C9 NOT_RUN, original BDF global temporal uncertainty HOLD.

DAG snapshot: VigilODE 7/23, REI central 28/88, working 33/88, BASS CE1 9/15, successor admitted 0/12, canonical delta 0. New independent auxiliary model-conditional output 1/1 **scoped science, publication/production HOLD**; NO_MORE_IVPS, NO_MAIN_MERGE, NO_FORCE_PUSH, NO_PRODUCTION_PROMOTION, NO_TOLERANCE_WEAKENING, AUTO_NEXT_NODE=false. Owner admission decision remains separate.

## References

- A. M. Anile, Astrophys. Space Sci. 29, 415 (1974), DOI `10.1007/BF02639073`.
- A. Challinor, Phys. Rev. D 62, 043004 (2000), DOI `10.1103/PhysRevD.62.043004`.
- M. Kamionkowski, Phys. Rev. D 104, 063512 (2021), DOI `10.1103/PhysRevD.104.063512`.
