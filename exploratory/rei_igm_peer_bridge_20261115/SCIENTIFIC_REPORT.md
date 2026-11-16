# VigilODE 교차 브랜치 Peer-W × 원본 REI F05: 실험 마감

검증·마감일: 2026-11-16 KST. 디렉터리의 20261115는 작업 시작 시점 식별자다. 본 결과는 기존 원본 F05 물리의 **선택한 단일 closure에 대한 1-tick point-IVP** 실험이다. 불확실성 전체의 interval IVP, 원본 최종 14개 native interval ledger, AXBI 전체 이력의 정확도 승인 또는 production solver 합격과 구별한다.

## 1. 실제 재사용과 소스 고정

기준 PR183의 450b7d10240e5b195d9dd2b99c5a8f4724078162 위에서 다른 VigilODE 개발 경로의 결과를 실제 도입했다. production-unification 커밋 **0caa3ed21fcbd5c3fdb19f873734d86ffeba5853**의 PPSW1-6b tableau·stage-input 구현을 byte-exact 복사했다. 복사본 SHA256은 **758f712ce1786a192f05dbcee2a94c6bd91ae6a2bc3ea760293966cbb3e424bc**다. 기존 core의 LowRankBorderedPreconditioner와 bounded ParallelExecution, 원본 REI native H/He/OTS/thermal 연산자를 결합했다. 다른 worktree의 미커밋 matrix-free 구현은 도입하거나 수정하지 않았다.

원본 source-owner는 f1b20b132e1ecc631c379e2d7ca6960cfdb8daaa, 고정 26-event kernel은 579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644다. 46,080개의 material/source cell을 보존했다. 이를 새 공간격자로 해석하지 않는다. 변수는 셀마다 (x_HI, x_HeI, r_HeIII, ln T)의 네 개, 총 **184,320차원**이다. 광자 4그룹의 시간 의존 원본 PPoly forcing을 사용했으며, 15변수 또는 photon-only surrogate가 아니다.

초기상태 원본 SHA256: 20b6066eade8a31ecfe46a47081a2e3a7fef7353dba4ac83204456c24ac5f628.
Forcing SHA256: 0608145d1ab7c61f9b596ad1e407bb93b496df6f8fc2e79d5a33dba1df1fb581.
독립 Python source-owner DOP853 궤적 SHA256: 82145e2db772eef833ec5a9823d1e995c1fcd7d4df4d280503b6df0ccbe353b3.

허용된 closure 중 f=0.55, source table-cell v midpoint를 선택했다. 원본 proper-time tick160→161을 무차원 tau∈[0,1]로 표현하되 RHS에는 원래 초 단위 구간 길이를 곱했다. 입자수는 셀별 extensive count, U는 erg, T는 K, k_B=1.380649e−16 erg/K다. 검사된 각 stage/state는 positive cone과 고정 v table-cell 조건을 만족해야 한다. HeII의 극미량 양의 rate가 binary64에서 underflow하는 과거 문제를 이 실험이 해소한 것으로 간주하지 않는다.

## 2. 구성적 결과와 수치적 수정

고정 source table-cell에서 전역 광자 의존성은 세 neutral moment를 통해 들어간다. 따라서 전체 point-RHS Jacobian은

J = blockdiag(J_local,i) + U V^T,  rank(U V^T) ≤ 3

로 분해된다. 이는 **전역 결합 보정항의 rank**이며 전체 Jacobian의 rank가 3이라는 주장이 아니다. local 4×4 derivative는 moments를 고정한 중앙차분, nonlocal derivative는 analytic chain rule이다. 전체 독립 방향차분 대비 JVP discrepancy는 4.503910843036103e−10이었다. 따라서 이 구현은 근사 frozen W-matrix를 사용한다. 원본 sparse+rank3 연산자를 기준으로 stage linear residual을 검사했으며 모든 보고값은 사전 고정한 1e−10 한계 이하였다.

첫 absolute-state 실행은 n=4,8,16으로 세분할수록 lnT discrepancy가 커졌다. 이 부정적 결과를 보존했다. 상수 history에서 직접 stage 합을 구한 뒤 기준 상태를 빼는 연산이 가짜 increment를 만들었다. 동일 시험의 red assertion을 관측한 다음, B·1=1과 Gamma·1=−gamma·1을 먼저 소거하는 centered-increment 표현을 구현했다.

기준 상태 a, Delta_j=Y_previous,j−a에 대해 같은 실수 산술의 Peer 식은

(I−h gamma J)(Y_i−a) = Σ B_ij Delta_j + h Σ A_ij F_previous,j + h J Σ Gamma_ij Delta_j

가 된다. coefficient bytes는 바꾸지 않았지만 binary64 평가 순서는 바뀐다. 따라서 원래 absolute-form과 byte-identical이라고 하지 않는다. 반면 같은 centered 방법의 직렬/6-stage 병렬 실행은 실제 전 궤적 bytes가 동일했다. centered stage는 incremental W residual과 복원된 absolute W residual을 모두 검사한다.

일반 Peer 방법의 문헌은 Schmitt & Weiner, SIAM J. Numer. Anal. 42 (2004), 265–282, DOI 10.1137/S0036142902411057이다. 그 문헌의 차수·안정성 정리를 변경한 starter와 근사 Jacobian을 갖는 본 실험 전체에 자동 전이하지 않는다. dense Radau starter 대신 비용을 포함해 기록한 RK4 history initializer를 사용했다.

## 3. 실제 물리 궤적과 독립 비교

독립 기준은 **원본 Python floating_reference_rhs**를 사용하는 DOP853이다. rtol=2e−11, atol=1e−13, max_step=원본 tick의 1/8로 실행했다. native RK4 64/128-step 및 17개 공통 epoch와 비교했다. native RK4-128 대 원본 DOP853 최대 discrepancy는 x_HII 1.1102e−16, x_HeII 2.2204e−16, x_HeIII 3.6429e−17, lnT **9.76996e−14**였다. 이는 기준해 상호검사이며 rigorous global error bound가 아니다.

| Peer 구간 분할 | absolute-form max lnT discrepancy | centered-form max lnT discrepancy | 감소 비율 |
|---|---:|---:|---:|
| 4 | 6.417977260753105e−11 | 5.133671265866724e−13 | 125.0배 |
| 8 | 1.275868299899230e−10 | 6.696865284538944e−13 | 190.5배 |
| 16 | 1.989590714401856e−10 | 9.148237722911290e−13 | 217.5배 |

4/8/16은 원본 1-tick 구간의 분할 수다. 첫 두 분할은 기록된 RK4 history 생성에 사용되므로 각각 Peer W-factorization은 2/6/14회, stage solve는 12/36/84회다. 시작 비용을 누락하지 않았다. centered discrepancy도 세분할에 따라 단조 감소하지 않는다. 이 한 구간은 rounding-floor 지배 범위이므로 5차 수렴 증거로 삼지 않는다.

대표 centered n=8의 선택한 point closure에서 ΔNe≈−1.55980452815385e60 electrons, ΔU≈−3.048497329190732e49 erg를 얻었다. 이 부호를 불확실성 가족 전체에 대한 interval sign으로 확대하지 않는다. H/He 원자핵 보존은 reduced-coordinate 구성에서 따르며, 독립적인 최종 14-ledger 검증의 대체물이 아니다.

## 4. 비용 비교: 계산한 것과 계산하지 않은 것

동일 centered n=8 전체 point-IVP, 동일 초기 source, 동일 결과에서 순서를 [직렬,6-stage,6-stage,직렬,직렬,6-stage]로 두고 6회 측정했다. 시작 history와 W setup 및 RHS work를 포함했다.

- 직렬: 3.427075782, 3.368278323, 3.288060580 s. 중앙값 **3.368278323 s**.
- 6-stage 병렬: 2.762564404, 2.730647652, 2.669587423 s. 중앙값 **2.730647652 s**.
- 병렬/직렬 시간비 **0.8106953731685432**, wall **18.9304626831% 감소**.
- 여섯 궤적 SHA256 모두 **7154ea71a1e298ddc5392ab3c5e5ad0e94e6e88396199ee271cf29026b008c04**.
- 해당 paired 실행의 관측 최대 RSS는 206,356 KiB. 단일 shared/busy host의 관측이며 일반적인 확장성·성능 보장을 하지 않는다.

이는 원래 frozen photon kernel이나 thermal-root만의 timing보다 넓은 **해당 선택 closure의 한 구간 point-IVP** 비용 비교다. 원본 전체 불확실성 IVP, AXBI zbar20→4 다섯 이력, production controller의 matched-accuracy 비용 개선은 아직 입증되지 않았다.

## 5. 중단 복구 후 새 검증과 적대적 감사

2026-11-16 재개 시 live PR183과 local HEAD는 6844c8fe18361f3c2243ee2db95bbf3d68a6ce7b로 일치했고 centered 수정은 미커밋 상태였다. 이를 보존해 다음을 실제 재실행했다.

- native Rust 선택 시험 **17/17 PASS**, offline/locked release 빌드 성공. 기존 unused-attribute/deprecation warning은 남아 있다.
- 현재 코드로 전체 centered 4/8/16 및 6-stage 실험과 RK4 기준을 다시 실행. **6개 raw trajectory가 기존 결과와 byte-identical**.
- 새 독립 audit 정상 모드와 Python −O 출력이 byte-identical.
- receipt census와 실제 solve 횟수 위조는 기존 감사가 거부하지 않았다. NaN W residual은 물리 guard를 통과하고 JSON serialization에서 뒤늦게 거부되었다. 이 구별을 보존한 red test 이후 explicit guard를 수정해 **4/4 normal, 4/4 optimized PASS**.
- native source/forcing bitflip, 손상된 궤적의 normal/optimized audit, missing case, NaN residual, false zero solves, forged official acceptance: **8/8 예상 거부**.
- source-owner 및 다른 development worktree를 변경하지 않았다. 별도 subagent/외부 심사자는 실행하지 않았으며 자기 적대적 검증이다.

전체 저장소 CI와 위 선택 시험을 구분한다. 부모 6844c8fe의 8개 workflow 중 6개가 실패했고 Production CI의 fmt/test/bench가 실패했다. 본 범위에서 그 전부를 수정하거나 원인을 확정하지 않았다. 최종 게시 HEAD의 CI 상태는 별도 live readback으로 남긴다. source별 선택 시험 PASS는 전체 production PASS가 아니다.

## 6. Drift watchdog와 후속 권한

이 로컬 실험의 네 노드 X1(source/유도), X2(교차 브랜치 native 구현), X3(실제 물리 실행), X4(감사/게시/백업)를 마감한다. 작업 완료 수와 canonical science DAG를 섞지 않는다. 기존 F05 track 카운터 VigilODE7/23, successor0/12, REI central28/88, working33/88(nonadditive), fast4/7은 이 작업으로 증가하지 않는다. Production HOLD, F05 official tick161 NOT_ACCEPTED를 유지한다.

**최신 원 목표의 소유자 계약을 우선한다.** REI PR375 @56dc609ea9b801c88d3364aed3eee140b3a20d46은 원 AXBI 다섯 이력을 완성하고 AX08을 승인해 원 목표 DAG가 **10/17**이다. 별도 expansion28/88은 그대로다. 시나리오는 PLANCKLIKE_MD14_BB50K_CONDITIONAL_HHE_V1, homogeneous non-tilted axisymmetric Bianchi I, primary-only Case-A escape, zbar20→4다. F05 OTS의 state/clock/closure를 여기에 이식하지 않는다.

다음 VigilODE 실제 과제는 **AX11 원본 source의 dominant error component와 native/adapter 시간 계측**이다. 관측된 19 rejections 및 RHS/trial≈7.00687만으로 Newton 실패 또는 stiffness가 병목이라고 단정하지 않는다. 원 배경 연산의 정확한 중복 제거와 discarded spectrum lazy construction을 검증한 뒤, full-IC 시간 refinement 1e−5 / 2.5e−6 / 6.25e−7을 유지한다. 이 마감은 AX11 자체의 진전으로 계산하지 않는다. 더 이상의 고립된 F05 성능실험은 유예한다.
