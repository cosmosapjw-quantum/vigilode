# 다음 대화용 고정 인계

$physmath-research-loop로 cosmosapjw-quantum/vigilode 연구를 이어가라.

먼저 research/v1-portable-norm-20261024의 실제 HEAD를 읽고
research/v1_portable_norm_20261024/{REPORT_KO.md,DAG.json,S2_DELIVERY.json,FINAL_DELIVERY.json,HANDOFF_KO.md}를 확인하라.
S2 구현 게시 pin은 909ed1577e49903443c4d6d94fb89bd6d0a01ea1이다.
기준 followup/production-frontier-20261023는 1cdf2c3071498888f2d8ff3ddc0629cb3cf443ef였다.
원래 중앙 인계는 rei_bianchi@0f9d9df825be1dac9ca9bdf73cdc2e8908bcba17의
research/orchestration_longterm_20261023/assembly/HANDOFF_vigilode_KO.md다.

V1p는 PR79 fixed compact30→physical33 norm을 다루는 독립 Rust arithmetic kernel이다.
33 u64-word exact signed accumulation과 exponent-separated outward E² bounds를 반환한다.
원래 실행은 Rust12시험, sum343+norm398=741 Fraction대조,128calls의 hot allocation0,
36cells/144000timed kernel calls다. Accept66/Reject326/Indeterminate6이며 관측 false decision0이다.
현재 native RODAS hook, full physical scale builder, 실제 BASS client 완료와 동일시하지 마라.
S2 코드/source identity와 원자료가 복원됐으므로 동일 science 또는 benchmark를 재실행하지 마라.
C99는 point norm이고 Rust는 squared interval이므로 kernel latency의 일반적 우위를 선언하지 마라.

다음 첫 확인은 V1 owner/source/현재 미커밋 변경과 기존 native 결과를 대조하는 일이다.
이미 구현된 hook이면 재사용하고 중복 구현하지 마라. 담당/소스가 정리됐을 때만 누락된 연결을 진행하라.
Indeterminate를 native driver가 어떻게 처리하는지, full33 scales와 denominator,
기본 WRMS 비회귀, stage/time commit 불변식, FP runtime contract를 별도로 닫아라.
현재 코드에는 FTZ/DAZ runtime guard가 없으며 IEEE/no-fast-math 전제만 있다.
추가 자동 guard 또는 fallback은 새로운 검증 단위로 기록하라.
V3 harmonic subnormal은 기존 PASS 재사용이고 V3b/client·V3c/cost를 대신 닫지 않는다.

기존 source bytes, 실패 로그, closed BUDGET를 보존하라.
원래 V1p validation10.602734846s + 이번 saved readback0.382237736s를 남겼고,
이전 PR80 7.354545033s와 더 오래된 campaign 잔여 미상도 초기화하지 않았다.
독립 인원/서브에이전트 review는 NOT_RUN이다. 같은 작성자의 CAS/Fraction 검산을 그 대신이라고 하지 마라.
사용자 local dirty tree와 NCP 결과를 보존하고 NCP/기존 캠페인을 재제출하지 마라.
각 새 노드 전에 AGENTS/실제HEAD/병렬 변경/백업을 확인하고 expected-SHA로 게시하라.
전체 project completion percent는 null; 한정 V1p 노드와 production admission을 구분하라.
