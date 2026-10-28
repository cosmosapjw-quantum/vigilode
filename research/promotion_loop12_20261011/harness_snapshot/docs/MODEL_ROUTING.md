# 모델별 연구 루프 선택 규칙

라우터 계약 버전은 `1`이다. 사용자 표현 `연구 루프`, `수학·물리 연구루프`, `코딩 연구루프`가 이 규칙의 호출 계기가 된다. 라우터는 실행할 하네스 파일을 선택하며 실제 모델을 변경하거나 식별·검증하지 않는다.

## 선택 순서

1. 사용자가 특정 하네스 버전을 명시하면 그 선택을 우선한다. GPT-6 Astra에서 실행하더라도 사용자가 GPT-5.6 하네스를 지정했다면 기존 하네스를 보존한다.
2. 별도 버전 지정이 없으면 **해당 작업을 실제로 수행할 모델**의 호스트 제공 메타데이터를 사용한다. 라우터를 호출하는 쪽이 이 출처를 확인·기록해야 한다.
3. 실제 모델 메타데이터가 없으면 사용자가 현재 채택한다고 명시한 모델명을 사용할 수 있다. 이때 출처는 사용자 선언이며 관측된 모델 신원이 아니다.
4. 어느 쪽도 확인할 수 없거나 별칭이 미지원이면 `MODEL_UNRESOLVED`로 둔다. 파일명·환경변수·문체·작업 내용으로 모델을 추측하지 않는다. 모델 선택이 작업을 실제로 갈라놓을 때만 한 가지 짧은 질문으로 확인하고, 그동안 가능한 모델 독립 작업은 진행한다.

| 모델 라벨 | 하네스 계열 |
| --- | --- |
| `GPT-5.6`, `5.6`, `gpt56`, `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`, 각각 지원된 `-wm` 별칭 | `gpt56` — v3.1.0 |
| `GPT-6 Astra`, `6 astra`, `gpt6-astra`, `gpt-6-astra`, `gpt-6-astra-wm` | `gpt6-astra` — v4.0.0 |
| `gpt-6`, `6`, 미래 모델명, 부분적으로만 일치하는 이름 | 미해결 |

정확한 별칭 목록은 `MODEL_ROUTER.json`에 있다. 대소문자와 문자열 앞뒤 공백만 정규화한다. 임의 접미사·부분문자열 일치·미래 모델 추정은 하지 않는다. 위 라벨들은 이 하네스의 선택 별칭이며 공개 API 모델 ID 또는 API 가용성 주장에 해당하지 않는다.

## 작업 선택과 단계 전환

- `research`가 기본값이다. 연구 하네스만 선택한다.
- `coding`은 코딩 하네스만 선택한다.
- `both`는 연구, 코딩 순서로 두 파일을 제시한다. 두 하네스를 무조건 동시에 실행하는 명령이 아니다. 연구에서 코딩으로 넘어갈 때는 연구 결과와 scientific contract, 해당 과제의 승인 범위 및 전환 조건을 전달한다.
- 연구와 코딩을 서로 다른 모델이 실행하면 각 작업 직전에 그 실행 모델을 기준으로 다시 선택한다. 부모 모델을 자식 실행 모델로 간주하지 않는다.
- 모델이나 버전을 바꿔도 현재 상태, 정의·컨벤션, 고정 입력, 완료된 work unit, 최초 실패 증거와 claim ceiling을 보존한다. 모델 교체 자체를 이유로 완료 계산·검증을 반복하지 않는다.

```bash
python tools/resolve_model.py --model 'GPT-6 Astra' --task research
python tools/resolve_model.py --model 'gpt-5.6-sol' --task coding
python tools/resolve_model.py --model 'gpt-6-astra' --harness gpt56 --task both
python -m unittest discover -s tests -p 'test_model_router.py' -v
```

`--model`은 호출자가 관측하거나 사용자가 채택한 모델 라벨을 전달하는 선택 입력이다. `--harness gpt56|gpt6-astra`는 사용자의 명시적 하네스 선택이며 `--model`보다 우선한다. `--task research|coding|both` 기본값은 `research`다. `--registry` 기본값은 이 스크립트가 속한 패키지 루트의 `MODEL_ROUTER.json`이며 현재 작업 디렉터리에 의존하지 않는다. 라우터는 환경변수를 읽어 모델을 탐지하지 않는다.

선택 성공은 JSON `status=SELECTED`, exit `0`이다. `selection_basis`는 `caller_supplied_model_label` 또는 `explicit_user_harness_override`로 기록된다. 메타데이터인지 사용자 선언인지는 호출 측 기록으로 보존해야 한다. 모델 미해결은 JSON `status=MODEL_UNRESOLVED`, 빈 `packages`, exit `2`다. 잘못된 `--harness`나 `--task`는 argparse 사용법 오류, exit `2`다. 누락·손상·모순된 레지스트리는 JSON `status=CONFIG_ERROR`, 빈 `packages`, exit `3`이다.

## 파일 선택과 로딩의 구분

레지스트리는 파일명, 경로, 알려진 Library ID와 SHA-256을 제공한다. GPT-5.6 ID는 보존된 원본을 가리킨다. 새 GPT-6 Astra 패키지의 ID가 `null`이면 고유 파일명과 `/Research-Harnesses/` 경로로 조회한다. 자기 자신을 포함하는 ZIP의 SHA-256은 내부 레지스트리에 넣지 않으며 외부 배포 기록에서 확인한다. `null`은 미기록이라는 뜻이고 검증 성공이라는 뜻이 아니다. 파일을 가져온 후 해당 패키지의 진입점을 실제로 읽고 적용해야 로딩되었다고 보고할 수 있다.

이 프로그램의 모든 선택 결과에서 `model_identity_verified`, `model_switched`, `harness_loaded`, `harness_executed`는 `false`다. 이는 라우팅과 로딩·실행을 구분하는 범위 표지다. 하네스 저장만으로 ChatGPT 메모리가 등록되지는 않으며, 자동 선택에는 설치된 라우팅 스킬 또는 프로젝트 지침과 실제 파일 접근이 필요하다. 명시적 기억 저장 여부, 라우터 설치 여부, 파일 로딩 여부와 계산 실행 여부를 각각 별도로 보고한다.
