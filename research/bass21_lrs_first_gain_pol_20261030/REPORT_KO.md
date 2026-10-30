# BASS21 saved-history LRS polarized Thomson first-gain: 실행 결과

**2026-10-30 KST, 모델 조건부 1차 편광 물리데이터 / source-authenticated; production HOLD.**  이 보고서는 원본 LRS photon `J₂`를 사용해 **새로운 164,050개 Stokes Q 방향×시점 셀**, matched-shear 82,025개 ΔQ, 25×17개 정확한 3입력군 attribution을 실제로 계산한 결과다. 새로운 chemistry/geometry/polarization IVP는 0번. 수학과 입력 계약은 `THEORY_KO.md`를 따른다.

## 1. 재현된 원본과 source identity

- Frozen owner `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`.
- Original source-full VigilODE PR127 ZIP SHA256: `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`, bytes 23,770,654.
- AtomicNetwork source SHA256: `baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80`.
- 원본 source manifest의 50/50 NPZ bytes·SHA256 인증. Original 62-state 동일 time grid 193, source H, b, 9 photon energy Legendre multipoles, ne, Γ/H, τ. Original J₂=raw_state62[:,6]; b=raw_state62[:,1], τ=raw_state62[:,61]와 exact same saved data. Original original source immutability maintained.
- 별도 CE1-Q/Y `six-cell` input 또는 41 proper-time DOP853 geometry를 **섞지 않음**. REI17/P19/Grackle rate 동등성 가정도 없음.

## 2. 물리 결과

| 분석량 | 실제 결과 |
|---|---:|
| Source-authenticated original cases | 50 |
| Saved epochs/case | 193 |
| Distinct half-sky directions | 17 (`|μ|∈[0,1]`) |
| New normalized Stokes Q cells | 164,050 |
| Matched-pair ΔQ cells | 82,025 |
| Endpoint matched Q rows | 425 |
| New IVPs | 0 |
| Max sampled `|Q/I|` | 0.011741191785797517 |
| Mean z=2 equatorial `Q(s04)-Q(s00)` | +0.010318432952964909 |
| Min/max 25 equatorial contrasts | +0.010226112488455352 / +0.010410842444475477 |
| Mean z=2 equatorial Q, s00 (25 cases) | +5.239788081860125e-7 |
| Mean z=2 equatorial Q, s04 (25 cases) | +0.010318956931773093 |

이 수치는 `I_ref (a_ref/a_o)^4`로 정규화된 **energy-integrated finite-z=2 model Stokes Q**다. No temperature conversion, observational frequency band, last-scattering physics, sky E/B map, observational likelihood, or detector prediction. 원본 9 Legendre moments `F`는 모델 내부 관측 방향의 비편광 기준 I로만 이용했다.

## 3. 수송 기하/광자/opacity attribution, 25 matched pairs

`geometry_b`, `photon_J2`, `(g,τ)` 3입력군의 8 혼합평가 Shapley order-mean (μ=0, z=2) 실측:

| 그룹 | 평균 ΔQ | 평균 총 ΔQ에서 차지하는 비율 |
|---|---:|---:|
| Photon incident quadrupole J₂ | +0.009850298098749538 | +95.46312064681434% |
| LRS ray geometry b | +0.000476418937736908 | +4.61716367115622% |
| Forward opacity rate/history g,τ | −0.000008284083521539 | −0.08028431797057% |
| Sum = original pair ΔQ | +0.010318432952964909 | 100% |

`max |sum(groups) - ΔQ| = 3.469446951953614e-18`. 이 기여율은 결코 세 구성요소의 causal change/physical independent intervention이 아니며, 입력군 치환값은 일반적으로 Einstein 방정식의 해가 아니다.

## 4. 검산과 정확한 상태

- Python `unittest`: **13/13 PASS** (all 50 original source SHA; cold Thomson explicit GL28×56 9 input/ray combinations; original clock K_T n_e/H; exact null-momentum map; vacuum/quadrupole=0/FLRW analytic cases; screen symmetry; positivity; stored vs replay; 25 pair Shapley).
- Wolfram `FullSimplify`: LRS null-geometry screen factor identity 잔차 **0**; angular characteristic / energy drift 미분 identity exact **0**.
- 모든 Q(x=0)=0, 모든 Q(μ=1)=0; intensity baseline >0, all `|Q|<I` on 17×193×50 tested points.
- 선택 사례 Simpson vs trapezoid max `7.245379559968758e-7`; 전체 50개 final sky 193→97 source-grid Simpson 최대 차이 `1.151072350283333e-9`, 193→49 최대 `5.845049061425911e-8`. 이 수치는 **sampled quadrature stability**만 인증하며 기존 193 saved BDF chemistry/geometry global solution error와 polarized feedback truncation error를 제한하지 않는다.
- Local same-context scientific test, **not** independent *different-author* R6/C9; native subagent run NOT_RUN.

## 5. 원본 문제와 다른 물리적 주장 게이트

이 구현은 원본 BASS unpolarized photon hierarchy의 frozen input을 후처리할 뿐 기존 solver, collision coefficient, tolerance, source F/J, N01/RODAS, PR118 ledger, PR127, 다른 BASS/REI branches를 변경하지 않는다. Polarized backreaction, E₂ feedback, global spin atlas, finite-temperature redistribution, Thomson validity with actual photon spectral hard support, `z=0` observer, observational CMB likelihood all **UNRESOLVED**. Telescope-level CMB model/production promotional claim **blocked**.

Original VigilODE DAG **7/23**, REI central **28/88**, REI working **33/88**, BASS CE1 upper **9/15** (different source), successor admitted **0/12**, official DAG delta **0**. Auxiliary science output **1/1 scoped source data created**; this does not claim a central DAG node complete. Original BASS21 R/C 17/20, C8 PARTIAL and independent R6/C9 NOT_RUN unchanged. In this new auxiliary unit R/C 18/20 scoped: sampled quadrature C8 completed for narrowly defined task, independent R6/C9 NOT_RUN. Production HOLD.

## 6. Next science step, no automatic execution

A genuinely **coupled polarized collision** calculation must evolve/solve the source-consistent polarization tensor and its feedback into photon `J₂` and Einstein shear with an authenticated collision operator, proper event-time screen map, spectral validity, and source-history errors. A full observer sky requires a longer sourced history including late-time observer, not an extrapolation of τ. This is a subsequent separately admitted task. All retained original artifacts, negative CI and authority locks remain intact.