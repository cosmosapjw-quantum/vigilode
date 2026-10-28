#!/usr/bin/env python3
"""Render the local Korean report from retained analysis; no solver execution."""
import pathlib,json
R=pathlib.Path(__file__).resolve().parent
s=json.loads((R/'RESULTS.json').read_text());rows={r['id']:r for r in s['all_rows_with_requested_tolerance_and_achieved_error']}
labels={'decay':'감쇠','driven':'비자율 forcing','logistic':'logistic','harmonic':'harmonic','rapid':'빠른 진동','damped':'감쇠 진동','mixed-scales':'혼합 scale','nonnormal':'비정규 transient','large-vector':'큰 vector','moderate-stiff':'moderate stiff','stiff-control':'stiff negative control'}
def comp(f,t,mode):return next(c for c in f['comparisons'] if c['normalized_error_threshold']==t and c['comparator']==mode)
def ratio(c):return f"{c['comparator_over_explicit_time']:.3g}" if c['status']=='COMPARABLE' else '미달/미비교'
table=[];details=[]
for f in s['problem_families']:
 native='rodas-mf' if f['dimension']>=256 else 'rodas-dense'
 table.append(f"| {labels[f['case']]} | {f['dimension']} | {ratio(comp(f,1e-6,native))} | {ratio(comp(f,1e-8,native))} | {ratio(comp(f,1e-6,'vode'))} | {ratio(comp(f,1e-8,'vode'))} |")
 for t in [1e-6,1e-8]:
  for c in f['comparisons']:
   if c['normalized_error_threshold']!=t:continue
   a=rows.get(c['explicit_config']);b=rows.get(c['comparator_config'])
   details.append(f"| {f['case']} | {f['dimension']} | {t:.0e} | {c['comparator']} | {a['rtol'] if a else '없음'} | {b['rtol'] if b else '없음'} | {c['explicit_error']} | {c['comparator_error']} | {c['status']} |")
report='''# Nonstiff 명시적 적분 경로의 로컬 검증과 확장 비교

2026-10-23 (Asia/Seoul). 고정 실행 소스의 준비된 시험·하네스를 실행했다.
소스 코드는 수정하지 않았고 새 알고리즘 연구나 과거 연구의 일괄 재감사는
수행하지 않았다. 표적 시험 25개와 새 측정의 재현성은 확인했지만 범용
production·통계적 속도 권한은 HOLD다.

## 실행 source와 환경

실행 source는 `45037fc24e8f8101605f309e1fdb3a9a4e3df56d`, tree는
`b43b69b1ec5b86f3577d7cd6efdefe2d3acb9bad`다. 후속 보고서 tip
`8c80985e30778df4ba44b2b7085753fa345cf0e2`에서 REPRODUCE/HANDOFF를 읽고
published-docs에 따로 보존했다. 후속 보고서 HEAD를 실행 source로 사용하지 않았다.
원래 checkout의 HEAD/branch/status를 기록한 뒤 mktemp로 만든 별도 디렉터리에
`git worktree add --detach`를 사용했다. 모든 결과와 build는 source worktree 밖의
새 실행 디렉터리에 썼다. NCP/SSH, reset, clean, 강제 checkout, 기존 파일 삭제는 없다.

Rust/Cargo 1.94.1, 고정 Cargo.lock을 사용했다. 제시된 /workspace/scratch의
installed/bin·cargo 경로는 이 호스트에 없어 사용하지 않았다. 최초 offline
metadata는 `anstyle-wincon 3.0.11` 누락으로 exit101이었다. 실패 로그를 보존하고
허용된 `cargo fetch --locked`로 정확한 lock의 의존성을 준비했다. 이후
`cargo metadata --locked --offline`은 통과했다. Lockfile/manifest 변경이나
upgrade, --locked 제거는 없다. 의존성 fetch 약3초, 표적 시험 빌드 포함
약81초, joint measurement build 약148초는 solver 시간에 포함하지 않았다.
정수 초 단계 시간은 실행 영수증의 UTC timestamp 차이이며 정밀한 profiler 값이 아니다.

Measurement profile은 opt-level3/LTO=false/codegen-units16이다.
CARGO_TARGET_DIR는 이 run의 build 디렉터리이며 jobs2, dev/test debug0이다.
CPU는 AMD Ryzen9 5900X, 허용 affinity0–23이다. 실제 cgroup 경로 및 상위
cgroup의 CPU/memory 제한은 max, 주소공간 rlimit도 제한 없음이었다.
시작 load average는 약16.6/13.0/10.6으로 호스트를 격리하지 않았다.
Python3.12.3, NumPy2.4.2, Matplotlib3.10.8, SciPy1.17.0을 실제로 확인했다.
NumPy/SciPy의 관측 OpenBLAS threadpool은 각각24 threads다.
이 환경을 timing에 대한 일반 보증으로 해석하지 않는다.

| 바이너리 | SHA256 |
| --- | --- |
| nonstiff_bench | 50534f6dc5da3c7c123e74c83c2d6d437474a94282decd274711534292586ac1 |
| nonstiff_ensemble_bench | d42d7338120dcd7288e9b1ac3a622fe0385ccba5d7bd9ba3ddd17f99e9228480 |

## 표적 시험과 공개 API 범위

Kernel contract12, allocation1, public API8, ensemble4로 총25/25가 통과했다.
차수·stage time, FSAL/거절 작업량, scalar/vector atol과 zero component,
잘못된 입력, recoverable/fatal/malformed callback, partial result, 지원하지
않는 기능의 조기 거절, 실제 ensemble overlap과 member ordering을 검사한다.
Kernel allocation 시험은 핫 루프의 무할당 계약이며 solve 전체의 heap 사용이
0이라는 주장은 아니다. 이25개 시험이 전체 저장소나 모든 production
문제군을 검증하지는 않는다.

Dopri5는 사용자가 선택하는 forward identity-mass ODE 경로다. 비자율 RHS,
scalar/vector atol을 지원한다. Jacobian/JVP/dfdt, matrix factorization이나
Newton solve를 호출하지 않는다. 이번192개 explicit raw record와 ensemble
전체의 관련 counter가0임을 추가 확인했다. Mass 선언, event, dense output은
RHS 전 거절하며 관측 시각에는 실제 step을 잘라 도착한다. 자동 stiffness
탐지나 method switching은 없다. Jacobian 부재를 nonstiff 판정으로 사용하지 않는다.
FSAL 때문에 한 solve 동안 RHS는 같은 수학적 함수를 나타내야 한다.
부분 실패는 success=false와 마지막 accepted state를 보존한다. 호출 예제는
published-docs/REPRODUCE.md의 `ensure_success()` 또는 동등한 검사를 따른다.

## 본 비교: 성공과 정확도 조건

--full의 실제 tolerance grid는 `1e-3,1e-5,1e-7,1e-9`이며
`atol=1e-3*rtol`이다. compact 계획 문단의 기본3단계 grid를 실행값으로
잘못 읽지 않았다. 11문제군, large-vector의4096/16384 두 차원을 합쳐
12개 case–dimension 조합, 세 방법, 네 tolerance로144설정이다.
각 설정 warmup1회와 timed3회를 실행했고 순서는 seed20261023으로 무작위화했다.
Native 내부 비교는 작은 문제에서 RODAS5P dense, 큰 vector에서 기존
matrix-free JVP 경로다. 큰 vector에 dense n^3 baseline을 억지로 적용하지 않았다.

7200초 main 캠페인 예산과45초/process 제한을 유지했고 캠페인은
158.1425초에576/576process를 완료했다. Timeout, budget exhaustion, launch/
solver failure는 이번 실행에서0이다. 144개 설정 모두 출력 일정이 완료됐다.
별도 분석에서576개 저장 상태의 길이·유한성·관측 시각·정확해·오차·source/
configuration identity·raw hash가 유효했고 기준해 불일치 최대값은0이었다.
성공 반환과 replay 유효성만으로 두 공통 정확도 기준을 만족했다고 간주하지 않았다.

공통 정확도는 모든5개 저장 시각·모든 성분의
`|y-y_exact| / max(1,|y_initial|,|y_exact|)` 최댓값으로 정의한다.
비용비는 양쪽 모두 이 값이1e-6 또는1e-8 이하인 설정 중, 각 방법의
측정 중앙값이 가장 작은 설정끼리만 계산했다. 실제 오차가 서로 정확히
같다는 뜻이 아니라 같은 사전 정의 상한을 충족한다는 뜻이다. 선택한 rtol이
다를 수 있으며 요청 tolerance는 전역 오차가 아니다. 네 설정 밖의 전역
최적 성능을 찾았다는 의미도 아니다.

아래 값은 **비교 방법 시간 / DOPRI5 시간**이다. 1보다 크면 해당 적격 표본에서
DOPRI5가 더 빨랐다. 시간은 workspace/setup·RHS·solve를 포함하고 정확해와
JSON 직렬화는 제외한다. Native compile과 process launch/import overhead는
solver timer 밖이며 process wall은 별도 보존했다. 미달/미비교는 null을
그대로 표시한 것으로 누락 수치를 채우지 않았다.

| 문제 | 차원 | Native/DP ≤1e-6 | Native/DP ≤1e-8 | VODE/DP ≤1e-6 | VODE/DP ≤1e-8 |
| --- | ---: | ---: | ---: | ---: | ---: |
'''+ '\n'.join(table)+'''

48개 비용 비교 중43개만 적격이다. 관측 min/max와 ratio envelope는
신뢰구간이 아니다. 전체 선택 설정의 rtol·실제 오차·비용은
RESULTS.json과 [원 분석](full-comparison/SUMMARY.json), 부록 표에 보존했다.

## 문제군별 손익과 미달

매끄러운 decay·forcing·logistic에서는 tight 공통 기준에서 native 대비
약2.29–3.90의 시간비를 관측했다. 비자율 forcing은 dfdt 호출 없이 처리했다.
이 세 analytic 사례를 모든 비선형/비자율 문제의 보증으로 옮기지 않는다.

Harmonic에서 DOPRI5의 tight 적격 표본오차는3.025e-9였고 native 시간비는
2.34였다. VODE의 최선 표본은1.284843e-8로1e-8 요구에 미달해 해당 비용비는 없다.
빠른 진동에서는 DOPRI5 최선도1.971506e-8, VODE도8.231704e-8로1e-8 요구에
미달했다. 더 느슨한1e-6 요구의 native/DP 시간비는 약0.820으로 DOPRI5가
느렸다. Native는1e-8 요구를 충족했다. 빠른 진동의 미달과 비용 손실을
삭제하거나 tolerance를 더 추가해 결과를 바꾸지 않았다. 별도 후속으로
phase/interior 출력 검증과 더 엄격한 사전등록 제어의 필요성을 남긴다.

감쇠 진동의 tight native 시간비 약1.07은 작은 차이이며 이 호스트의 부하·
세 반복을 넘어선 안정적인 속도 우위로 승격하지 않는다. 혼합 scale과
비정규 transient에서도 finite sampled 성능/오차 관측은 확보했다. 그러나
정규화된 오차와 절대오차는 다르고5시각 사이의 peak나 phase 오류는 별도다.
성분 scale·물리 불변량·중간 시각에 대한 client 검증이 추가로 필요하다.

4096/16384차원 diagonal vector에서 tight native MF 시간비는 각각
14.78/14.08이었다. 4096의 더 느슨한1e-6 요구에서는 VODE 시간비가0.963으로
DOPRI5가 약간 느렸으며 이 작은 차이를 안정적인 우위/열위로 승격하지 않는다.
Tight 요구에서 VODE 시간비는1.12/96.15로 급격한 차원
성능 절벽이 보였다. VODE rtol1e-9의 solve 중앙값은4096에서0.004467초,
16384에서1.52681초였다. 관측 threadpool24와 동시 호스트 부하를 보존했지만
원인은 NOT_MEASURED다. 이를 일반적인 복잡도·차원 scaling 법칙 또는
VODE 전체에 대한96배 우위로 해석하지 않는다. BLAS thread/호스트 부하를
고정한 별도 후속 측정이 개선 항목이며 이번 결과를 재튜닝하지 않았다.

Moderate-stiff control의 tight native 시간비는1.34였지만 stiff negative
control에서는0.00853이었다. 즉 이 stiff 표본에서 DOPRI5가 native보다
약117배 느렸다. 이 control에서 VODE BDF(with_jacobian=false)는 모든
등록 tolerance에서1e-6 기준도 충족하지 못했다. 최선 오차는1.659438e-6이고
해당1e-6/1e-8 비용비는 모두 null이다. 이는 기록한 VODE 설정과 문제에 대한
관측이지 모든 VODE BDF 구현의 실패 판정이 아니다. 명시적 안정성 제한을
숨기지 않으며 자동 stiffness switching 기능이 있다고 주장하지 않는다.

## 외부 비교 identity와 제한

실행한 외부 비교는 bundled scipy.integrate.ode의 Fortran VODE/DVODE뿐이다.
Nonstiff에는 Adams, moderate/stiff에는 BDF, with_jacobian=false,
first_step=1e-3, nsteps=100000을 사용했다. SciPy RK45/Radau/LSODA를 대체
비교로 사용하지 않았다. Python/NumPy callback 비용은 solver timer에 포함하고
imports는 제외했다. VODE는 관측 출력에 내부 interpolation을 사용할 수 있어
실제 step을 잘라 착지하는 native 출력 정책과 차이가 있다. 이 구현/언어/
출력 비용 차이를 수학적 stage 비용의 차이라고 부르지 않는다.

Diffsol·Julia는 NOT_RUN이다. Julia 실행 파일은 관측했지만 호환되는 준비된
adapter는 없었다. 기존 local diffsol binary와 Julia component adapter는
요청/응답 schema가 달라 연결하지 않았으며 새 설치/adapter 구현은 하지 않았다.
VODE adapter source 사본·SHA256, Python executable와 Fortran extension SHA256,
실제 SciPy/NumPy 버전·환경 freeze lock 사본과 hash, BLAS/thread 설정,
미실행 candidate 식별자/원인은 EXTERNAL_ADAPTERS.json에 있다. 미실행 Julia의
method/version/JIT 정책이나 diffsol runtime version을 추측하지 않았다.

## 독립 궤적 ensemble

별도180초 예산에서16개 독립4096차원 문제를1/2worker로 실행했다.
SUMMARY의 캠페인·replay elapsed는 약3.8924초(archive 생성 전 기록)였고
16/16 process와 replay가 유효했다.
각 member의 적응 step과 오차 제어는 독립적이다. pool setup은 timer 밖에
따로 기록하고 각 member의 setup/workspace/integration은 timer 안이다.

| rtol | 1worker 중앙값(s) | 2worker 중앙값(s) | 1worker/2worker | 최대 표본오차 |
| --- | ---: | ---: | ---: | ---: |
| 1e-6 | 0.031819852 | 0.016783773 | 1.896 | 6.234858e-8 |
| 1e-8 | 0.063726150 | 0.035190792 | 1.811 | 5.291845e-10 |

모든 warmup과 반복을 포함해 worker 수 사이의 저장 상태 digest와 work가
일치했다. 관측은 독립 궤적 throughput이며 단일 궤적 RK stage 병렬화,
homotopy 또는 한 문제의 물리 시간 구간 병렬화 근거가 아니다.

## 재현성, 한계와 판정

원시 stdout/stderr는 삭제하지 않았고 main1152개, ensemble32개 archive entry의
bytes/hash를 현재 파일 및 tar 내용과 모두 대조했다. Receipt576+16개의
raw hash,687개 source-input hash, 두 binary hash도 일치했다. Native/외부
전체 설정의 work는 warmup·반복 사이에 같았고 explicit derivative/solve
counter는0이었다. PLAN, RECEIPTS, SUMMARY, REPLAY, RAW_OUTPUTS.tar.gz,
RAW_MANIFEST.json, 도표, 환경·의존성 준비 실패와 실행 명령을 보존했다.

25개 계약과 등록된 finite analytic 사례의 측정/재현 검증은 scoped 결과다.
모든 시간에 대한 엄밀한 오차보증, phase/peak 완전 검증, 일반 stiffness
분류, event/mass/dense-output 지원 또는 범용 production 승격은 아니다.
통계적 권한과 범용 production은 HOLD다. 현재 최소·최대는 관측 범위이며
신뢰구간이 아니다. 정확도 미달5개 비교와 외부 adapter 부재는 그대로 남는다.
전체 작업에 빌드·의존성·분석 시간이 별도로 있었으므로7200초 main 예산을
작업 전체의2시간 보증으로 서술하지 않는다.

이 실행의 원래 checkout 보존과 마지막 source/tree/status, 독립 검토 결과는
FINAL_CHECKS.json과 REVIEW.json에 기록한다. 원격 publication은 수행하지 않는다.

![저장 상태 재검증 기반 work–precision](full-comparison/WORK_PRECISION.png)

## 부록: 실제 선택 tolerance와 달성 오차

| 문제 | 차원 | 공통오차 상한 | 비교방법 | DOPRI rtol | 비교 rtol | DOPRI 실제오차 | 비교 실제오차 | 판정 |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | --- |
'''+ '\n'.join(details)+'\n'
(R/'REPORT_KO.md').write_text(report)
print(str(R/'REPORT_KO.md'))
