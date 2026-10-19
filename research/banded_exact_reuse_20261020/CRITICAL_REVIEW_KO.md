# 개선 결과의 비판적 해석

기준일:2026-10-20KST. 수치 코드는 b51c051, 성공한 공통 benchmark 어댑터·실행 소스는 f8e89b7이며, 결과는 run39304607635에 있다. 새 source와 원자료를 실제로 push했고 default kernel은 바꾸지 않았다.

## 결함 수정

단독 banded solve에 nonfinite RHS나 상삼각 원소를 주면 Ok(NaN), 표현 불가능한 큰 해에는 Ok(Inf), 극단적인 크기에는 overflow panic이 생기는 네 회귀 사례가 원본에서 재현됐다. 입력·출력 유한성 및 checked size arithmetic으로 수정했고, 작은 pivot에서 표현 가능한 해는 여전히 허용한다. 이는 mathematical singularity와 floating-point representation failure를 구분하는 수정이며 scaled solve를 새로 구현한 것은 아니다.

처음 의심한 callback 실패 후 snapshot 재사용은 반증됐다. 원본의4개 retry검사가 통과했고 완전한 Err 분기에서 fresh_state를 무효화한다. 따라서 그 코드는 수정하지 않았다. 이 반증과 실제 수정 대상을 섞지 않는다.

## 수치 동일성과 비용

432개 실행 행에서324개의 parent/new 비교가 모두 같은 상태·출력시각 비트를 보였다. rtol은1e-4/1e-6/1e-8, h0배율은0.5/1/2, 공간격자는50/127/200cells다. LU cache를 제외한 작업 카운터도 일치한다. 시간은 별도로 고정rtol1e-6에서12paired블록으로 측정했다. 모든 개별 반복 시간과 상태 hash 연결이 남아 있다.

N=400의 대표 값(ms/적분)은 다음과 같다.

|문제·출력|기존 Slices|패치 후 Slices|FixedWidth2|Exact cache|
|---|---:|---:|---:|---:|
|Brusselator endpoint|7.851086|7.718258|7.485703|7.6511665|
|Brusselator uniform101|11.8616235|11.7034365|11.302102|11.5675985|
|선형 diffusion endpoint|0.84341525|0.82893825|0.80473075|0.82548675|
|선형 diffusion dyadic129|11.34740675|11.2135955|10.8461625|10.16744325|

비선형 사례의 FixedWidth2 시간 감소는 기존 대비 대략4.7~5.3%다. 그러나 패치 후 기존 Slices 경로도 기존 binary보다 소폭 짧았으므로 전체 약5%를 상수폭 특화 하나의 효과로 귀속하면 안 된다. 같은 새 binary에서 Slices와 FixedWidth2를 비교하면 추가 시간 감소는 대략3%다. 코드배치·inlining 변화의 개별 기여를 분리한 profiler실험은 하지 않았다.

## 정확 cache의 양성 결과와 부정 결과

선형 dyadic129에서는 모든 크기에서 LU134회가8회로 줄었다.126회 재사용이며 Jacobian134회는 유지된다. 전체 시간 감소는 기존 대비 약10~11%로, LU횟수94%감소와 같은 비율이 아니다. 다른 stage연산/RHS/출력 처리 비용이 남는다.

반대로 비선형 Brusselator에서는 cache재사용이0회다. FixedWidth2만 쓸 때보다 exact-cache가 약2~3%느리다. 따라서 이 사례에서 cache를 무조건 켜는 정책은 지지되지 않는다. 선형 endpoint도 step이 계속 바뀌어 재사용이0회이므로, '선형 또는 constant-J이면 항상 이득'이라는 주장 역시 틀리다. 필요한 조건은 실제 shift까지 반복되는 것이다.

양성대조군의 해석에는 추가 주의가 필요하다. dyadic129는 출력 시각에 step을 맞추게 해서 반복 행렬을 많이 만드는 의도된 stress case다. 여기서 경험적 E는약1.6e-13으로 입력rtol1e-6보다 훨씬 작다. endpoint는10attempts인데 dyadic129는134attempts다. 이 차이는 출력정책이 지배하는 과잉 적분을 보여 주며, 이를 일반적인 비선형 cosmology계산의 speedup으로 일반화하지 않는다. 정확하고 저렴한 dense output은 여전히 독립적인 중요 과제다.

Cache key는 n*(l+u+1)개의u64를 추가한다. bandwidth2,N=400이면16000bytes다. 이는 bandmatrix/factor/key의8byte-slot 장부이며 전체 peak RSS나 allocator측정치가 아니다. No-hit 경로에서도 이 저장·복사·검사 비용은 발생한다.

## 이론상 보존되는 것

W=I/(gamma*h)-J이므로 J가 같더라도 h가 바뀌면 W가 달라진다. 이번 cache는 실제 계산한 rounded shift와 J전항목의 bit identity, successful finite LU라는 충분조건만 사용한다. 같은 factor/pivot배열로 같은 triangular solve를 하므로 해당 결정론적 환경에서 stage와 embedded error를 바꾸지 않는다. 근사 J를 고정하는 Rosenbrock/W방법은 차수·안정성 조건 검증이 따로 필요하며 이번 패치에 포함되지 않는다.

이론문서와 symbolic 반례는 THEORY.md에 있고, 실제 factor/pivot/solve-bit 검사는 internal test에 있다. 동등성은 측정된 환경/입력 범위 및 명시한 결정론적 가정에 한정되며 arbitrary compiler fast-math나 외부callback의 FPU상태변경까지 보장하지 않는다.

## 검증 한계와 다음 판단

회귀9개, 관련 내부17개가 통과했다. 전체workspace는660초 timeout으로 끝났고, 그 실패를 PASS로 바꾸지 않았다. 성공 측정 재시도는 numerical crates가 동일함을 확인한 뒤 그 timeout증거를 재사용했으며 전체suite를 또 완료했다고 주장하지 않는다.

첫 benchmark어댑터는 필요한 JVP등록을 빼먹어 모든 호출이 적분 전에 거부됐다. 로그의 BLOCKED_NUMERICAL_PARITY는 원자료를 검토해 ADAPTER_CONSTRUCTION_FAILURE로 정확히 분류했고, 스칼라오차나 tolerance를 바꾸지 않고 어댑터만 수정했다. 이후에는 smoke로먼저검사한다.

Shared-runner 시간구간은36개비교에 Bonferroni예산을 배분했지만12블록의bootstrapcoverage·호스트자기상관이엄밀히인증되지는않는다. 약5%의 작지만 일관된 변화는 이호스트의 관측이다. Fullworkspace완료와다중호스트검증전에는기본커널을승격하지않는다.

결론: verifiedcorrectnessfix와opt-in구현개선은완료했지만, 기존DVODE공간격차해소·approximateJacobianreuse·전역연속오차·Diffsolsparsebackend·실제물리client검증은미해결이다. 이번에그범위를완료했다고주장하지않는다.
