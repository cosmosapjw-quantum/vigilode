# A03 독립 검토 01

이 내용은 `/root/a02_supplement_review`가 작성한 읽기 전용 검토를 controller가 보존했다. 검토자는 packet을 작성하거나 코드를 구현하지 않았고 native 실행·테스트·빌드·benchmark도 수행하지 않았다.

## 판정

**READY_SCOPED — A03-SCALAR-RATE-01 bounded 구현 계약만 승인한다.** 조건부 수학 유도는 명시된 전제 안에서 채택할 수 있다. 전체 damped local certificate, trajectory, production 판정은 **HOLD**이고 scalar adapter의 신규 numerical validation은 **NOT_RUN**이다.

## 입력과 identity

- 관측 checkout: `/home/cosmosapjw/vigilode`, `audit/production-readiness-20261018`, HEAD `7e2873887ed6038b47b667c976435810851c5dba`, tree `9275e35d3d5afe1ae4e40e1c0d9aca1034994ca6`.
- `RESEARCH_DECISION.json`의 입력 21개 SHA-256은 전부 현재 bytes와 일치했다. 그중 19개는 frozen HEAD와 일치한다. `AGENTS.md`와 A01-LCERT 독립검토는 hash-bound working-tree 입력이며 commit 객체가 아니다.
- `DERIVATION_KO.md`: `2fbde354008641a7b50df91bf6755d7648ac2c84ae0d2d21f135e1634a25e8fe`
- `PREREGISTRATION.md`: `8f6a8635f0d9b3366cc457411c0f2dc6b54892e387bfadb505961e462e548f51`
- `RESEARCH_DECISION.json`: `00daf13ce6d45f7aac9d913e1cb170b5b8bc281a2b8ad93f950c9f419d92793c`

## 수학 검토

`a=e^(−κt)b` 변환에서 cubic forcing 계수는 `e^(−2κt)`다. 실제 저장 가중치 합 S를 유지하며 S=1을 가정하지 않는다. A3.12는 basis defect, convolution defect, `g(U)−g(M_Q)`를 모두 포함하고 `M_Q=Σw_jQ_j`와 candidate U를 교환하지 않는다. 저장 coefficient 반올림도 곡선 정의 또는 residual enclosure에 남겨야 한다.

Weighted log norm은 `−κ+S|χ||m|²`로 factor 1이다. Factor 3은 cubic Lipschitz/singular norm의 상계다. A3.16은 `R_Y(s)=e^(−κs)R_scaled(s/h)/h`를 T1 적분에 대입해 얻으며 실제 reconstructed-mean tube와 returned-center displacement가 필요하다.

`Q_{r,d}=I_{d,r}`는 비음수 rate에서 비증가, horizon에서 비감소하므로 `[Q(a_hi,t_lo).lo,Q(a_lo,t_hi).hi]`가 interval 입력을 감싼다. A3.8 positive beta-cell 공식도 유효하다. 별도의 정확 Fraction 확인은 `T=1/2`, cell `[1/8,3/8]`, `r,d=0…4`에서 **25/25, exit 0**이다. 이 유한 계산은 native kernel 실행 근거가 아니다.

κ→0 연속 연결은 맞지만 작은 양의 κ를 exact zero로 치환할 수 없다. Stable confluent representation은 여전히 OPEN이다. Native RHS는 `width*xi`를 사용하며 carrier metadata를 더하지 않는다. 비영 lab phase는 명시된 origin과 검증된 phase enclosure가 없으면 거절한다.

## 다음 구현 단위와 제한

승인 범위는 **A03-SCALAR-RATE-01**이다: exact rate/time interval과 provenance, endpoint hull, caller-owned scratch, prescribed width 실패 후에만 선택하는 positive-cell fallback, 고정 width/work gate, typed refusal, attempted-work/allocation accounting, frozen S01–S10 실행 및 allocator positive control.

고정 한도는 `r,d≤64`, fallback depth 20, leaves 4096, attempted cells 8191, interval primitive attempts `10^7`; oracle degree 128에서 64씩 증가해 최대 2048이다. Width gate는 S01/S02/S03/S04/S07에서 `2^-40`, S05에서 `2^-35`, S10에서 0이다. 양성 대조군이 같은 allocator에서 세 Vec 할당을 검출하기 전에는 zero allocation을 인정하지 않는다.

**Tolerance provenance finding:** preregistration의 `rtol=1e-8`, `atol=1e-11`에는 기존 damped-local 근거가 결속되지 않았다. 이는 사용자가 Robertson/A07에 고정한 값이다. 이후 stable-representation 계약에서 새 local acceptance 기준으로 명시하고 norm과 권한 근거를 결속해야 한다. 이를 기존 damped tolerance로 부르지 않는다. 이 finding은 별도 scalar width gate나 현재 scalar adapter READY 판정을 막지 않는다.

작성자의 Rust/test/build/oracle/benchmark 실행은 **NOT_RUN**이다. 이 검토는 구현이나 numerical admission을 허가하지 않는다. Full A03, positive-kappa local inclusion, damped trajectory, high-order extension, lab phase, speed 및 production promotion은 계속 HOLD다.
