# F6 로컬 원자료 분석

원자료는 `/home/cosmosapjw/evidence-20261028T170155Z`에 그대로 두었습니다. 이 분석은 native 실행 commit `299310290c2e254149ee30f4a9834c1123066815`와 allocation followup `a1a459f2be186d9f67ff86bf38c19ae24b769768`를 구분합니다. 측정 명령의 source SHA는 최종 보고 문서 SHA와 다릅니다.

## 정확도와 경로 검증

참조는 등록된 정확 다항식 `p(t)=3/4+t/8+t^5/64`이며, 격자는 `[-1,1]`에서 `N=16,32,64,128,256`개의 dyadic slab입니다. 각 경로의 물리 경로 오차 cap은 정확히 `1/100000000`입니다. 독립 Fraction oracle은 export된 모든 slab의 원래 방정식 잔차·domain·clock·chain·비용 기록을 검사했습니다. 총 992개 slab(2 route × 496) 모두 검사됐고, 65,672개 검사 후 bounded oracle이 `PASS_SCOPED_ALL_EXPORTED_SLABS_CHECKED`를 반환했습니다. 이는 낮은 격자의 물리 cap 통과를 뜻하지 않습니다.

| route | N | max nodal error | external whole-path upper | native whole-path upper | cap | integration + native cert (s) |
|---|---:|---:|---:|---:|---|---:|
| direct_moments | 16 | 5.03621650716e-07 | 2.487852302e-05 | 2.2244924217e-05 | FAIL | 0.00081646 |
| shared_polynomial | 16 | 5.03621650716e-07 | 2.487852302e-05 | 2.2244924217e-05 | FAIL | 0.000788969 |
| direct_moments | 32 | 3.17551136497e-08 | 2.9080514181e-06 | 2.73783933978e-06 | FAIL | 0.001602954 |
| shared_polynomial | 32 | 3.17551136497e-08 | 2.9080514181e-06 | 2.73783933978e-06 | FAIL | 0.001600098 |
| direct_moments | 64 | 1.99206395823e-09 | 3.50361445471e-07 | 3.39539967714e-07 | FAIL | 0.003231777 |
| shared_polynomial | 64 | 1.99206395823e-09 | 3.50361445471e-07 | 3.39539967714e-07 | FAIL | 0.003212791 |
| direct_moments | 128 | 1.24713017691e-10 | 4.29561597182e-08 | 4.22739818132e-08 | FAIL | 0.006469584 |
| shared_polynomial | 128 | 1.24713017691e-10 | 4.29561597182e-08 | 4.22739818132e-08 | FAIL | 0.007038197 |
| direct_moments | 256 | 7.80109310483e-12 | 5.3165252301e-09 | 5.27370737712e-09 | PASS | 0.013007648 |
| shared_polynomial | 256 | 7.80109310483e-12 | 5.3165252301e-09 | 5.27370737712e-09 | PASS | 0.013905934 |

두 route는 각 격자에서 반환 관측 시각과 state bits가 모두 일치했습니다. 두 independent whole-path 상한 모두 `N=256`에서 처음 cap을 만족합니다. `N=16..128`의 원래 residual/path 상한은 초과하므로 해당 행들은 정확도 부적합이며, oracle의 구조 검사 PASS와 분리했습니다.

## 비용과 변동

Native 전체 cargo run process는 172.702s였으며 compile과 driver 실행을 분리한 setup 시간이 아닙니다. 실행 내부에서 기록된 global diagnostic preparation은 0.057802793s지만 route별로 나뉘지 않습니다. 따라서 이 값을 route total에 임의 배분하지 않았습니다. route별 native total은 integration + native outward certificate입니다. external Fraction oracle 비용은 별도 audit 비용이며 production route 비용에 합치지 않았습니다.

N=128의 반복 timing은 1 warmup과 7 interleaved block입니다. trajectory phase는 정확한 certification을 반복한 게 아니라 128-slab RK4 route만 반복했습니다. 아래는 trajectory phase 요약이며 시간은 진단용입니다.

| route | min (s) | median (s) | max (s) | max/min |
|---|---:|---:|---:|---:|
| direct_moments | 0.000515352 | 0.000519279 | 0.000718797 | 1.395 |
| shared_polynomial | 0.00050373 | 0.000506085 | 0.000661098 | 1.312 |

첫 cap 통과 격자 N=256의 단일 paired native total은 direct `0.013007648s`, shared `0.013905934s`이고 raw `shared/direct=1.069058`입니다. 그러나 certification 포함 total의 반복 쌍이 없고 preparation도 route별 배분되지 않아 이 수치는 성능 비율로 승격할 수 없습니다. 일곱 반복 trajectory 표본의 median은 direct `0.000519279s`, shared `0.000506085s`입니다. 이 좁은 phase의 paired direct/shared 시간 비율은 전부 1보다 크고 median `1.02695`(범위 `1.02307–1.41955`)였지만, 짧은 N=128 workload의 진단용 측정이며 인증 비용을 포함하지 않습니다.

## 작업량과 음성 대조

경로 생성의 RHS 호출 수는 `4N`으로 N=16,32,64,128,256에서 각각 64,128,256,512,1024였습니다. N=128의 7회 반복 trajectory block에서 direct/shared는 각각 moment batch 16,384/8,192개, scalar additions 147,456/131,072개, scalar multiplications 212,992/172,032개를 기록했습니다. `jacobian_evaluations`, `jvp_evaluations`, `ad_evaluations`, `newton_updates`, `root_enumerations`는 계측한 해당 route work counters에서 0입니다. 이 사실은 이 고정 manufactured 경로의 범위에만 적용하며 일반 ODE solver 경로에 대한 derivative-free 주장은 아닙니다.

Fraction oracle의 8개 음성 대조(중근에서 distinct-root-count, corrupt residual, clock, chain, domain, false cap, provider work, total cost)가 모두 reject됐습니다. semantics 테스트는 all-complex multiplicity를 accept하고 selected-branch와 real-only 요청을 reject했습니다. allocation 검사는 successful slab arithmetic 1개와 rejected strict tube / successful slab arithmetic 2개를 통과했습니다. 이는 해당 slab hot-loop 경우의 시험이며 전체 orchestration이나 solver의 zero-allocation 증명은 아닙니다.

## 한계

이 자료는 all-complex algebraic-multiplicity observable을 가진 하나의 manufactured F6 궤적과 고정된 5개 dyadic grid에 한정됩니다. 일반적인 ODE 적분기, 다른 물리 모델, 넓은 차원, production route 선택을 승인하지 않습니다. 상세 축약 수치는 `SUMMARY.json`에 있고, raw stdout/stderr/oracle/receipts는 원래 위치에서 SHA로 연결됩니다.
