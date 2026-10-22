# H/He/Li 샌드박스 입력 조사

## 바로 재사용할 결과

Li 계수 원자료는 실제로 존재한다. `raw/`에 보존한 7개 파일은 각 파일의 실제 Dropbox GET, 크기, Dropbox content hash를 검증했다. 전체 SHA256은 `metadata/DROPBOX_RAW_FILES.json`에 기록했다. 현재 Git 조사 담당자가 REI PR150 `fd9a21c4d3886a495540b51d780a83b608e72ea2`의 Li율, isotope_state, 계수 JSON, threshold JSON과 4개 SHA256의 일치를 독립 확인했다. RUN19의 두 Rust 파일도 같은 blob이다.

핵심 source는 `lithium_rates.rs`, `lithium_coefficients.json`, `badnell_lithium_rr_rows.txt`, `badnell_lithium_dr_rows.txt`, `voronov_lithium_ci_rows.txt`다. Badnell RR20230511/DR20230512 및 Voronov1997 원행을 쓴다. 완전한 source URL·원표 해시와 적용범위는 `extracted/dropbox/source_registry.txt`와 `source_notes.txt`에 있다.

## 종 목록과 실제 범위

| 구분 | 종 | 실제 의미 |
|---|---|---|
| native13 | n; H1/D/T 각각 q0,1; He3/He4 각각 q0,1,2 | number bookkeeping; 이것만으로 모든 반응율 선택은 아님 |
| dossier17 | native13 + Li7 q0,1,2,3 | 2026-10-14 별도 dossier Eq1.15의 inventory |
| union21 | dossier17 + Li6 q0,1,2,3 | 명시된 isotope extension; Oct16 snapshot에서 coupled runtime activation=false |
| 외부 reservoir | 전자, 광자, 분자/음이온, 비열전자, Be ancestry 등 | 21개 핵종·전하상태 배열과 다른 owner |

이 registry는 현재 첨부된 PNG와 같은 문서가 아니다. root가 직접 읽은 PNG의 LiIV/Li6 누락 여부는 별도 기록해야 한다. registry의 과거 `no provider` 메모는 2026-10-16 snapshot이다. 이후 PR210/RUN19의 T/He3/HHeD 구현 유무는 최신 Git 조사 결과를 따른다.

Li6와 Li7은 각각 q=0…3 정체성을 보존하지만 계수는 **elemental electronic equality 근사**다. `IsotopeResolved`는 MissingSource를 반환한다. Li8/Li9는 핵붕괴·딸핵종 closure가 미해결이다. 21 fixture states를 21-species network의 실행으로 세어서는 안 된다.

## 지금 고정할 수 있는 계수와 공백

| 채널 | 확인된 계수·영역 | 남는 한계 |
|---|---|---|
| Li RR q1→0, q2→1, q3→2 | Badnell 실제 원행; Tmin=10,40,90 K | 전자 운동에너지/방출 에너지 moment는 이 표가 공급하지 않음 |
| Li DR q1→0, q2→1 | Badnell 실제 원행; 각각 q²에 따른 영역 | q3 bare-ion DR만 구조적 0 |
| Li CI q0→1, q1→2, q2→3 | Voronov 원행; electron Tmin=1,4,5 eV | cold 구간에 자동 외삽하면 안 됨 |
| Li isotope threshold | 중성 Li7 측정 threshold 및 Li6/7 shift donor | 검사한 abstract에서 shift 부호 규약 미해결; 열반응율 보정은 아님 |
| D | 기존 H-equality atomic provider 및 별도 Savin CX | 정밀 isotope-native rate라는 주장은 불가 |
| H/He | 기존 source-backed point provider | old Peebles/Saha 첨부 코드만으로 EoR isotope solver가 완성되지는 않음 |

새 샌드박스가 photo+recombination만 계산한다면 그 축소 process 계약을 명시하면 된다. 이 경우 CI·CX·분자·핵붕괴·광핵반응을 자동으로 포함했다고 기록해서는 안 된다. 냉각과 photon/전자/결합에너지 ledger는 동일 사건을 한 번만 소유해야 한다. 완전한 photothermal 연구에는 별도 RR/DR kinetic/emitted-energy moment와 source spectrum이 필요하다. H edge 이상의 광원만 사용하면 LiI의 약5.392 eV 문턱부터13.6 eV까지의 광자를 빠뜨리므로 광원 support를 명시한다.

## 첨부 원자료 판독

`20-report-1-.pdf`는 HTT Research Compendium(2026-09-13), tensorized MES·운동학·관측·통계 보고서다. 전체 text를 추출해 검사했으며 isotope chemistry rate registry가 아니다. 원파일을 변경하지 않았다.

코드 archive10/11/12/14/17은 vendor를 제외한 regular text source를 검사했다. archive14의 `bianchi/thermo/recombination.py`에는 H Peebles3-level/RECFAST fudge와 He Saha가 실제 들어 있으나 Li/D/T/He3 atomic network는 없다. 12는 scientific harness 계약이다. 19-papers의8개 PDF는 첫2쪽을 읽어 CMB/covariant/Bianchi-radiative-transfer 문헌임을 확인했다. 18-literatures는7개 일반 교과서 제목 목록만 보았다. 모든 교과서 전체에 계수가 없다고 주장하지 않는다. 추출한 archive code는 실행하지 않았다.

## 최신 백업과 대화의 조사 범위

2026-10-21T13:13:38Z 이후를 delta 기준으로 삼았다. REC47/REI55/BASS105/HH464/CR46개의 현재 직접 child 목록, Dossiers3,280개 전체6페이지를 끝까지 확인했다. Rate project35/src51/data10/evidence38/sources2개, Atomic handoff184개 재귀 목록도 끝까지 확인했다. 합계4,317개 metadata 행은 중복을 제거한 전역 파일 수가 아니다. 모든 명시한 scope의 `has_more=false`를 확인했다. `metadata/DROPBOX_LISTINGS.json`과 `DROPBOX_ADDITIONAL_LISTINGS.json`이 원 metadata 범위를 보존한다.

새 파일18개를 확인했으며 이미 완료한 archive는 재다운로드하지 않았다. REI D9 publication ACK와 PHYS77 post-publication receipt는 실제 내용까지 읽었다. D9는 source407f2a30…, metadata cce008b3…에 고정된175case×153saved-epoch 예산 사후 분석이며 new IVP=0이다. PHYS77은 source04bc4a06…, finald43cf2df…의 유한8state/L8 cell 하나이며 z20→약19.53435, new ODE=0, NCP=0이다. 두 결과 모두 새 HHeLi chemistry sweep의 실행 근거가 아니다. 다른 새 ACCEL03/CR_OFFGRID4/HE31 archive는 metadata를 확보했고 최신 Git 내용 검토를 담당 agent와 공유했다. 그 archive 전체를 여기서 다시 읽었다고 기록하지 않는다.

PersonalContext 실제 검색1회는 `Personal context is unavailable for this conversation.`으로 반환되었다. 따라서 다른 대화의 전수 내용 탐색을 완료했다는 주장은 하지 않는다. Dropbox 검색은 index 검색이므로 이름 검색0건을 전역 source 부재의 증명으로 쓰지 않는다.

## 이번 조사 실행 기록

원자료 읽기, 안전한 선택 추출, metadata pagination, 7개 coefficient/source 원byte 확인만 실행했다. 과학 ODE0, NCP0, benchmark0, 완료 case replay0, Git mutation0, 외부 publication0이다. 기존 REC/REI의 physical HOLD와 production 미승인 판정을 바꾸지 않았다.

Stage A publication을 위한 기계판독 요약: `INPUT_SURVEY.json`. 코드/계수의 보존 목록 및 해시는 그 JSON의 artifact_manifest와 `metadata/DROPBOX_RAW_FILES.json`에 있다.
