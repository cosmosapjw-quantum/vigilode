# V1p portable physical-norm 연구: S2 복원과 최종 인계

기준일: 2026-10-24 KST. 이번 재개는 중단 전에 끝난 V1p 구현·시험·kernel-cost 실험을 S2 백업에서 복원하고, 정확한 소스와 결과를 게시하는 S3 작업이다. 새 solver 실행이나 새 성능 표본을 만들지 않았다.

## 결론

PR80의 geometry 준비를 실제 실행 가능한 독립 Rust kernel로 발전시킨 S2가 보존돼 있었다. 고정된 PR79 compact30→full33 lift에서 최대 세 개 binary64 항을 정수로 정확히 합산하고, 지수를 분리한 outward interval로 물리적 WRMS 제곱을 계산한다. 반환 구간이 수락 경계를 가로지르면 `Indeterminate`를 반환한다. 원래 C99의 extended-long-double 지원을 요구하지 않는 Rust 구현이지만, 임의 affine map이나 실제 native RODAS 적분기의 qualification은 아니다.

복원한 원본은 `VIGILODE_V1_PORTABLE_S2_20261024.zip`, 802047 bytes, SHA256 `ffe50c8f246e38fef472eb69dd7528b88d96d31c2a95f2ec1b356349e965a661`이다. ZIP CRC와 manifest의37개 payload 크기·SHA256·Git blob을 확인했다. Rust source/test/probe/bench와 Python oracle은 재작성하지 않고 원본 그대로 게시했다. 실행은 로컬 파일 집합에 대해 먼저 이루어졌으며, 이후 게시 commit을 실행 당시 commit으로 바꾸어 표시하지 않는다.

## 1. 조사와 병렬 연구의 통합

원래 S0 intake는 VigilODE branch 이름196개를100/96/0 세 페이지로 확인했다. 모든 이름을 열거했다는 사실은 모든 branch의 source를 심사했다는 뜻이 아니다. 관련 최신 branch·PR·백업과 대화 검색의 제한은 S2 `INTAKE.json`에 보존돼 있다.

기준은 `followup/production-frontier-20261023@1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef`다. 이 후속 branch에는 FA01 scalar fatal, SW02의 제한된 native persistent session, GE01 예산 소비자, QA01 두 상태 collision witness, LP01 선형 위상 표본이 이미 있어 신규 기능으로 중복 집계하지 않는다. 독립 V1 native hook과 실제 BASS client의 현재 담당/완료 상태는 여전히 미확정이다. 로컬 quota blocker를 우회하거나 사용자 checkout을 건드리지 않았다.

이번 재개에서 base와 V1p HEAD를 다시 읽었고, 중단 시점의 값과 같았다. V1p는 S1 HEAD `057fecf0698fa262d881f101625e6b0bb93791cf`에 머물러 있었고, S2 구현은 Drive에만 완료된 상태였다. S2 게시 commit은 `909ed1577e49903443c4d6d94fb89bd6d0a01ea1`이다. 기존 NCP·native ODE·물리 campaign은 재실행하지 않았다. V3 harmonic subnormal의 기존 PASS도 그대로 재사용한다.

## 2. 수학적 대상과 계산형

고정 affine 복원 y=Lz+b, 축약 embedded difference e, caller가 이미 구성한 양의 물리 scale s에 대해

    E² = (1/33) Σ_i ((Le)_i/s_i)².

b는 상태와 scale에는 관계하지만 오차에 더하지 않는다. 종속 physical rows23/25/28은 각각 한 항, 두 항, 세 항의 음의 합이고 나머지는 복사다. 각 비율은 무차원이다. 고정 ±1 coefficient map에 대한 산술 문제이지 임의 chart 또는 실제 global ODE error의 상계는 아니다.

모든 유한 binary64는 정수×2^-1074다. 최대 세 항의 절댓값 합은 이 정수 단위에서2^2100보다 작으므로33개의 u64 word(2112 bits)로 담긴다. 양·음 항을 별도로 누적하고 정확히 빼서 상쇄를 보존한다. 원래 큰 lifted 값이 f64 범위를 넘어도, leading mantissa와 별도 exponent로 정규화할 수 있다.

커널은 `[lo,hi] × 2^exponent` 형태의 E² 구간을 반환한다.

- upper≤1이면 Accept.
- lower>1이면 Reject.
- 그 외는 Indeterminate. 수락 상태를 commit할 근거가 아니다.

조건부 산술 증명은 기존 `THEORY.md`에 있다. IEEE binary64의 correctly-rounded 기본연산, gradual underflow, no-fast-math/FTZ/DAZ가 전제다. **현재 kernel은 FP mode를 runtime에 검사하지 않는다.** 특히 이 전제를 이미 검증된 자동 guard라고 읽으면 안 된다. 프리미티브와 컴파일러 전체의 형식검증도 수행하지 않았다.

## 3. 완료된 실행 근거와 이번 재개 검산

중단 전 Rust1.94.1로 시행한12개 시험이 모두 통과했다. 상관항, full33 분모, raw overflow/finite normalized norm, 최대값 세 항, 상쇄 순서, minsubnormal 잔여, 극단 scale, invalid input, 경계 보류, dyadic rescaling, heap 계수를 포함한다. 최초 미구현 scaffold RED와 환경 추출45초 timeout은 원본 증거에서 보존한다. 환경 실패를 과학 계산 실패로 세지 않는다.

Python Fraction oracle은741건이다. 그중343건은 세 항 합,398건은 전체 물리 norm이다. 전체 norm 결과는 Accept66, Reject326, Indeterminate6이다. 이 corpus에서 구간 누락0, false accept0, false reject0, 최대 상대 구간폭 약1.54605e-14였다. 741개 전부를 전체 norm 시험이라고 부르거나 finite corpus를 전체 domain 정리증명으로 확대하지 않는다.

직접 측정한128회 kernel 호출에서 heap allocation 요청0을 확인했다. 파서·테스트 하네스·caller·native solver 전체의 무할당 주장은 아니다.

이번 재개는 컴파일 또는 Rust 시험을 반복하지 않았다. 대신 저장된 input bits와 output interval, exact numerator/denominator를 모두 재계산했고,37개 payload,9개 원래 budget log hash,741개 대조와36개 cost cell·3개 paired median을 확인했다. `READBACK.json`의 결과는 PASS_SAVED_S2_READBACK이다. 다른 계산 경로를 사용한 같은 작성자의 작업이며 독립 인원/agent 심사는 NOT_RUN이다.

## 4. 성능 진단과 한계

세 입력별 kernel latency 중앙값은 다음과 같다. 단위는 microsecond/call이다.

| 입력 | Rust interval-square | C99 point-norm |
|---|---:|---:|
| 보통 scale | 2.7594 | 3.4242 |
| 큰 양·음 항 상쇄 | 1.7436 | 3.0228 |
| raw lift overflow, normalized finite | 2.5560 | 2.0559 |

한 프로세스에서 case3×block6×arm2=36cells, cell당4000회, 총144000timed kernel calls다. 각 cell의100warmups는 별도이며 preflight calls도 timed 총수에 넣지 않는다. Rust 타이머는 `enclose`와 endpoint checksum을 측정하고 decision 자체는 포함하지 않는다. C99는 sqrt를 포함한 점값과 status check를 수행한다. **동일 계약·작업의 성능 비교가 아니다.** 단일 프로세스 표본은 독립 프로세스 confidence interval이 아니며 순위 승격이나 전체 solver 가속을 주장하지 않는다. 상세 범위는 원본 COST_SUMMARY와 KERNEL_COST.log에 있다.

## 5. 진척과 남은 의무

이 한정 단위는 S0 조사/계약 → S1 이론/CAS → S2 Rust/원자료 → S3 복원·게시·인계의4단계다. S2 완료를 native RODAS 전체 완성으로 올리지 않는다. 전체 연구의 검증 가능한 공통 분모가 없으므로 global completion percent는 null이다.

V1p arithmetic kernel은 PASS_SCOPED. V1 native hook, full physical scale builder, default-WRMS 비회귀, 실제 BASS client, Indeterminate의 driver 정책, platform FP-mode guard, 독립 인원 심사와 production은 열려 있다. caller가 범위를 벗어난 FP 환경을 사용할 때도 자동 안전하다는 주장은 없다.

다음 준비 노드는 **V1 owner/source reconcilation 후 native hook integration**이다. owner 소스가 이미 완료됐다면 재구현하지 않고 그 기록에 이 kernel의 지원범위/경계 정책을 대조한다. 실제 client는 별도 V1b로 남긴다. V3 harmonic의 이미 완료한 회귀나 기존 NCP campaign을 다시 제출하지 않는다.

## 6. 예산·배포

원본 V1p BUDGET는 closed이며 compiler/test/oracle subprocess10.602734846초, cap120초다. 이전 PR80의7.354545033초와 미상 campaign 잔여를 지우거나 새 예산으로 초기화하지 않았다. 재개 saved readback0.382237736초를 별도 append-only recovery ledger에 기록했다. 이 단위의 합산 attributable validation은10.984972582초다. 문서 집필·다운로드·게시 전체 시간이 이 수치라는 뜻은 아니다.

Git에는 실행 가능한 소스와 작은 근거·문서가 있으며, 큰 exact 원자료와 모든 실패 로그를 포함한 원본 S2 ZIP 및 최종 S3 ZIP은 Drive/Dropbox에 보관한다. 완전한 원본 payload는 S2 manifest를 기준으로 한다. 새 clone에서 과거 결과를 확인할 때는 컴파일하지 않는 `readback_saved.py`를 우선 사용한다. 새 빌드/실험은 새 directory와 새 budget을 사용하며 옛 결과를 덮어쓰지 않는다.
