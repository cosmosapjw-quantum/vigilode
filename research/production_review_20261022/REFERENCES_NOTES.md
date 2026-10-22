# 원전과 직접 유도에 대한 독서 기록

확인 일자: 2026-10-22. 검색 색인은 발견 수단으로 사용했다. 수학적 주장에 대한 인용은 아래의 출판사 페이지 또는 저자 원고로 제한했다. 원문 전체를 새 보고서에 복제하지 않는다.

## Steinebach (2023)

**Source**: Steinebach, G. (2023). Construction of Rosenbrock–Wanner method Rodas5P and numerical benchmarks within the Julia Differential Equations package. *BIT Numerical Mathematics*, 63, article 27. DOI: [10.1007/s10543-023-00967-x](https://doi.org/10.1007/s10543-023-00967-x).

Primary page: https://link.springer.com/article/10.1007/s10543-023-00967-x

확인 범위: 출판사 검색 결과의 원문 추출은 초록, ROW 공식 (1.2)–(1.3), 차수 저하 논의, 계수 구성 동기를 포함한다. 논문은 Rodas5P의 고전적 5(4)차와 stiff problem에서의 개선을 다룬다. 논문의 벤치마크를 현재 VigilODE 구현의 벤치마크로 전용하지 않는다. 실제 코드의 계수·stage·dense output 및 Jacobian 처리의 일치 여부는 별도 소스 검사 대상이다.

## Southworth, Sivas and Rhebergen (2020)

**Source**: Southworth, B. S., Sivas, A. A. and Rhebergen, S. (2020). On fixed-point, Krylov, and 2×2 block preconditioners for nonsymmetric problems. *SIAM Journal on Matrix Analysis and Applications*. DOI: [10.1137/19M1298317](https://doi.org/10.1137/19M1298317). Author manuscript: [arXiv:1911.02664v4](https://arxiv.org/abs/1911.02664).

Primary full text: https://arxiv.org/html/1911.02664

확인 범위: 정확한 한 대각 블록 역연산자를 가정한 블록 Krylov와 Schur 문제 수렴의 연결. 본 보고서의 대수적 소거와 q-노름 상계는 직접 전개했으며 이 원전의 일반 정리를 특별한 코드 성능 수치로 확대하지 않는다.

## Lang and Verwer (2001)

**Source**: Lang, J. and Verwer, J. (2001). ROS3P—An accurate third-order Rosenbrock solver designed for parabolic problems. *BIT Numerical Mathematics*, 41(4), 731–738. DOI: [10.1023/A:1021900219772](https://doi.org/10.1023/A:1021900219772).

Primary author PDF: https://ir.cwi.nl/pub/1102/1102D.pdf

확인 범위: 적응 격자에서 one-step 방법의 이력 이식상 이점, 경계조건과 정칙성에 의한 차수 저하, ROS3P의 추가 차수 조건. 본 보고서에서 one-step RODAS5P가 임의 remap 뒤 자동으로 5차를 유지한다고 해석하지 않는 근거이다. 이 논문 자체가 현재 8단계 Rodas5P의 검증 자료는 아니다.

## Murphy, Golub and Wathen (2000)

**Source**: Murphy, M. F., Golub, G. H. and Wathen, A. J. (2000). A note on preconditioning for indefinite linear systems. *SIAM Journal on Scientific Computing*, 21(6), 1969–1972. DOI: [10.1137/S1064827599355153](https://doi.org/10.1137/S1064827599355153).

Primary accepted author report (June 1999): https://ora.ox.ac.uk/objects/uuid:36c706dd-45d0-4882-9ff3-cae603b964be/files/mdedd5b22c9f398e41fa1990b1936a2eb

확인 범위: 정확한 Schur 보완이 주는 작은 최소다항식 차수와 정확한 산술에서 유한 반복 종료. 본 코드의 일반 근사 전처리기가 항상 두 번 또는 세 번에 종료한다는 근거는 아니다.

## SciSpace 검색의 역할

SciSpace는 Ostermann–Roche의 stiff PDE 차수 논문(1993, DOI 10.1137/0730056), Ros-SSP 및 exponential Rosenbrock 문헌을 찾았다. 검색 초록만으로 이들 논문의 세부 정리를 인용하지 않는다. 실제 외부 보고서에는 원문을 확인한 위 문헌과 보고서 자체의 완결된 유도를 사용한다.

## 기호 검산과 원전 검토 기록

- EXA_SEARCH.json: Schur 원전 탐색.
- PRIMARY_SOURCE_FETCH.json, SOUTHWORTH_FULL_FETCH.json: 저자 원고 취득.
- EXTRA_PRIMARY_FETCH.json: Murphy–Golub–Wathen 저자 원고.
- RODAS5P_SEARCH.json, LANG_METADATA.json: 원문/서지 확인.
- SCISPACE_SEARCH.json: 관련 논문 발견 기록.
- WOLFRAM_CHECK.json: 원래 차원의 역행렬 및 잔차 전달에 대한 유리수 사례 검산. 첫 호출은 보호된 기호 C,D,E를 사용해 무효였고, lowercase 변수로 수정한 두 번째 호출은 정확한 영행렬과 잔차식을 반환했다. 실패 출력을 지우지 않았다.

## 연구 루프 요약

R1–R3: 기준 코드와 차원·노름·가역성·동일 연산자 전제 확인. R4–R5: 보조 변수 제거, 잔차 전달, 재사용 반경, 다항식 소거, 양의 전파와 remap 오차식 구성. R6–R7: 코드 담당자와 부호·입출력 연결 교차 점검, Wolfram 유리수 역대입. R8–R9: 대수적 결과와 실행 속도·전역 정확도 의무 분리, 구현 입력으로 전달. R10: THEORY.md와 출처 영수증 보존. 기존 공개 증명을 새 성과로 부르거나 변경 없는 수치 캠페인을 재실행하지 않았다. 새로운 no-go 정리를 성과로 채택하지 않았다.

