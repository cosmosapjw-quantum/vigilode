# BASS21 저장된 실제 물리 이력의 재평가: 전자 source, H–Li, 광학깊이

2026-10-29. 범위: BASS 원본 `cc602e209e7deeb511ab2a1f3b2268d55206c43c`, 원본 `AtomicNetwork`/초기 isotope inventory와 source-matched compact33/full62 저장 이력. REI17/p19 대조는 참고용이며 이 결과의 state source가 아니다. 새로운 IVP는 실행하지 않았다. 원래 BASS pilot는 prescribed UV/temperature, test-fluid closure와 불완전 광자 수송/가열 closure를 가지므로 관측적 우주론 결과로 승격하지 않는다.

## 정의, 차원, 시간 변수

Y_i=n_i/n_b는 baryon당 핵종/이온 숫자, q_i는 ionic charge, A_i는 질량수. n_b,n_e는 proper m^-3, t는 proper seconds, x=ln(a/a_ref). 원래 모델:

F(Y;t)=(G_0(t)+D+n_b(t)(q^T Y)G_ne(t))Y,   dY/dt=F,

n_e=n_b x_e, x_e=q^T Y.

G_0,D는 s^-1, G_ne는 m^3/s. J=G_0+D+n_e G_ne+n_b (G_ne Y)q^T. `AtomicNetwork.evaluate`가 실제로 이 식을 계산한다. 충돌 이온화/charge exchange/분자/외부 nuclear source는 이 finite source에 포함되지 않는다. 성분별 이온화 event에서는 q^TS=+1, RR/DR event에서는 q^TS=-1. 원래 finite source의 neutron/T 약붕괴는 전자를 방출할 수 있으나 원래 BASS pilot의 해당 두 inventory는 0이다. 그 결과

(dx_e/dt)_chem = q^T F,
(dn_e/dt)_chem|_nb = n_b q^T F = n_b (R_photo-R_RR-R_DR+R_weak),

dn_e/dt = n_b q^T F + (dn_b/dt) x_e.

`(dn_e/dt)_chem`을 전체 시공간 electron-density derivative로 해석하지 않는다.

## 저장된 τ와 순간 opacity

원래 `coupled.py`는 dτ_T/dx=(c σ_T)n_e/H, cσ_T=(2.99792458e10 cm/s)(6.6524587051e-25 cm²)/1e6=1.994...e-20 m³/s로 구현한다. 역으로 기존 저장 `γ_T/H`와 `n_e`에서 H=(cσ_T)n_e/(γ_T/H)로 **기존 기하학 값을 재구성**한다. 이것은 새로운 geometry/Einstein 방정식 풀이가 아니며, 저장된 cumulative τ는 기존 193 epoch BASS 실행의 결과이다. 계산된 H를 새로운 full62 진화의 소유권 증인으로 인정하지 않는다.

## 계산된 물리 출력

원래 127 endpoint에서 q^TY, Σ A_iY_i, lithium charge-pool, He4 charge-III fraction, stored τ, H를 계산했다. BASS 원래 source의 129 binary64 coefficient 값을 byte-verified primitive source 구조에 재결합한 기존 PR269 loader로 selected 5 physical endpoints의 F,J, photo/RR/DR, absorbed photon power, primary photoelectron power, residual core deposition을 다시 계산했다. 광자 absorbed power는 thermal heating이 아니다. 활성 Li I 1s coarse-grained q->q+1 yield 또한 원래 source 가정이며 실제 Auger/fluorescence 분기 구현이 아니다.

## 실제 25 matched stored histories의 의미

기존 125-grid에서 같은 5×5 물질/복사 targets를 두고 Σ_F=0과 Σ_F=0.6을 pairing. 평균 Δτ_T=-0.00012407565482797477; 평균 상대 Δτ=-0.0017893609445001236. 25/25 음수. 같은 endpoint의 |Δx_e|는 1.32e-12..1.60e-12. 저장된 전체 이력의 τ와 endpoint 화학 반응의 차이를 보존한다. 기하학/이력에 대한 조건부 sensitivity이며 universal monotonic response, data likelihood, cosmological inference가 아니다.

## negative evidence and gates

- REI PR265: original pilot H1/He4 inventory mismatch 약 0.4943..0.5186%, 불변.
- REI PR269: BASS phenomenological UV vs P19 spectral shape nonidentity, 불변.
- REI PR270: 동일 H/He 6 moments로 LiI photo response를 고유하게 고정하지 못하는 positive spectral freedom, 불변.
- VigilODE PR117의 derivative-free causal/RHS-only 우선 설계 불변. 이 산출물은 dense J가 production 주 경로라는 주장이 아니다.
- 원래 BASS finite21 RR/DR/photo source의 fit validity, 원래 numerical source, saved 193 epochs는 제시된 테스트 모형 수준이며 원래 관측 데이터/임의 Bianchi/model class 검증이 아니다.
- R6/C9 genuine distinct-author review NOT_RUN, new full-IVP NOT_RUN, full native RODAS stage or epoch transaction NOT_RUN, production HOLD.

## 문헌과 방법 출처

[Verner et al. 1996](https://www.pa.uky.edu/~verner/photo.html) ground-state photo fits; [Verner & Ferland 1996](https://doi.org/10.1086/192284) RR fits; [Badnell RR data](https://amdpp.phys.strath.ac.uk/tamoc/DATA/RR/), [Badnell DR data](https://amdpp.phys.strath.ac.uk/tamoc/DATA/DR/). 원자료의 범위를 명시한 `atomic_sources.json`이 공식 provenance. 문헌의 fit 권위와 이번 코드의 실제 사용/출력 적격성은 별도다.

수학 상태: 전자 source identity DERIVED from original source; BASS stored cases NUMERICALLY_RECOMPUTED (scoped); cosmological observables inference UNRESOLVED.
