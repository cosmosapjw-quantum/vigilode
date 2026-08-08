# 다음 수치실험 cycle — order-aware output-certificate policy

## 목적

Cycle 1에서 탈락한 것은 partial-coupling homotopy 전체가 아니라, 다음 고정 정책이다.

\[
E_{\rm out}\le0.1.
\]

이 absolute normalized output-WRMS threshold는 finite one-step grid에서 false accept가 없었지만,
all-fast low-depth trajectory의 global fifth order를 보존하지 못했다. 다음 cycle은 path kernel을
광범위하게 튜닝하기 전에 **acceptance policy 자체**를 분리 검증한다.

## 비교 정책

동일한 candidate stage vectors와 exact target certificate를 사용해 다음을 비교한다.

1. fixed absolute budget:
   \[
   E_{\rm out}\le\varepsilon_0;
   \]
2. embedded-relative budget:
   \[
   E_{\rm out}\le\eta E_{\rm emb};
   \]
3. step-size power budget:
   \[
   E_{\rm out}\le C h^{p+1};
   \]
4. mixed policy:
   \[
   E_{\rm out}\le\min(\eta E_{\rm emb}, C h^{p+1}).
   \]

Calibration and holdout trajectories must be disjoint. A policy is killed immediately by any false
accept, fifth-order regression before roundoff, stiff-decay regression, or unbounded fallback rate.

## 순서

1. freeze the cycle-1 stage candidates and replay only the policy layer;
2. all-fast global-order screen;
3. Prothero–Robinson stiffness/nonlinearity screen;
4. nonnormal/noncommuting mass screen;
5. compare retained low-depth rows with equal-work sequential and SABR controls;
6. only after these gates, allow adaptive theta/lambda/q schedules;
7. actual concurrent batch backend remains a later, separate performance cycle.

## 승격 기준

- false accepts: 0;
- protected fifth-order trend before roundoff;
- no stiff-decay regression relative to protected sequential RODAS5P;
- low-depth accepted regime remains nonempty on holdout;
- median fallback-inclusive scalar work does not exceed the protected control without a separately
  measured concurrency benefit.
