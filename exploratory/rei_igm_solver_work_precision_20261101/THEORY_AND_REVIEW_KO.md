# VigilODE × original REI 26-event source: 정밀도 보호형 rank-three nonlocal Jacobian

## 원래 26-event 소스에서 직접 얻은 구조

REI PR348의 immutable owner (`fcc90939b6b7cc842fae65d5a0ffbb69a3ec3120`), 원본 source SHA `a861278201313c55e08ba6323b5c1d2ad97bf5765f429807b4eba0a1c2465d0b`, 원본 native 26-event kernel SHA `579df3cc99987dcce9205166752570bdaf4c7cb3640c525be6da260d94b8e644`을 읽기 전용으로 사용했다. Accepted tick160 46,080개 실제 H/He 5-population 상태, 네 photon group, 세 공통 global neutral moment, 원래 `forcing_bounds(t160,t160)`을 사용했다.

원래 `ReducedIntervalModel._explicit_photo_fields`의 한 광자군 항은

`F_{isg} = J_g α_{sg} N_{is} / D_g`, `D_g = E_g + Σ_{a=1}^3 α_{ag} S_a`, `S_a = Σ_{i=1}^{46080} N_{ia}`.

원래 `alpha`는 H I에 `scale_z*sigma[H,g]/N_H`, He I에 `Y_He*scale_z*sigma[HeI,g]/N_He`, He II에 `Y_He*scale_z*sigma[HeII,g]/N_H`이며 실제 원본 support mask를 유지한다. 같은 원래 시각의 source parameter를 고정한 방향 미분은

`(dF v)_{isg} = (J_g α_{sg}/D_g) v_{is} - (F_{isg}/D_g) Σ_a α_{ag} Σ_j v_{ja}`.

따라서 photon 부분 Jacobian의 전역 coupling은 `U V^T`, `V^T v = (Σ_j v_{j,HI}, Σ_j v_{j,HeI}, Σ_j v_{j,HeII})`, rank <=3이다. 네 군을 더해도 공통 우측 V가 있으므로 전역 rank 상한은 3이다. 셀 국소 대각 항은 남는다. 이 rank가 original 26-event 전체의 비국소 Jacobian rank를 결정한다는 주장은 하지 않는다. 원래 열근, Hummer–Seaton 분기, OTS, 26-event ledger를 최적화가 교체해서도 안 된다. Wolfram 독립 기호항등식은 미분 잔차 0 및 photon fraction sum+external=1이다.

## 적대적 부정 대조군 및 정밀도

처음엔 모든 계수를 float64 `alpha=c/N`, `denom=E+alpha dot(sumN)`으로 단순 재배열했다. 수학적 JVP 중앙차분은 <1e-12 수준이지만, 이 float 재배열은 원본의 좁은 원본 interval enclosure에서 H I **0/46080** (단 하나도 포함되지 않음) 및 heat **3659/46080**을 기록했다. 이 실패는 `ORIGINAL26_INITIAL_INTERVAL_FAIL.json`으로 보존. 수용조건을 바꾸지 않았다.

대안은 원래 물리적 source 법칙과 모든 support / current / weights를 보존하면서 **오직 global three sums와 group scalar coefficients**를 `numpy.longdouble`로 계산하고, 46,080개 cellwise flux/JVP 산술은 float64로 수행한다. 본 호스트에서 H I/He I/He II **각 46080/46080**, 원래 `photoheat` **46080/46080**의 원본 interval enclosure를 만족한다. Photon budget과 JVP 유한차분도 별도로 계산한다. 본 실험은 x86 확장정밀도에 의존하며 MPFI 유향 반올림의 증명이 아니다. Python array temporary allocations가 있으므로 VigilODE zero-allocation production kernel이 아니다. `source_owner_comparison_pass`는 이 한 accepted tick160 대표점에서만 의미 있다.

## 원래 REI 임무에 대한 판정

VigilODE RODAS5P 네 경로는 PR175 15-state source reduction의 frozen/timevarying 두 모델에서 Radau 기준을 모두 만족했다. 그러나 timevarying native sequential은 JVP 10051, fast-variant는 10516으로 **+465 / +4.63% 작업 증가**했다. 단일 walltime은 빠른 경로에 유리했지만 timing authority는 diagnostic-only이며, 원본 26-event 비선형 통합 속도 향상으로 전이할 수 없다. Source-only rank-three 구조는 original 26-event solver용 **전처리/선형연산자 후보**이지 production acceleration result가 아니다.

Original-owner PR348의 common-parent four-channel defect 연구는 그대로 보존한다. 기존 worker의 original 2.1245050576368385e-4 > 2e-4 분류는 바꾸지 않았다. REI 28/88 central, 33/88 working, fast 4/7, VigilODE 7/23, successor 0/12, all canonical delta=0. Original tick161 not admitted; no merge / no force push / no tolerance relaxation. 다음 단일 구현 목적은 46k state의 full 26-event source owner에서 이 JVP를 **잔차 검사형 opt-in preconditioner**로 실제 통합하는 것. 그 전에는 rank-three algebra 단독 연구의 추가 확장을 중단한다.

## 리뷰 등급

`derived` (rank <=3), `numerically_checked` (46k, original source-conditional midpoint), `implementation_verified_scoped` (Rust 4-method reference, only reduced owner), `unresolved` (true full 26-event JVP, error-gate admission, all-X finite-source uniform remainder, per-step zero heap allocations), `production=HOLD`. Same-author adverse check only, not distinct-author referee.

## 추가 경고: 극미소 He II source
원래 accepted tick160의 He II photo rate는 longdouble에서 46,080셀 전부 양수이지만 float64로 각 셀을 바꾸면 46,080셀 전부 zero underflow한다. Longdouble의 전 셀 rate 합 4.9266637335576802504e-324 particles/s는 수학적으로 0이 아니며 owner interval에 0이 포함된다고 physical exact zero로 해석할 수 없다. Result JSON에 underflow counters를 포함한다.
