# D3에 필요한 기존 source 대응

원본 D2 6파일과 native 관련 5파일을 source/에 보존했다. BYTE_CHECK.json 7/7, ADDITIONAL_BYTE_CHECK.json 4/4로 실제 Git blob 일치를 확인했다. 이 파일들은 읽기 전용 source snapshot이며 solver/build/test 실행은 없다.

## D2의 실제 재사용 경로

- stage_safe.rs blob62b32b9e65116b8f5d9d559641299cf3bfb40282,2957bytes.
  attempt(z,h,alpha,guard,rhs,visit) → Trial {candidate,error,work}.
  analytic frozen diagonal w[j]=1/(h*GAMMA)+rate(j)+2alpha*z[j].
  8stage 각각 lift/domain을 RHS보다 먼저 검사한다. CallbackFault recoverable/fatal과 partial RHS/solve/checked work를 반환한다.
  내부 stage 배열은 반환하지 않는다. D3 cubic/quintic endpoint-Hermite는 기존 Trial candidate를 소비할 수 있다.
- trajectory.rs blob768d751d95c30f47442c328fa1c97c3a3bbd4ce3:
  rate(j)=[1,20,300][j%3], initial(), retained(j)와 분석해 exact().
  exact()는 audit-only로 유지한다.
- admission.rs blob0b1d4fd7be28e2d2f8b61a03ef6f5faf7dcebd0d:
  기존 scale/Session/lift/FP환경 계약. MXCSR unsafe는 연구용 sibling 경계이며 production 복사 금지.
- domain.rs blob4aab0cc6c921ad9eebb469574c851c5fdc77111f:
  exact nonnegative compact sum과 represented full33 sign을 검사한다.
- integer_gate.rs blob3cba0e3c8bff1ce104379e69a1233807ccc0af99:
  기존 full33 exact admission을 변경하지 않는다.
- table.rs blobc66e27543fc1d30ab869638b8cb4b9c23e4c6079:
  기존 RODAS tableau 그대로.

## Native dense machinery와 의미

실제 path는 crates/rodas5p-integrators/src/dense_output_v2.rs,
blob9ac5e6984fae836408a496f43aac18e6e023254e,134460bytes.
핵심 구간115–225,300–520,667–862를 읽었다.

- Rodas5pQuarticSegment::from_step은 StepResult의 endpoints와8stages로 native quartic curve를 고정하고 입력 bits identity를 보관한다.
- represented time interval t_new-t_old가 정확한 normal binary64인 경우만 수용한다. ODE candidate step.h와 represented curve clock은 구분한다.
- scalar_quartic_coefficients/power_coefficients가 interval coefficient algebra를 제공한다.
- ContinuousResidualProvider는 enclose_rhs와 one_sided_log_norm_upper를 제공하고, 모델 C1/identity mass/outward bound 진실성은 provider 의무다.
- verify_continuous_residual_tube는 whole-segment curve/derivative/RHS box에서 residual L2 bound와 lognorm을 계산한다.
- endpoint radius = exp(mu h)*r_in + Phi(mu,h)*defect.
  uniform radius=max(r_in,endpoint radius)를 고정 tube와 비교한다.
- 이 low-level helper는 한 segment 인증이다. 전체 trajectory inheritance는 generic_verified.rs가 이미 구현한다.

## 이미 있는 전체 trajectory transaction

crates/rodas5p-integrators/src/generic_verified.rs,
blob4d8e906f687dae338f3cec031e59e458d1b417b9,17676bytes.
핵심1–165 및278–485를 읽었다.

integrate_verified_residual:
- immutable crate-owned VerifiedModelProvider와 opaque VerifiedContinuousProblem으로 model binding.
- inherited radius와 absolute L2 budget을 사용.
- 모든 requested output를 interval/query rounding 반경과 함께 준비한 뒤 state/radius/cursor/history를 commit.
- failed candidate와verification work를 누적; maxattempts/minstep 경로.
- 현재 공개 factory는 BoundCollectiveProblem과 Pr101Model.
- D3 고정30/33 Riccati 모델과 cubic/quintic represented curves를 이 existing native capability의 완료로 간주하지 않는다.
- D3 별도 제조 consumer의 의미론에 기존 uncertainty inheritance/prepare-before-commit 원리를 적용한다.

## 관련 packet 원리

packet_curve.rs blob3851555523719ef70f1455254b21b49af86f65ba
packet_curve_output.rs blobcbda0350c72bc2d025a94e5def1cbe6fc426e9c2
packet_verified_trajectory.rs blob627b34281a95dd5ceda456d916de2124f17b2152

source snapshots와 API/관련 원리 위치를 확인했다. RepresentedPacketCurve::with_bernstein_residual_bound,
PacketFrozenOutput의 weighted-radius transport/query displacement, integrate_verified_packet/advance_verified_packet의 commit 전 output 검증이 있다.
이 source는 collective complex packet 모델 전용이며 D3 Riccati 의미로 그대로 재명명하지 않는다.
새 Bernstein residual 또는 inherited radius 원리를 발명했다고 주장하지 않는다.

## 예약 관측

D3 source HEAD는 마지막 실제 readback에서 d702012… 그대로다.
공개 CONTRACT는 S0 scope와 native owner 미확정만 기록한다.
새 S1 이후 게시/진행 lease/예약 해제 receipt는 이번 조회에서 확보되지 않았다.
이 상태는 다른 스레드의 현재 진행이나 종료를 확정하지 않는다.

