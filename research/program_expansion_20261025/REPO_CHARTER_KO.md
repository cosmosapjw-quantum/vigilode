# VigilODE 확장 역할과 연구 범위

VigilODE는 여러 핵종·이온, 복사와 baryon source를 결합한 실제 물리 모델을 계산할 수 있는 수치 engine을 담당한다. 원자·핵 반응 자료, radiation field와 물리 closure의 과학적 소유권은 물리 저장소에 남는다.

## 담당 범위

- 실제 full-state/reduced-state 오류의 일관된 adaptive control, stiffness 대응과 명시적 derivative policy.
- RHS 호출 전 domain 검사, 안전한 stage·endpoint·dense output, accepted-state transaction과 작업량 보존.
- 물리 모델의 analytic Jacobian/JVP와 선택된 whole-domain provider를 native 수치 경로에 연결.
- 필요한 과학 재시작, 사건·source epoch 변화, accepted-only spectral remap의 수치 인터페이스.
- 같은 물리 모델·오차·출력을 기준으로 한 비용 측정과 한정 최적화.

BASS/REI는 물리 RHS, 종별 에너지·입자 원장, frame·clock과 source ownership을 제공한다. REC는 광자 history/field와 line/continuum source를 제공한다. WU/bass_cr/HE는 반응·교환·stress 자료와 유효 domain을 제공한다. VigilODE는 유한 callback 반환만으로 derivative correctness나 물리적 보존을 승인하지 않는다.

## 현재 근거와 다음 과제

V1p/q/r 및 D1–D5는 각 연구 범위에서 완료됐다. 최신 D5는 full-write RHS/Jacobian 경계를 제공하며 실제 finite J+I 오류를 derivative oracle에서 구별한다. Native frontier에는 AffineErrorNorm, Transaction, provider/output hook과 구조적 opt-in 알고리즘이 이미 있다. 이들을 새로 복제하지 않는다.

다음 핵심 과제 V6는 하나의 실제 BASS/REI 모델의 RHS, Jacobian/JVP, full33/reduced30 관계와 domain을 기존 engine에 연결하는 것이다. 현재 native 담당 source·예약·live dirty·완료 receipt가 없으므로 그 branch의 구현을 인수하지 않는다. 공개 source에서 준비할 수 있는 실제 모델 수학 계약은 계속 진행한다.

V2의 persistent native session, V7의 필요한 완전 재시작, V8의 모델별 whole-domain provider는 별도 과제다. 기존 harmonic regression과 완료된 benchmark/NCP campaign은 재실행하지 않는다. V5의 일반 production 및 전체 프로젝트 완료율은 각각 HOLD와 null을 유지한다.

## 작업 경계

이 PR은 역할·배치·인계 문서만 추가한다. 생산 crate의 forbid(unsafe_code), 원래 tolerance와 기본 behavior를 유지한다. D5의 과학적 PASS와 metadata CI의 authority failure는 구분한다. 새 heavy 실행·NCP·예산 생성·atomic table 소유권 이전은 포함하지 않는다.

향후 구현은 exact source/input, 허용 경로, 원래 acceptance, 양성·음성 control, 정확한 command와 누적 budget을 고정해 lower-tier 구현자에게 전달한다. 별도 과학 검토자가 diff와 raw outcome을 읽고 범위 내 admission을 결정한다.
