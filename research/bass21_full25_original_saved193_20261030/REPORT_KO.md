# VigilODE × BASS21: 원본 저장 전단 25쌍 전체 물리 이력 보고서

**날짜:** 2026-10-30 KST. **source-owner:** `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`. **기준 PR:** [VigilODE #124](https://github.com/cosmosapjw-quantum/vigilode/pull/124), frozen HEAD `e48a049e143bb9d57a1d9e2fb2b570c32d0db2dc`. **결정:** 원본 저장 모델 데이터의 scoped physics atlas 완료 / 독립 검토 및 production HOLD. 독립 R6/C9 NOT_RUN.

## 1. 이번 단위의 실제 생성 결과

원본 BASS GitHub 고정 커밋에서 40개 새 `trajectory.npz`의 원본 `RESULT.json` 예상 크기/SHA256과 실제 Git blob을 대조했고, 기존 PR122 원본 10개와 합쳐 50/50 original binary SHA256/Git blob/ZIP CRC를 확인했다. 각 193개 epoch, 50×193=**9,650 saved physical states**. 동일 source `AtomicNetwork.evaluate` **9650회** 직접 실행했고 original physical 21-component F, 21×21 Jacobian J, photo/RR/DR 전자 생성과 H/He/Li 7 photon-ionization rates, absorbed/primary photon work를 물리 단위와 함께 저장했다. 뉴런/ODE surrogate 또는 새 IVP는 **0개**다.

산출물 `ALL_50_NATIVE_193_CHEMISTRY.csv` (9650 rows), `ALL_50_NATIVE_FJ.npz` (50×193×21, 50×193×21×21), `MATCHED_25_X_193_OPTICAL_DEPTH.csv` (25×193=4825 rows), `MATCHED_25_PAIR_SUMMARY.csv` (25 rows), `CLOCK_QUADRATURE_50_CASES.csv` (50 rows), 원본 50 NPZ/이론 및 20개 집중 테스트/그래프 3장. 매 원본 입력에 대해 `results/SOURCE_INPUT_MANIFEST.json`에서 expected SHA256+Git blob과 physical owner를 보존한다. 전송 중 부분 실패 1건은 `INCOMPLETE_DISCARDED`로 고정하고 재수집본만 사용했다.

## 2. 물리적 결과 (derived + numerically-checked)

- **25/25쌍**에서 원본 저장 누적 optical-depth contrast \(\Delta	au_T<0\) (전단 0.6 minus 0), 평균 \(-0.000124075654827975\), range [-0.00012517786846776, -0.000122934831682991]. Original forward x clock z=20→2, 관측 CMB 후방 광학깊이 아님.
- 동일 native chemistry와 density bath를 사용한 순간 \(\Delta(Kn_bx_e/H)\) 정확한 3항 유한 분해의 적분: geometry share 평균 **92.07015546%**, 범위 92.02167046~92.11687022%; ionization share 평균 **7.92984454%**, 범위 7.88312978~7.97832954%; density 0. 이 수치는 모두 193점 composite Simpson postprocess. 임의의 독립된 인과 개입 비율이 아니다.
- m/r 조합 전체에서 geometry fraction은 아래와 같이 변화한다. 이는 **25개 원본 저장 표본의 수치 패턴**이며 연속 파라미터 전역 정리가 아니다.

| m/r | r00 | r01 | r02 | r03 | r04 |
|:---|---:|---:|---:|---:|---:|
| m00 |92.1169% | 92.0959% | 92.0748% | 92.0535% | 92.0321% |
| m01 |92.1151% | 92.0942% | 92.0730% | 92.0518% | 92.0304% |
| m02 |92.1134% | 92.0925% | 92.0713% | 92.0501% | 92.0286% |
| m03 |92.1100% | 92.0890% | 92.0679% | 92.0466% | 92.0252% |
| m04 |92.1067% | 92.0856% | 92.0645% | 92.0432% | 92.0217% |

- 각 이력의 \(n_b(x),T(x),UV(x)\)는 matched pair에서 원본 바이트 수준 array equal. 7개 H/He/Li 광이온화 계수는 **25×193×7 = 33,775개 비교에서 정확히 동일**. 달라지는 것은 \(Y,H\)와 실제 photoionization/recombination net source, proper-time exposure다.
- **chemistry 항의 99.0996%**가 중간 \(z=11.910551	o6.937254\) 구간에서 누적되며, 모든 25쌍의 해당 집중 비중은 99.0979~99.1022%. 반면 geometry는 그 밖의 구간에서도 계속 누적된다. 최종 \(\Delta x_e\sim10^{-12}\)는 이력 전체의 \(\max|\Delta x_e|pprox0.0005179085531\)을 대표하지 않는다.
- 기존 PR122 다섯쌍의 원본 Δτ/Simpson total/geometry/chemistry는 **수치 값 동일(각 항 차이 0)**. 별도 결과를 재사용하여 5→25 coverage를 확장했다.

## 3. 검산·실패 보존

- 50개 입력 SHA256/Git SHA-1 binary data exact, 원본 photon/atomic network/model source SHA256 checked, NPZ CRC pass.
- 25쌍×4 원본 epoch **100개 exact `fractions.Fraction` product-identity checks** PASS; Wolfram `FullSimplify=0` (source-independent algebra); 20개 Python focused unittest PASS (실제 커맨드/exit 로그 별첨).
- native \(n_e\), \(d	au/dx\), photon absorbed power와 저장 진단값은 최대 절대차 0; \(\max|A^	op Y-1|=2.10942e-15\). Native 직접 단일 epoch F/J와 저장 bundle 간 \(pprox10^{-32}\) 수준 numerical sub-ulp drift 발견: 이를 byte-identical로 주장하지 않음. 새 unit-test는 model scale보다 충분히 작은 absolute tolerance로 비교하며 원래 솔버 tolerance는 바꾸지 않음.
- 50개 개별 저장 \(	au_T\)와 같은 193-grid 재적분의 최대 시점별 Simpson difference **6.8130566e-08**, trapezoid **5.8804425e-06**. 25쌍의 \(\Delta	au\) Simpson/저장 차이 최대 **5.2269242e-10**. 이는 저장 BDF와 후처리 sample quadrature·interpolation 차이 관찰이며, 원래 solver의 전역 오차 인증 또는 수정이 아니다.
- source/epoch independent audit R6/C9 NOT_RUN. PR118의 genuine pre-registration/ledger 미등록 CI-FAIL, PR117 causal RHS-value-only 우선순위, Q-LEDGER/Q-API/Q-GIVENS 및 기존 8 suites/10 interactions 실패/예산과 native RODAS production 보류를 그대로 둠. 이 코드는 Rust production solver를 수정하지 않는다.

## 4. 진척도와 물리적 후속

- 공식 VigilODE **7/23**, N03 이전 기록 **17/20 scoped**, REI central **28/88**, REI working **33/88**, successor **0/12 admitted**, original physical node delta **0**. 이번 auxiliary **25/25 pairs, 1/1 scoped source-owned data**; independent review HOLD. R/C 수행 scoped count **17/20 complete, C8 partial, R6/C9 NOT_RUN**. 날짜와 PR 게시 상태는 `PUBLICATION_RECEIPT.json`에 실제 도구 ACK 후 별도로 기록.
- 가장 직접적인 후속 물리 노드: 같은 원본 BASS chemistry와 별도로 계산된 angular Liouville/Bianchi-I sky의 **동일 source/clock + microphysical Thomson collision operator** 결합 계약을 새로 admission하고 observer directional kernel을 구성한다. 이번 25쌍 결과만으로 관측 CMB E/B나 REI Grackle/P19 물리 동치를 생성하지 않는다.

핵심 세부 유도: `THEORY_KO.md`; 주석 없는 원본 데이터: `inputs/`; 코드: `code/full_history_replay.py`, `code/clock_quadrature.py`; 검증: `tests/test_full_history.py`, `VERIFY.sh`; 실패 증거와 R/C 상태: `results/TEST_*.txt`, `STAGE_LEDGER.json`.
