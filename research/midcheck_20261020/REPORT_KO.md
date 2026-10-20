# VigilODE 중간점검: residual-first, fast, Diffsol BDF, 원본 DVODE

## 0. 출처 및 판정

- 기준 데이터: 새 GitHub Actions run **39360536475** (`2026-10-20` UTC/KST). 동일 Ubuntu 공유 runner에서 빌드한 네이티브 비교기, 소스와 컴파일 조건은 `EXEC_SOURCE_COMMIT.txt`, `ENVIRONMENT.txt`, `Cargo.lock`, `BINARY_SHA256`, `SOURCE_SHA256SUMS`에 보존.
- 비교기는 엄격 비교 소스 `9c6a6d9145795e35b978652a567b806ddc84a6bd`를 고정해 최신 coupled companion(`research/coupled-economy-20261020`)에 빌드했고, 기존 다섯 stiff 문제의 방정식은 유지하며 `few-mode-96`만 추가했다.
- 보존 결과 `SUMMARY.json`: 39개 사전검사, **312/312 성공한 새 timing cells**, **26개 실제오차 2배 이내 정합 pair**, **4개 미정합 pair**. 별도 `AUDIT.json`: **312개 full-state 재계산**, **30개 paired median** 재검산, `PASS_SCOPED_RAW_STATE_READBACK`.
- 모든 수치는 per-integration/step median(ms), 8개의 새 프로세스 블록과 블록당 warmup 1회·time 5회의 기록이다. 통계 단위는 8개 블록이며 공유 VM의 시간은 진단용이다.

## 1. 같은 runner에서 재측정한 원본 stiff 문제 (endpoint, 단위 ms)

| 문제 (변수 수) | VigilODE fast | VigilODE banded | Diffsol0.16.1 BDF dense | Netlib DVODE |
|---|---:|---:|---:|---:|
| Robertson (3) | 0.02908 | 해당 없음 | 0.08920 | 0.07789 |
| HIRES (8) | 0.22672 | 해당 없음 | 0.45563 | 0.37634 |
| van der Pol (2) | 0.20282 | 해당 없음 | 0.80924 | 0.59148 |
| Brusselator (100) | 2.66248 | 1.53083 | 5.30989 | 0.69930 |
| Brusselator (400) | 29.86867 | 6.02362 | 193.34939 | 2.63843 |

위 다섯 문제에는 과거 정확도 기반으로 **시간을 보기 전에 고정한 solver별 rtol**을 적용했다. 새 출력은 독립 Radau/LSODA 경험적 참조에 대한 실제 최대격자 E를 새로 계산했고, 2배 이내 정합과 공통 anchor 기준을 모두 확인한 pair만 `MATCHED_ACHIEVED`로 분류한다.
원본 DVODE의 local tolerance는 global/output 오차에 대한 증명된 상계가 아니므로 `R_up>1`을 명세 위반으로 취급하지 않는다.

## 2. 동일 수식에서 직접 비교한 새로운 few-mode-96 (endpoint)

자율·항등질량 ODE: `dy_i/dt = a_i y_i + y_i^2`, `a_i∈{-1,-20,-100}` 주기 반복, `y_i(0)=0.1`, `t_f=0.001`. 참조해는 독립 Riccati 닫힌해를 계산했다. VigilODE fast와 coupled는 96변수의 같은 RHS·초기상태·출력 시각을 사용한다.

| 솔버/방법 | 입력 rtol | 시간 ms | 실제 E_up |
|---|---:|---:|---:|
| coupled-eager | 1e-6 | 0.103817 | 9.704179e-13 |
| coupled-lazy | 1e-6 | 0.086967 | 9.704179e-13 |
| vigilode-fast | 1e-6 | 0.023882 | 9.704179e-13 |
| diffsol-bdf | 1e-6 | 1.054369 | 1.063559e-8 |
| netlib-dvode | 1e-6 | 0.215195 | 6.818933e-7 |

Coupled eager/lazy의 관측 오차는 동일하고, 블록별 paired 중앙 시간비 `eager/lazy`는 **1.1969배** (관측 8블록 범위 1.1509–1.2360). 같은 error 수준에서 VigilODE fast는 coupled-lazy보다 약 **3.64배** 짧았다.

Diffsol과 DVODE에는 같은 입력 tolerance를 사용했지만 두 외부 BDF의 **실제 E_up가 coupled보다 크게 달랐으므로** 그 시간비는 4개의 `DIAGNOSTIC_ONLY_NOT_MATCHED` 중 일부다. 이를 외부 솔버에 대한 공정한 속도 순위로 발표하지 않는다. 특히 coupled는 고정 한 step이고 BDF는 기동/적응적인 구간 적분이라는 차이가 있다.

## 3. 성능상 중요한 결론

- **같은 coupled 구현의 계산량 개선은 관측됐지만, 작은/저비용 문제에서는 전체 solver 경쟁력이 부족할 수 있다.** few-mode-96에서 lazy가 eager보다 약 16% 짧았어도 fast는 lazy보다 훨씬 짧다. stage 병렬성과 homotopy를 무조건 켜는 dispatcher는 지지되지 않는다.
- **공간 이산화에 강한 banded BDF 대조는 여전히 남는다.** Brusselator N400의 새 endpoint 값에서 DVODE 2.6384ms, VigilODE banded 6.0236ms로 DVODE가 약 2.28배 짧았다. Diffsol은 이번에 FaerLU dense 구성이라 다른 sparse backend 결과로 일반화하지 않는다.
- **이전 보고서 절대시간과 이번 공유 runner의 절대시간을 직접 나누지 않는다.** 같은 소스라도 다른 CI 실행에서 표시 시간에 큰 변동이 있다. 단일 런의 매우 좁은 비율 범위를 다중 호스트 확정 성능으로 과장하지 않는다.
- **빠른 root 풀이와 정확한 trajectory는 다르다.** coupled가 반환한 operational full-stage residual 및 embedded 시험과 물리적 전체 trajectory·dense output의 정확도는 별개다.

## 4. 검증 가능한 파일과 재현

- Run: [Actions 39360536475](https://github.com/cosmosapjw-quantum/vigilode/actions/runs/39360536475)
- [결과 CSV](runs/39360536475/RESULTS.csv), [개별 8블록 시간비](runs/39360536475/PAIRS.json), [사전 실행](runs/39360536475/PREFLIGHT.json), [full states + raw timing gzip](runs/39360536475/RAW.jsonl.gz).
- [저장 결과 요약](runs/39360536475/SUMMARY.json), [독립 재계산](runs/39360536475/AUDIT.json), [빌드 및 과학적 소스](../midcheck_native_20261020/src/main.rs), [소스 변환 계약](patch_native.py), [재현 명령](run.sh).
- 최초 두 실행에서 소스-증거 commit 혼동과 감사의 pair 건수 판정 오류를 재현했고, 실패 로그를 보존한 뒤 수정했다. 최종 CI는 0으로 종료했으며 benchmark source와 frozen 결과 source는 명시적으로 구분했다.

## 5. 다음 최소 실험

few-mode에서는 BDF와 coupled의 **달성 오차가 2배 이내가 되도록 입력 tolerance를 별도로 교정한 뒤** 같은 호스트에서 재측정해야 세 경로의 공정한 상대 순위를 결정할 수 있다. 현재 조건의 시간은 성능 진단으로만 사용한다. 또한 중·대규모 stencil에서 실제 accepted time 단위 비용과 출력 오류를 함께 측정해야 coupled dispatcher의 승격을 논할 수 있다.
