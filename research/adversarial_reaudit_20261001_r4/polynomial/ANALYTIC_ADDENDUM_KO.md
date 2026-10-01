# 실행하지 않은 추가 설계: Laguerre scale과 degree의 공동 선택

root가 제안한 exact-arithmetic tail Tₘ(β)=W exp[ρ/(2β)] [hβ/(1+hβ)]^(m+1), W=Σ||wₖ||₂/k!의 scale 최적화는 직접 미분으로 확인된다. h>0, ρ>0, W>0, r=m+1에서

    d log T / dβ = −ρ/(2β²) + r/[β(1+hβ)].

따라서 2r>hρ이면 유일한 최소점 β*=ρ/(2r−hρ)가 있고, s=hρ를 쓰면

    minβ Tₘ(β) = W exp(r−s/2) [s/(2r)]^r.

2r≤hρ이면 T는 감소하며 β→∞의 infimum이 W이므로 이 bound만으로 W보다 작은 목표를 만족할 수 없다. h=0, ρ=0, W=0은 별도 exact branch다. 현재 L=ρ/β≤16 정책을 유지하면 최적의 연속 scale은 L*=min(16,2r−hρ) (양수인 경우)로 제한된다. 이는 현재 {1,2,4,8,16} 탐색에 대한 degree별 연속 최적화 후보이며 새 수치실행을 하지 않았다.

이 식은 tail 상한의 최적점이고 전체 계산오차·실제 runtime의 최적점은 아니다. 특히 L cap을 크게 풀면 exp(L/2)에 따라 basis 크기 및 cancellation 부담이 커질 수 있다. 따라서 cap 제거를 자동 권고하지 않는다. 구현 순서는 기존 cap 안의 β 후보를 비교한 뒤, recurrence roundoff 인증과 coefficient 생성 비용을 포함하여 별도 사전등록 실험을 하는 것이다. 증거 상태는 derived이며 numerically checked 또는 implementation-verified가 아니다.
