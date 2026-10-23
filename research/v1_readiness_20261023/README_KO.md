# V1 native affine 인수 준비와 V3 harmonic 갱신

고정 인계: rei_bianchi@0f9d9df825be1dac9ca9bdf73cdc2e8908bcba17의 `research/orchestration_longterm_20261023/assembly/HANDOFF_vigilode_KO.md`.

## 현재 상태와 선택

첫 확인 대상은 V1-affine-native다. PR79@69e83044aacfb9a8fd2c6c3fb1146911beeedd79는 C99 full33 client norm이다. DOPRI AffineErrorNorm과 native RODAS 물리 norm 연결은 별개다. 검사한 공개 근거로 최신 native hook과 실제 nonlinear BASS client의 완료를 확정하지 못했다. 로컬 PC의 현재 예약/미커밋 상태는 Desktop Commander 월간 사용한도로 조회가 막혔고, 재연결·우회·수정을 하지 않았다. 이는 native 구현 부재의 증명이 아니다.

그 때문에 선택한 다음 준비 노드는 **V1a-native-geometry-readiness**다. 생산 hook을 중복 구현하지 않고 원본 C99에 연결된 5개 집중 대조, exact Fraction oracle, 별도의 Decimal 계산 및 native receipt 필수조건 검사기를 만들었다. Scope는 준비 도구이며 native qualification이 아니다.

**V3 harmonic subnormal 회귀는 기존 공개 실행에서 통과했다.** `evidence/REUSED_native_integrator_tests_final.log`의 해당 target은 8 PASS이고 `harmonic_subnormal_forcing_is_not_halved_before_amplification`도 PASS다. 이 로그는 복사한 기존 근거이지 이번 재실행이 아니다. V3b 실제 client 및 V3c matched-cost는 이 회귀로 완료하지 않는다.

현재 공개 frontier HEAD는 cff15d6da06effe494664f1ef50e20d47d3ad619다. Local55ad7ec6는 public df5c6c56da0eb93b58147d6630a0f094cf4af212와 tree5430c642253cfc9f4b05f9531a6dccafce8eb6c8로 연결된다. PUBLISH_MAP의 이 구분을 유지한다. GitHub API compare(df5,cff)는 native crate 변경을 나열하지 않았다. 연구 자료와 별도 PDE 변경은 있다. 실행 commit과 게시 commit을 바꾸어 부르지 않는다.

새 연구 경로의 조상 AGENTS.md와 research/AGENTS.md는 cff pin에서404였다. 다른 branch/로컬 트리까지 AGENTS 부재를 주장하지 않는다. PR79와 frontier는 게시 직전에 다시 읽었고 위 SHA를 유지했다.

## 계산 결과

고정 affine lift y=Lz+b와 양의 물리 scale에서 E^2=e^T[L^T diag(s^-2)L]e/33이다. Offset b는 상태와 scale에는 들어가지만 error에는 더하지 않는다. 물리 성분별 비율은 무차원이다. Gram matrix는 증명용이지 runtime에 조립하라는 요구가 아니다.

- PR79 ordering의 두 관련 성분 error가 +2.5,+2.5이면 E^2=25/22>1이다. +2.5,-2.5이면25/66<1이다. 같은 절댓값이어도 상관항 때문에 판정이 다르다. 전자의 Gram off-diagonal을 버리면25/33<1로 잘못 수락한다.
- 직접 복사되는 error 하나만5.5라면 full33 E^2=121/132<1이지만 분모30은121/120>1로 잘못 거절한다.
- 두 error가1e308,1e308이고 모든 scale이1e308이면 실제 dependent error는-2e308이지만 E^2=2/11로 유한하다. Full lifted error를 binary64로 먼저 materialize하는 방식은 이 지원 범위를 잃을 수 있다. 이는 이번 Rust 실행에서 결함을 재현했다는 뜻이 아니라 native 이식에 전달할 지원 계약이다.

PR79 C/H의 Git blob을 확인하고 원본 그대로 컴파일한 5개 대조는 모두 성공했다. 관측 norm은 순서대로1.0660035817780522,0.6154574548966637,0.9574271077563381,0.4264014327112209,0.24618298195866548이다. 별도 800자리 Decimal direct-block 계산이 Fraction 및 C99 결과와 일치했다. 기존 10 Decimal+5 negative suite, short-IVP13, NCP 캠페인을 재실행하지 않았다.

준비 도구의 12개 시험은 GREEN이다. 최초 미구현 scaffold의 오류5/실패7, 후속 assertion RED12실패도 백업에 보존했다. 이것은 기존 solver의 버그12개라는 뜻이 아니다. Synthetic positive receipt는 검증기 시험용이며 실제 native 증거가 아니다. `native_gate`는 필요한 schema 필드만 검사하고 supplied evidence의 진위를 인증하지 않는다.

## 검토와 claim gate

Fraction sparse-loop와 Decimal direct-block은 독립 계산 경로지만 같은 작성자다. **독립 인원 review는 NOT_RUN**이며 그 요구를 완료했다고 하지 않는다. Native RODAS, 실제 BASS client, defaultWRMS digest, zero-allocation, 성능 승격은 모두 열린 gate다. 이번 게시물은 소유자 검토를 위한 draft preparation evidence일 뿐이다.

STATE.json은 중앙 immutable 인계를 변경하지 않는 additive DAG overlay다. V1은 native 완료 근거와 현재 소유자를 먼저 읽는 것이 다음 단계다. 이미 native 결과가 있으면 재구현/재실행하지 않고 재사용한다. 없고 owner scope가 해제된 경우에만 V1a 구현, 그 후 별도 V1b 실제 client로 진행한다.

## 예산·보존·재현

새 검증 subprocess 누적은7.354545초, 첫 RED 시작부터 마지막 검토 종료까지116.436835초로120초 안에서 끝났다. 집필·원격조회·게시 전체 시간이 이 값이라는 뜻은 아니다. 이전 캠페인 사용량/잔여는 미상으로 보존하며0으로 초기화하지 않았다. 새 native RODAS/IVP/NCP/원격 캠페인 실행은 각각0이다. 기존 NCP96설정/384실행 및 dense72/288은 건드리지 않았다.

전체 상세 보고서, 사전 계획, R1–R10/C1–C10 단계별 상태, exact 결과, 모든 실패·검산 로그, 소스 SHA와 BUDGET 원문은 BACKUP_ACK.json의23파일 ZIP에 있다. 저장 성공과 복원 검증을 구분하며 공유 권한을 확대하지 않았다.

재현은 Python 표준 라이브러리와 C99 compiler를 사용한다. 과거 BUDGET를 지우거나 자동으로 갱신하지 말고 새로 허가된 준비 단위의 별도 복사본에서만 실행한다:

```
python -m unittest test_readiness
python compare_c.py
python review_contract.py
```

원본 출력과 타이밍은 새 실행으로 덮어쓰지 않는다. 이 명령은 준비 계약만 검사하며 native RODAS qualification을 수행하지 않는다. 생산 crate/기존 owner branch/로컬 미커밋/기본 branch는 변경하지 않았고 merge도 하지 않았다.
