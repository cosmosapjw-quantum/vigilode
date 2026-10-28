# 독립 세션에 대한 유한표본 median 구간 후보

이 문서는 R4의 새 통계 후보를 직접 유도한다. 기존 bootstrap을 수정하거나 통계적 권위를 승격하지 않는다. 실제 검산 상태는 `EXACT_SESSION_INTERVAL.json`과 실행 receipt에 따로 기록한다.

## 정의와 전제

사전에 정한 유한 case 집합의 크기를 C라 하자. 각 독립 세션 s에서 case c의 ABBA log-speedup들을 계산하고 그 셀의 median을 Z_sc로 둔다. 세션 벡터 Z_s=(Z_s1,...,Z_sC)는 독립이고 동일한 분포를 갖는다고 가정한다. case 사이의 독립성은 가정하지 않는다. 모든 세션은 동일한 C개 case를 완전히 측정해야 한다. 누락 셀이나 실패 세션은 이 후보의 입력에서 명시적으로 거절한다. 성공 세션만 고르는 outcome-dependent selection에는 아래 정리가 적용되지 않는다.

각 case의 모집단 median m_c를 한 개 지정하여

P(Z_sc <= m_c) >= 1/2,  P(Z_sc >= m_c) >= 1/2

라고 하자. 관심량은 theta = median_c(m_c)이며 case 가중치는 동일하다. 로그를 취하기 전 reference/candidate 시간의 비는 무차원이므로 theta도 무차원이고, speedup은 exp(theta)이다.

이 estimand는 기존 코드가 사용하는 pooled-pair 분포의 median과 일반적으로 다르다. 기존 coverage simulator의 대칭적인 additive random-effects 모형에서는 session-cell median의 분포도 theta+v_c를 중심으로 대칭이므로 두 estimand가 일치한다. 다른 timing 법칙에서는 자동으로 같다고 놓으면 안 된다.

## Case별 정확한 순서통계량 구간

S개 세션의 c번째 값들을 Z_(1)c <= ... <= Z_(S)c로 정렬한다. k>=1에 대해

I_c = [Z_(k)c, Z_(S-k+1)c]

로 둔다. m_c < Z_(k)c이면 m_c 이하인 관측 수가 k-1 이하이다. 그 수는 success probability P(Z_sc<=m_c)>=1/2인 Binomial 변수이고, 이 사건의 확률은 success probability=1/2일 때 가장 크다. 반대쪽도 같은 논리로

P(m_c not in I_c) <= q(S,k),
q(S,k) = 2 * 2^(-S) * sum_{j=0}^{k-1} binom(S,j).

연속 분포의 유일한 median에서 이 값은 정확하다. median에 원자가 있으면 이 부등식은 보수적이다. 수학적 보장에는 Gaussian 분포, 분산 추정, pair 독립성, bootstrap Monte-Carlo 근사가 필요하지 않다.

원하는 전체 실패확률 alpha에 대해 q(S,k)<=alpha/C를 만족하는 가장 큰 k를 고른다. 만족하는 k가 없으면 유한 구간을 억지로 만들지 않고 전체 실수 구간을 반환한다.

## 고정 case corpus의 median으로 전달

Union bound에 의해 모든 m_c가 각각 I_c에 들어갈 확률은 최소 1-C q(S,k)이다. case 간 상관이 임의적이어도 성립한다. Median은 각 좌표에 대해 단조이므로 동시 포함 사건 위에서

median_c Z_(k)c <= median_c m_c <= median_c Z_(S-k+1)c.

따라서 양쪽 endpoint의 case median으로 만든 구간은 theta를 최소 1-alpha 확률로 덮는다. 이 단계는 정확한 단조성 정리이며 새 bootstrap이 아니다. 짝수 C에서도 가운데 두 순서통계량의 평균이라는 median 정의를 양쪽에 일관되게 사용하면 단조성이 유지된다.

## 해상도와 극한

alpha=0.05, C=1, S=6에서는 k=1이고 coverage가 31/32=0.96875이다. C=5, S=6에서는 5*(2/64)>0.05이므로 이 construction은 무한 구간을 낸다. C=5, S=8에서는 k=1, 전체 coverage 하한은 1-5*(2/256)=123/128=0.9609375이다. 이는 이 construction의 유한 구간에 필요한 세션 수이며 모든 가능한 통계 방법의 불가능성 정리로 확대하지 않는다.

세션 안에서 pair를 반복하거나 case 이름을 늘려도 독립 세션 S가 늘지 않는다. 기존의 5-case bootstrap이 특정 모형에서 보수적이었다는 관측 역시 일반 보장이 아니다. 다섯 case의 세션별 값이 모두 동일하면 case resampling이 값을 바꾸지 않아 정확한 bootstrap 분포는 1-case 분포로 환원된다. 서로 다른 case를 셌다는 사실만으로 실패한 1-case coverage 문제가 제거되지 않는다.

## 결정 규칙과 구현 범위

완전한 검증된 입력에 대해서만 lower > log(1.15)이면 Promote, upper < log(1.15)이면 Block, 나머지는 Inconclusive로 둔다. threshold tie는 보수적으로 Inconclusive이다. 이 연구 후보의 경계 선택은 기존 >= 규칙을 바꾸는 production patch가 아니다.

표본 수와 C, alpha, case 집합, complete-session admission, 고정 실행 종료 규칙을 관측 전에 정해야 한다. 순차적으로 세션을 늘리거나 여러 후보 중 좋은 것만 택하면 별도 오류예산이 필요하다. OS process id가 서로 다르다는 사실만으로 iid 세션 가정이 증명되지 않는다. 공유 호스트의 시간 변화나 열적 drift에는 차단·랜덤화·독립 실행 조건과 민감도 검증이 필요하다.

Rust 이식 전 단계에서는 stdlib Python exact Fraction/binomial 계산으로 k와 실패확률을 검산한다. 정렬/median endpoint는 보통의 f64 입력을 쓰므로 이 스크립트는 로그 평가 및 부동소수점 rounding의 interval certificate가 아니다. 정확한 부분은 유한 조합확률과 coverage 논증이며 production 통계 구현의 최종 승인과 실제 속도 주장은 별도다.
