# BASS21 원본 193개 저장시점: 이론식과 계산 계약

**범위:** BASS @ `cc602e209e7deeb511ab2a1f3b2268d55206c43c`의 `s00_m00_r00/trajectory.npz`, 254301 bytes, SHA256 `37d1bab173e81449cf474b21a678077edd0150cf0b2cb386d014764580678086`. 기존 저장된 62차원 BASS compact33/full62 시스템의 193 진단시점을 **있는 그대로** 평가. 새로운 IVP, source 변경, fitting, REI17/Puchwein19 대체, RODAS promotion 없음.

## 물리적 정의와 시간 변수

원본 x = ln(a/a_start), a_start=1/21, z=21 exp(-x)-1, x∈[0,ln7]. BASS 기하학 저장값 H=d ln a/dt, 단위 s^-1, t는 고유시간 s. 이 해에서 193개 저장시점 x가 등간격이며, 이는 **원래 BDF 내부 적응적 steps**를 뜻하지 않는다. 핵종/전하 기저 Y_i=n_i/n_b, A_i=핵자수, q_i=전하. n_b,n_e는 proper m^-3. 초기 온도=100 K, 후기 온도≈12000 K; UV bath는 고정된 원래 z에 따른 prescribed closure.

동일 source `AtomicNetwork.evaluate`의 화학 계는

```
n_e = n_b (q^T Y)
F(Y,T,UV,n_b) = (G0(T,UV)+D+n_e Gne(T,UV)) Y  [s^-1]
J = G0+D+n_e Gne+n_b (Gne Y) q^T  [s^-1]
dY/dx = F/H
```

여기서 G0,D [s^-1], Gne [m^3 s^-1], n_e [m^-3]. 전하 생성률을 각각의 photo/RR/DR/weak 채널로 분리하고 `n_b q^T F`를 계산했다. 이는 고정된 n_b에서 chemical electron source이고, 총 `dn_e/dt`와 다르다. 일반적으로 `dn_e/dt=n_b q^T F+(dn_b/dt)(q^T Y)`이다. 저장된 본 BASS 유체의 baryon conservation이 성립하면 후자는 `-3H n_e`이지만 이 테스트의 동역학 source를 변경하지 않았다.

## 동일 BASS source/clock을 이용한 optical-depth 비교

```
THOMSON_m3_s = (2.99792458e10 cm/s)(6.6524587051e-25 cm^2)/1e6
             = 1.994356946945426e-20 m^3/s
tau_x = d tau_T/dx = THOMSON_m3_s n_e / H
tau_T(x) = ∫[0,x] tau_x(s) ds
```

원본 `input/coupled.py` BDF RHS의 `out[61]=THOMSON*ne/H`와 `diagnostics.gamma_T_per_H`를 직접 확인했다. 원본 `tau_T`는 `sol.y[61]`에 저장된 **BDF가 원래 진화시킨 누적 상태**이지, 이번 193개 시점에서 재구성한 Simpson/사다리꼴 결과가 아니다. 기하학 H는 `diagnostics.H`를 직접 사용한다. FLRW의 대체 H나 opacity를 역산한 H를 쓰지 않는다. 비경사·균질 전자계·상수 σ_T인 source-defined scalar optical depth이며, 현재에서 과거로 적분하는 관측 optical depth 또는 방향별 scattering을 의미하지 않는다.

## 출처와 수치 부호·오차

Source input: `trajectory.npz` 원본 Git blob `57a5375c5183f83d3eb3dd094cac31a212c39de3`, 원본 RESULT.json Git blob `4ee306f3af3ef5391d6309794cccfa1459067441`; BASS atomic source sha `baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80`. 원본 129개 실수 coefficient는 이전 PR269 수치 projection과 일치하며 **원본 coefficient JSON 전체 바이트의 동일성을 인증한 것은 아니다.** 원래 BASS source models의 prescribed temperature/UV, finite photon bins, effective photo yields, photon heating non-closure 등이 그대로 한계를 결정한다.

고정 `t_eval=193` 격자에 대해 `cumulative_trapezoid` 및 `cumulative_simpson`를 나란히 기록한다. 원본 방법은 `solve_ivp(BDF,jac=...,rtol=2e-8,geometry_atol=2e-11,atomic_atol=2e-10,tau_atol=2e-11,max_step=.025)`; 이번에는 절대 **재호출하지 않는다**. 사다리꼴과 Simpson의 오차 차이는 sampled-curve quadrature/interpolation의 영향, BDF 저장값과의 비교에는 원래 solver error도 포함된다. source mismatch는 모든 193개 epoch의 native `tau_x`와 stored `gamma_T_per_H` 동등성 검증으로 별도로 판별한다. 전역 truncation bound나 생산 solver 정확도를 이 대조만으로 보증하지 않는다.

`Y` 원본 저장 배열은 Fortran-strided. 행을 C-contiguous 복사하여 source evaluator에 전달하면 원래 BASS diagnostic `n_e`와 binary64 정확히 같으며, raw stride로 직접 곱하면 BLAS/reduction 순서에 따라 최대 약 8.53e-14 m^-3의 반올림 차이가 있다. 이는 물리 source 차이가 아니다.

## 유효 범위

`PHOTO absorption`은 photon-absorbed power일 뿐 gas heating이 아니다. LiI 1s yield/branch는 native finite physical test source의 가정; real cascades, full spectral RT, observational CMB, Bianchi anisotropic photon Boltzmann, posterior/likelihood는 검증하지 않았다. Native RODAS stage output/epoch transaction 및 genuine distinct-author R6/C9 미수행. PR117 derivative-free RHS/value-only 우선순위는 기존대로 유지.
