# VigilODE CHEM6-MU01: source-structured signed lognorm 연구 보고서

**2026-10-29 KST** · `cosmosapjw-quantum/vigilode` · 기준 PR102 `8357faa5b42724a8be24609d4d2f16aec8a6eb3a` · 검증 code base `0d722b4ed71413926c76bc0037f02f3a6bbdd362`

## 결론과 적용 범위

고정 BASS chemical6 source topology `F=b+Pz+(e^Tz)Qz`에 대해, 24개의 H/He/Li product-simplex vertices, 6개 행, 각 행의 32개 sign patterns와 12개의 독립 rate endpoints만으로 **고정 weighted-infinity norm의 signed logarithmic norm 최댓값을 정확히 구하는 유한 알고리즘**을 유도했다. PR95의 개별 36-entry exact interval extrema와 달리, 이 계산은 **동일 state/rate witness에서 달성되는 하나의 lognorm scalar**를 직접 최적화한다. Source의 `(Qz)e^T` electron feedback을 유지한다.

이 결과는 actual accepted owner rates, full30 Jacobian, physical33 non-diagonal Gram norm, source-clock corridor, 전체 ODE 또는 production을 검증하지 않는다. 원본 23-node DAG는 `7/23 inherited scoped` 그대로이며 중앙 REI `28/88`도 별개다. 신규 보조 단위 `CHEM6-MU01`은 독립 저자 R6/C9가 수행되지 않아 **PASS_SCOPED_ALGORITHMIC_PENDING_INDEPENDENT_REVIEW**, 배포 `HOLD`다.

## 정확한 수학

이론은 `THEORY_KO.md`에 전개했다. 가중 무한노름 `||v||_w=max_i |v_i|/w_i`, `w_i>0`에서

`mu_w(J)=max_i[J_ii+sum_(j!=i)|J_ij|w_j/w_i]`.

`|a|=max(a,-a)`를 각 offdiagonal에 적용한 signed row는 fixed state에서 12 rates의 affine 함수이고, fixed rates에서 state의 affine 함수다. 따라서 그 최대는 24 physical vertices 및 rate endpoints에서 달성된다. Rate corner `2^12`를 열거하는 대신 signed row coefficient별로 상한/하한 rate endpoint를 선택한다. 정확한 state/rate witness를 반환한다.

실제 해와 represented path가 동일 physical simplex 안에 있고, 전체 clock slab에서 rates가 선언 box에 포함되며, 원래 방정식 defect 상계 `delta`, 초기오차 `E0`가 주어지면 `D+E<=mu_* E+delta`다. `m=max(mu_*,0)`, `mh<1`에서는 coarse rational whole-slab bound `(E0+h delta)/(1-mh)`를 얻는다. 이것은 아직 **입력된 실제 defect가 없는 조건부 소비식**이다. Electron event `e^Tz-theta`의 dual constant `sum |e_i|w_i`와 엄격한 부호 판정도 구현했다. Unique crossing은 `e^T F`의 transversality certificate가 추가로 필요하다.

## 계산 결과와 독립 알고리즘

선언한 synthetic dyadic rate-box 예제에서 exact rational 계산은

- `mu_sharp = 378302368699121663 / 2305843009213693952`, binary64 표시 `0.1640625`.
- `mu_entrywise = 49/256 = 0.19140625`.
- 상계 감소율 `(mu_entrywise-mu_sharp)/mu_entrywise`는 약 **14.2857%**. 이것은 이 synthetic box에서의 **인증 상계 차이**이며 물리 정확도나 ODE wall-time 개선이 아니다.
- Exact attaining witness: chemical row 4 (0-based), state vertex 23, `z=(T_H,0,1,0,0,1)`, rate endpoints는 `EXACT_RESULT.json`에 보존.

검증은 `pytest` 8/8 PASS. 독립 SymPy flux 미분으로 12 rate-basis Jacobians ×36 entries를 대조하고, 4 variable-rate full-corner direct-J oracle와 exact signed-row optimizer가 일치했으며, 300개 interior random rational cases가 상계를 넘지 않았다. Feedback omission과 음수 rate, 0 norm weight, reversed interval, `mh>=1` negative controls를 보존했다.

Rust 1.94.1 standalone crate (`chem6_lognorm.rs`, `Cargo.toml`)는 `#![forbid(unsafe_code)]`, x86_64 nearest-even/gradual arithmetic probe, outward `next_up/next_down`, 24×32 finite sign optimization, source/clock/unit guards, coarse Dini bound와 event strict-sign consumer를 구현했다. `cargo test --offline --all-targets`: **8/8 PASS**, `cargo clippy --offline --all-targets -- -D warnings`: **PASS**. 초기 Clippy 5 diagnostics는 이름·iterator 표현만 변경하여 해결했고 원본 실패 로그를 보존했다. Python `Fraction.from_float` 독립 비트 대조 7/7 cases PASS, 최악의 native upper minus exact bound는 `8.326672684688674e-16`이다. Rust 결과는 outward **upper**이며 exact maximum equality는 Fraction oracle의 주장이다.

`CHEM6_LOGNORM.png`는 5개 synthetic rate uncertainty widths에서 sharp bound와 entrywise bound를 비교한다. 그림 원자료는 `FIGURE_DATA.json`; 실제 이미지 시각 검토에서 axes, 범례, 증가 추세가 데이터와 일치한다. 측정 성능 또는 관측량을 표시하지 않는다.

## 오류·실패 분류

- 최초 Wolfram 대수 호출은 uppercase symbol 충돌/재귀 경고로 실패했으며 증거로 PASS 취급하지 않았다. 별도 3×3 generic symbolic Jacobian을 안전한 변수명으로 재평가한 결과 차이 행렬의 원소 집합은 `{0}`이다.
- 첫 native Clippy `-D warnings`는 5개 software lint 지적을 반환했다. 수학식을 변경하지 않고 이름·iterator를 교정했으며 targeted Rust tests, Clippy, Fraction bit oracle를 다시 통과했다.
- Native subagent tool이 현재 세션에 없어 다른 저자에 의한 R6/C9 review는 **NOT_RUN**. 독립 **알고리즘** 검증(SymPy/direct corners/Fraction)과 독립 **reviewer**를 구분한다.
- 컨테이너 Git HTTPS는 DNS resolution 오류였으나 GitHub connector의 non-force branch/commit/readback 경로는 이용 가능하다. 원격 source push의 성공은 실제 connector ACK와 readback에만 근거한다.

## 다음 실행점

1. 실제 owner-accepted reduced30 capsule, chemical6 isotope scales/effective rate intervals, physical clock `x=ln(a/a_ref)`, geometry/H/nb, initial global error, FP/accepted-step identity를 확보한다. 이 source 없이는 conditional `mu_*`를 실제 물리 bound로 승격하지 않는다.
2. 이 chemical6 norm과 physical33 native Gram 사이의 **명시적 norm transport** 및 crossblock coupling을 검증한다. Full30 `J`와 signed `mu`를 구성하지 못하면 full-model result는 null.
3. Actual native dense quartic의 vector residual `p'-F(x,p)`를 Bernstein degree8 및 coefficient/source rounding으로 감싸고, p∈S와 initial error를 검사한다. 필요하면 DOPRI piecewise 별도 route를 연다.
4. Event unique crossing에는 full-tube `e^T F` sign, transversality와 actual query rounding을 별도로 요구한다.
5. 기존 Q-LEDGER/authority, Q-API breaking change, Q-GIVENS profile, 10 interactions, full native/release CI, negative-cost 결과를 보존한다. 이전 campaign 재실행이나 NCP 재활성화는 하지 않는다.

이론 문헌: Neumaier (1994) https://arnold-neumaier.at/ms/ode.pdf; Söderlind (2006) DOI 10.1007/s10543-006-0069-9; Akrivis–Makridakis–Nochetto (2009) DOI 10.1007/s00211-009-0254-2. Actual source: VigilODE PR95 §3–6, BASS PR233·234; prior SYM02 PR102. 새로운 일반 no-go, 전체 solver speedup 또는 production certification은 주장하지 않는다.
