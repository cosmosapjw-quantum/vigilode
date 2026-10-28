# Next8: 실제 cc602 source에서 다음 enclosure 단위로 연결

## 조사 범위와 상태

고정 BASS commit **cc602e209e7deeb511ab2a1f3b2268d55206c43c**, binding **8cb268e8de741e284cf494013f1e0748d27f03c5e77ba0c64413ae92af082d16**의 관련 원문 12개를 새로 회수했다. 모두 bytes와 Git blob SHA가 일치한다. 10개는 전체를 읽었고 donor/atomic manifests 2개는 구조·식별자·관련 file entries를 읽었다. 기존 client_binding.py, CLIENT_IDENTITY.json, source_binding.json, affine_system.py 4개도 원본 그대로 보존했다. 회수한 affine_system.py는 기존 보존본과 정확히 같은 bytes다.

새 source를 import하거나 실행하지 않았다. 과학 계산·test·CAS·native norm/lift·RHS/Jacobian/ODE/NCP/build 호출은 0이다. 모든 원격 동작은 GET이다. 이전 next7 및 이전 실행 단위는 CLOSED를 보존한다.

부모의 새 중앙 관측은 HEAD **df806909aee8f25efcfa4faeefca77a5271f546c**, 기존과 같은 DAG blob, AGENTS blob **429a5a13ee75b1222bc50627f8d5973d01dd21c4**, 게시 목표 **78f7ac5864d5817ceb79b48cb5e298cc30006510**, **28/88 / 이번 조사 증가0**이다. 백업·게시 및 사용자 요청 재시도는 부모가 관리한다.

이번 fixed-source 루트 AGENTS.md GET은 404였다. 최신 관측 VigilODE 분기의 AGENTS.md도 404였다. 이 사실로 중앙 정책을 무시하지 않는다.

## 실제 call graph와 불변 anchors

| 경로 | 의미 | Git blob |
|---|---|---|
| inherited client_binding.py | FrozenClient strict point preflight → affine rhs/jac; ProvidedOnly; partial_x 미공급 | SHA256 327514dc3a036690ad584f9461375ec1cb034f5cf5b282294e3afcb7e6df7a13 |
| research/sandbox_affine_chemistry_20261022/code/affine_system.py | compact33 → affine30; neutral totals와 charged selection; J30=J33[KEEP] LIFT | baa8272d507375cdbcb11ec89af1a6282dca6b0b |
| research/rei_hheli_compact_pilot_20261022/code/compact_system.py | full62 → compact33 constant restriction, isotope-fraction equality, n/T zero subspace | 4769767f1fb6a13856c86e0c36484a95282be570 |
| vendor/rec_hheli/code/coupled.py | geometry40 + atomic21 + optical depth, proper-time chemistry를 H로 나누고 전체 analytic point J 조립 | 7d1a346081528f7f9449fdcb1032cf84a9aabb44 |
| vendor/rec_hheli/code/atomic_network.py | source rates 및 abundance-dependent electron feedback | e3a0603176304f64b953c0b2faf898d6b75eceb5 |
| vendor/rec_hheli/code/geometry.py | dynamic electron Thomson term, analytic hgrad와 geometry J | a6ba35fff4f62ef795628499f108705ab6b55ac1 |
| vendor/rec_hheli/vendor/baseline/canonical_fv.py | positive finite momentum packets, H²와 matter stress | c5386da152fb242020610fc24e25b8e155ea5125 |
| vendor/rec_hheli/vendor/tangent/tangent_core.py | prepared finite-sum JVP/VJP, 실제 mass-shell derivative formulas | 01bee25389889ba376f2bfa060ff903eb3093237 |

위 vendor 상대경로의 기준은 research/rei_hheli_compact_pilot_20261022다. 전체 immutable URL과 byte/SHA256은 SOURCE_IDENTITIES.json에 있다.

DONOR_IDENTITY.json은 REC donor commit **a949529872eb5ba0e20c7e206fe893e5d544e48c**의 57파일을 지정한다. 실제 회수한 atomic/coupled/geometry/canonical/tangent/data source blobs는 그 donor entries와 일치한다. 이는 이번에 읽은 파일의 계보 확인이며 donor 57파일 전체를 다시 검증하거나 실행했다는 뜻은 아니다.

## 직접 확인한 수학적 구조

AtomicNetwork.evaluate의 real-arithmetic source 식은

\[
n_e=n_b q^\mathsf TY,\qquad
f_Y=(G_0+D)Y+n_b(q^\mathsf TY)G_{\rm ne}Y,
\]

\[
D_Yf_Y=G_0+D+n_eG_{\rm ne}
+n_b(G_{\rm ne}Y)q^\mathsf T.
\]

마지막 rank-one term은 evolving abundances가 electron density를 공급하기 때문에 필요하다. 고정 temperature/UV/density에서 RHS는 quadratic이고 J는 population-affine이다. 그 term을 누락한 fixed-electron generator는 실제 source Jacobian이 아니다.

CoupledSystem.jac는 chemistry rows의 geometry derivative도 유지한다. Source-clock scaled chemistry RHS를 \(F_{\rm chem}=(dY/dt)/(\mathrm{scale}\,H)\)라 쓰면

\[
J_{\rm chem,geo}=-F_{\rm chem}\,(\nabla_{\rm geo}\log H)^\mathsf T.
\]

Geometry.jacobian_blocks의 hgrad는 실제로 \(\delta H/H\)의 계수다. 이 off-block을 생략하고 chemistry 6×6을 full30 Jacobian으로 해석해서는 안 된다. Optical-depth row와 photon Thomson chemistry coupling도 실제 source에 존재한다.

Tangent source의 mass-shell variables는 \(U,W\ge0,\ E=\sqrt{30^2+U+W}\)이며, positive packet weights에서 \(\rho_b=-6\pi\) 및

\[
\pi_b\text{ kernel}
=-\frac{(2U+8W)30^2+U^2+14UW+4W^2}{6E^3}\le0
\]

가 명시되어 있다. 이는 추후 geometry moment enclosure의 좋은 실제 kernel anchor다. 현재 prepare는 한 점의 finite sum을 계산하며 whole-corridor interval을 생산하지 않는다.

## 실제 reduced chemical 좌표

Reduced30의 chemical indices는 **23,24,25,26,27,28**이다. 순서는 HII, HeII, HeIII, LiII, LiIII, LiIV다. 나머지는 geometry23좌표와 optical depth(index29)다.

Compact33 chemistry의 순서는 H0,H1 / He0,He1,He2 / Li0,Li1,Li2,Li3이다. Neutral fractions는 기존 source의 totals와 LIFT로 복원한다. **H total을 새로 1로 바꾸지 않는다.** 현재 binding의 H total은 hex 0x1.fffffffffffffp-1, He/Li는 각각 0x1.0000000000000p+0이다.

Source equality model은 H1/D, He3/He4, Li6/Li7 charge fractions를 같게 두며 n/T inventory가 0이다. Positive isotope weights를

\[
w_H=s_{H1}+s_D,\quad w_{\rm He}=s_{He3}+s_{He4},\quad
w_{\rm Li}=s_{Li6}+s_{Li7}
\]

라 하면 electron fraction은 \(e^\mathsf Tz\)이고

\[
e=(w_H,w_{\rm He},2w_{\rm He},w_{\rm Li},2w_{\rm Li},3w_{\rm Li}).
\]

이 weights는 atomic_model.json과 _prepare_inventory의 source-defined normalization/scales에서 온다. 이번 조사에서는 owner 초기화나 그 계산을 수행하지 않았다. 실제 represented scale 값의 보존 또는 별도 정확한 생성 계약 없이 임의의 값으로 대체하면 안 된다. Source real formula와 binary64 준비·callback roundoff도 구분한다.

## 옵션 A — 추천: source chemistry J의 정확한 전체 simplex extrema

**과학적 산출물:** 실제 source transition topology와 electron feedback를 사용하여 chemical 6×6 Jacobian의 36 entry에 대한 전체 chemical domain extrema 및 attaining witnesses를 만든다. 기존 matrix certificate의 identity wrapper를 하나 더 만드는 일이 아니라 source RHS에서 interval entries를 생성하는 단위다.

Source-clock의 6개 photo effective rate와 6개 recombination effective rate를 각각 \(\gamma_i=\Gamma_i/H\), \(\kappa_i=n_b\alpha_i/H\)로 둔다. Frozen positive isotope electron weights 및 기존 totals 아래, source affine lift를 재사용하면

\[
F_\chi(z)=b+Pz+(e^\mathsf Tz)Qz,\qquad
J_\chi(z)=P+(e^\mathsf Tz)Q+(Qz)e^\mathsf T.
\]

\(P,Q,b\)는 실제 upward/downward adjacent-charge transition 구조로 rate에 선형이다. Recombination은 neutral source에서 출발하지 않으므로 \(G_{\rm ne}\)의 neutral-offset action은 0이다. 이 구조 때문에 Hessian과 정확한 nonlinear remainder도 즉시 구체적이다:

\[
F_\chi(z+d)-F_\chi(z)-J_\chi(z)d=(e^\mathsf Td)Qd.
\]

### A1. Frozen coefficient 범위

고정 effective rates에서 모든 J entry는 \(z\)의 affine 함수다. 실제 total constraints

\[
0\le z_H\le T_H,\quad
z_{\rm He1},z_{\rm He2}\ge0,\quad z_{\rm He1}+z_{\rm He2}\le T_{\rm He},
\]

\[
z_{\rm Li1},z_{\rm Li2},z_{\rm Li3}\ge0,\quad
z_{\rm Li1}+z_{\rm Li2}+z_{\rm Li3}\le T_{\rm Li}
\]

의 product simplex는 **2×3×4=24 vertices**를 갖는다. 각 entry의 정확한 min/max는 이 vertices에서 취해지고 witness population을 보존할 수 있다. 같은 결과는 각 block의 affine support를 0과 coefficient extrema로 계산해 구성할 수 있다. Local chemical boxes를 추가하면 block별 bounded-simplex linear program으로 좁힐 수 있으나 최초 최소 단위는 full physical product simplex만으로 충분히 구체적이다.

### A2. Interval coefficient/clock 범위

12 effective rates에 독립 rational intervals를 제공하면, 고정 state vertex에서 J entry는 rates의 affine 함수다. State 24 vertices 각각에서 rate coefficient sign에 따라 endpoint를 선택하면 min/max를 정확히 구할 수 있다. 따라서 이 **declared independent rate-box × physical simplex family**의 entrywise extrema는 finite exact algebra로 얻을 수 있다. 실제 rate 사이의 공통 H·T·UV 상관관계를 무시한 만큼 actual family를 넓게 감쌀 수 있으므로 실제 모델에 대한 optimality를 주장하지 않는다.

Frozen rates는 한 고정 parameter choice에 관한 결과다. Interval rates를 전체 clock cell에 적용하려면 그 rate intervals가 실제 H/T/UV/n_b/source preparation을 감싼다는 별도 전제가 필요하다. 이 전제와 certificate algebra의 PASS를 분리한다.

### A3. Native norm 및 full30 연결

기존 LIFT·_row_terms·fixed_norm_id를 재사용한다. Chemistry variation을 physical33에 넣었을 때의 norm은 기존 metric의 정확한 restriction이며 새 weighted l1/infinity norm으로 바꾸지 않는다. Dual support를 쓰는 경우도 기존 support_core를 사용한다.

6×6 partial derivative block은 full30 certificate가 아니다. 다른 geometry columns, geometry rows 및 optical-depth row는 **UNKNOWN/OPEN**으로 남기고 0으로 채우지 않는다. 완전한 full30 assembly가 확보되기 전 기존 matrix_bounds에 이를 full Jacobian으로 공급하지 않는다. 별도로 명시된 polynomial coefficient operator Q에 관한 component bound를 개발할 수는 있으나 그 역할도 Jacobian 전체와 구분해야 한다.

정확한 quadratic remainder는 새 VigilODE 분기의 near-affine remainder premise와 연결된다. 다만 전체 source의 remainder에는 geometry·H dependence·clock terms가 남는다.

## 옵션 B — source RR/DR coefficient의 clock 전구간 enclosure

Atomic source literal coefficients와 prescribed thermal path \(T=100+11900Q(x)\), \(Q=(1+\tanh((7-z)/0.8))/2\), \(z=21e^{-x}-1\)를 사용하여 recombination coefficients의 \(T\in[100,12000]\) 상계를 직접 생성하는 단위다. 옵션 A에서 외부로 남긴 effective coefficient premise의 일부를 실제 source로 내려간다.

주의할 실제 source 구조:

- RR는 sqrt, exp와 temperature-dependent noninteger powers를 포함한다.
- DR의 He(2:1)와 Li(3:1)에는 **음의 fit coefficient**가 있다. 모든 summand가 양수라는 가정은 틀리다.
- 현재 rates 함수의 nonnegative/nonfinite 검사와 점별 값은 전구간 positivity 증명이 아니다.
- H denominator 및 proper n_b(x), prepared photo spectrum coefficients, native arithmetic error는 별도다.

Coarse analytic bounds는 실제 positive/negative coefficient 크기와 exponent 순서를 이용해 구성할 수 있으나 유용한 수치 폭과 source floating error까지 한꺼번에 닫는 일은 옵션 A보다 범위가 크다. Fixed source의 retained-process approximation/physical omission error도 그대로다.

## 옵션 C — Thomson·optical-depth 또는 mass-shell geometry kernel enclosure

Coupled source의 \(\tau'=\mathrm{THOMSON}\,n_e/H\), photon collision term \(-\mathrm{THOMSON}\,n_e H^{-1}D_\ell y_\ell\)는 실제 chemical/geometric coupling을 드러낸다. Source-defined electron row, H lower bound, hgrad enclosure를 사용하여 이 contribution의 Jacobian을 만들 수 있다. Source block coverage를 명시해야 하며 다른 components를 0으로 간주할 수 없다.

더 근본적인 geometry 방향은 positive finite packet mass-shell kernels \(\rho,\pi,\rho_b,\pi_b\)를 \((x,b)\) rectangle 전체에 interval로 감싸는 것이다. 준비된 packet arrays의 정확한 identity, exp/sqrt outward enclosures, normalization 및 accumulation errors를 다뤄야 하므로 최초 단계로는 A보다 크다.

## 새 VigilODE 분기를 실제로 사용한 연결

최신 census는 **222 branches / 63 open PRs**다. 기존 모든 branch heads와 PR metadata는 unchanged이고 새 분기는 research/integrated-polyalgorithm-20261028, HEAD **af7f3e9f55d652554042d2386bf07abb7d857d75**다. Parent는 PR93 HEAD 384dba091a3d9e1aac0ecddec264958298239253이다. 변경 9개 파일 전부를 회수·전체 독해했으며 모두 blob이 일치한다.

이 commit은 preregistration, 두 THEORY, 두 CONTRACTS, 두 CHECK source와 resume/skill context다. Runtime source 변경과 실행 결과가 없다. CHECK.py를 이 조사에서 실행하지 않았다.

- error_contract/THEORY.md의 E1–E3은 same physical-flow anchor, discrete-map anchor, norm·clock·error ownership를 구분한다. Chemical block algebra를 physical-step certificate로 올려 부르는 오류를 막는 직접 연결이다.
- algebra_contract/THEORY.md의 Theorem B/B6는 virtual stage의 **model-specific nonlinear remainder**를 실제 입력 의무로 요구한다. 옵션 A의 source quadratic remainder가 그 leaf의 구체적 후보다.
- 같은 source는 exact/virtual stage 연결 선분과 whole-domain Lipschitz/inverse를 요구한다. Chemistry leaf만으로 그 전체 전제를 채우지 않는다.
- 다른 분기의 workflow 1.3 selected stages는 그 분기의 기록이다. 현재 사용자가 지정한 full physmath1.0.0 R1–R10/C1–C10을 바꾸지 않는다.

이번에 읽은 fixed source와 새 VigilODE source에는 fixed binding의 accepted native-step capsule, model-owned whole-corridor full30 Jacobian enclosure 또는 BASS VerifiedModelProvider가 확인되지 않았다. 다른 조사자가 회수하는 backup에 관한 결론은 그 결과와 합쳐야 한다. 전 저장소·전 백업의 보편적 부재를 주장하지 않는다.

## 부모 결정에 필요한 최소 선택

추천은 **A2를 상위 계약으로 두고 A1을 exact control 및 zero-width specialization으로 포함**하는 것이다. Frozen e/totals/source topology, independent nonnegative 12-rate intervals, full product simplex, 36 entry extrema/witness, source quadratic remainder가 최소 구성적 목표다. Rate/time enclosure truth와 full30 coverage는 명시적 open outputs로 유지한다. 기존 next7 matrix bound와 native norm은 수정·재실행하지 않는다.

이 문서는 소스 조사와 후보 제안이며 다음 과학 계약이나 이론 실행의 승인을 대체하지 않는다. root가 다음 단위를 고정하기 전 구현·test·CAS를 작성하거나 실행하지 않았다.
