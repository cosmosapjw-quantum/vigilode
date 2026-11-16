# U09 geometry/MOL — 2026-11-16

판정: **U09 PARTIAL**. 기하 fixture, 공간/시간 오차 분리, 가중 인증서, mass/event capability는 확인했다. 구면 조화함수의 **bandlimit 안 정확성**은 spectral 이산화가 없어(유한체적만 구현) NOT_RUN이다. library 변경은 없다(test 파일과 근거만). Layout3(log-energy, μ, φ)는 Cartesian 3D가 아니므로 쓰지 않았다.

## 2D: 기존 Gray–Scott 연산자 재사용 (`rodas5p_integrators::gray_scott_2d`)
v=0에서 u의 RHS에서 반응항 F(1−u)를 빼 `D_u Δ_h`를 꺼냈다. Fourier mode (2,1)은 이산 고유벡터다(결함 ≤1e-10). 상수 mode는 소멸한다. 가중(H=dx²) 대칭이고 Rayleigh 몫 ≤ 0이다. 고유값 상대오차는 4.3e-2 → 1.1e-2 → 2.7e-3으로 공간 차수 1.98/1.99다.

## 3D: 비등방 주기 Cartesian (test 안 신규)
격자 (m, m, 2m), 길이 (1, 2, 0.5), D=0.1, mode (1,1,2). 이산 고유벡터 정확, 상수 소멸, 가중 대칭·소산, 공간 차수 1.98/1.99.
시간 분리: 보호된 RODAS5P(JVP 경로)로 반이산 mode를 t=0.05까지 적분했다. **시간 오차**(대 exp(λ_h t))는 rtol 1e-4/1e-6/1e-8에서 1.4e-7/1.2e-9/1.3e-11이다. **공간 진폭 오차** |exp(λ_h t) − exp(λ t)|는 6.2e-3이다. 이 격자에서는 공간 오차가 지배하며, 시간 허용오차를 줄여도 감춰지지 않는다.

## 구면: 위경도 유한체적 (H = 셀 면적, 극에서 flux 0)
- 면적 합 4πR² (1e-10), 상수 소멸, H-가중 대칭과 Rayleigh ≤ 0, 가중 질량 균형 Σ H(Lu) ~1e-14.
- 가중 L2 상대오차 차수: Y10 2.00/2.00, Y20 1.99/2.00, Y11 1.87/1.89.
- **극 고리 오차**: Y10·Y20은 가중오차와 같은 2차, **Y11(m≠0)은 극 고리에서 1차**(2.4e-2 → 1.2e-2 → 6.1e-3). 극 정칙성 저하로 기록한다.

## 인증서와 capability
- 중앙·upwind 이류는 가중 자기수반 검사에서 결함 0.056/0.028로 거부된다. 가중 소산성/resolvent 증명 없이는 SDC 인증 경로를 쓰지 않는다.
- registry: mass + events + dense output을 동시에 지원하는 method는 RODAS5P뿐이다. PPSW1과 matrix-free 경로는 거부된다.

## 검사와 남은 의무
`geometry_mol_contracts.rs` 5/5, strict clippy·fmt 통과. 남은 의무: spectral 구면(bandlimit 정확성), m≠0 극 정칙화, 비선형 반응·비등방 확산의 결합 IVP, 공간·시간 refinement를 함께 한 오차 곡선(현재는 고유값과 단일 mode).
