# 로컬 성능 병목 연구 준비 결과

24개 실제 IVP 실행기와 소스 고정형 로컬 캠페인을 연결했다. 현재 준비는 로컬에서 컴파일하고 실행 결과를 해석할 수 있는 단계이며, 사용자의 Ryzen 머신에서 광범위한 캠페인을 완료한 상태는 아니다. NCP는 사용하지 않았다.

이번 실행기 검증은 Python 제어기·외부 계약 테스트 42개(기본 제어기/외부 계약 30개, 조건부 wrapper 12개), 실제 실행기 경유 smoke 18개(최종 바이너리 재검증 9개 포함), 현재 모델과 RHS probe가 일치하는 외부 비교 smoke 20개다. 마지막 20개는 1/10/15/16번 × DOPRI5/RODAS5P-mf/switching/VODE-Adams/VODE-BDF이다. 15개는 해석해 endpoint 조건을 통과했고, Robertson 5개는 독립 참조가 없어 종료 성공만 기록했다. 모두 수정 중인 작업공간의 탐색적 실행으로 성능 순위와 production 승격에서 제외한다. 저장된 각 영수증에 당시 바이너리 SHA256이 있으므로 이후 최종 재빌드 결과와 동일하다고 추정하지 않는다.

계획상 물리 코어 12개가 허용된 호스트에서 기본 standard 구성은 120개 문제/스레드 설정에 방법·반복·별도 serial 기준·프로파일링을 곱하여 **1539개 프로세스**를 요청한다. Dense 차원 제한으로 15개 문제/방법 조합을 제외한 수다. 모두 60초 timeout을 소모하면 92340초이므로 7200초 accounted budget 안에 전부 완료한다는 보장은 없다. 실제 허용 코어가 적으면 설정 수가 달라지고 실행 계획에 명시된다. 이 카디널리티 계산은 12코어 실행 결과가 아니다.

실제 정확도가 없는 모델은 프로파일링 대상으로 유지하지만 시간 우위의 증거에서는 빠진다. 계측이 없는 선형계 분해/기저 투영/할당 비율을 추정으로 채우지 않는다. callback 시간은 별도 계측 실행에만 존재하며 ensemble 합산 시간은 겹칠 수 있다. 메모리 peak는 할당 횟수를 뜻하지 않고, process wall에서 solve 시간을 뺀 나머지는 I/O만의 시간이 아니다.

외부 비교는 현재 4모델에 한정된 실제 SciPy VODE adapter를 제공한다. Julia adapter는 설치된 고정 프로젝트에서 실행하도록 준비했지만 이 환경에서는 실행하지 않았다. diffsol은 이전 fixture와 현재 IVP가 다르므로 현재 비교가 구현되지 않았음을 `NOT_RUN`으로 기록한다. 이전 결과를 현재 문제의 비교 성능으로 가져오지 않는다.

다음 로컬 단계는 고정 commit/tree의 별도 clean checkout에서 build 영수증을 만들고 smoke부터 실행하는 것이다. 소스, binary, host/affinity 또는 plan이 다르면 기존 결과 디렉터리를 재사용하지 않는다. 실패와 timeout은 삭제하지 않으며 정확도 적격 반복이 모두 모인 그룹만 표에 median/range를 표시한다. 시간 오차의 별도 probe 캠페인과 stage 연구는 서로 다른 출력 계약과 예산을 가지므로 한 latency 표에 합치지 않는다.

구체적인 실행 지침은 `../local/README_KO.md`, 복사 가능한 작업 지시는 `../local/HANDOFF_TEMPLATE_KO.md`, 후속 작업과 범위는 `LOCAL_CAMPAIGN.json`에 있다. 장기적으로 필요한 순서는 (1) 실제 로컬 전체 실행 및 병목 분류, (2) 미검증 17슬롯의 독립 참조 구축, (3) diffsol 현재모델 adapter, (4) Julia 계약/버전 검증, (5) 측정상 비용이 큰 내부 영역만 추가 계측하는 것이다.

조건부 chart/continuation/structured residual/invariant module의 별도 캠페인과 12개 wrapper 검증은 `../local/CONDITIONAL_CAMPAIGN_KO.md`에 통합되어 있다. 이 후보는 속도만으로 배제하지 않고 원래 좌표의 정확도·잔차 보증·거절/복구와 강건성의 이점을 함께 기록한다.
