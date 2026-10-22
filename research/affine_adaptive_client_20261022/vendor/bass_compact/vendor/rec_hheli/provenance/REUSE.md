# Stage A: 최신 source 및 실행반환 재사용 조사

조사 시점은 2026-10-21 14:43:37 UTC 이후의 GitHub connector 조회다. **6개 저장소의 현재 branch 1,060개와 all-state PR metadata 824개를 API의 빈 마지막 페이지까지 수집했다.** 전체 branch의 모든 commit과 파일을 내려받았다는 뜻은 아니다. 필요한 scientific owner, 새 실행반환, 원자·핵종 provider 파일을 선택해 byte SHA256와 Git blob SHA1로 고정했다.

## 실제 범위와 페이지 완결성

| 저장소 | 현재 branches | branch 페이지 행수 | all-state PR | PR 페이지 행수 | PR head와 매칭되지 않은 branch |
|---|---:|---|---:|---|---:|
| rec_bianchi | 136 | 100, 36, 0 | 118 | 100, 18, 0 | 23 |
| rei_bianchi | 258 | 100, 100, 58, 0 | 216 | 100, 100, 16, 0 | 42 |
| WU088_HH | 141 | 100, 41, 0 | 132 | 100, 32, 0 | 9 |
| bass | 264 | 100, 100, 64, 0 | 179 | 100, 79, 0 | 87 |
| bass_cr | 172 | 100, 72, 0 | 95 | 95, 0 | 77 |
| BASS_HE | 89 | 89, 0 | 84 | 84, 0 | 5 |

243개의 “PR head 미매칭 branch”는 각 prs/*_all_metadata.json의 unassociated_branches에 이름과 SHA를 보존했다. 이는 원격 refs이므로 미게시 파일이나 모든 사적 대화가 존재하지 않는다는 판단으로 확장하지 않는다. branch metadata에는 commit 시각이 없으므로 이름만으로 최신 실행 여부를 결정하지 않았다. API 응답은 끝까지 수집했고, 일부 도구 출력의 화면 truncation은 보존된 JSON/source bytes의 truncation이 아니다.

## 현재 고정과 실제 새 결과

- **REC118** 99f5f4fb57358a0347286f8299977125a2724774 유지. 현재 HEAD checks 2개 모두 success. 저장 200점 검증과 tiny IVP 2개는 완료 결과로 재사용하며, full 153-node × 3-case tangent는 실행 완료로 바꾸지 않는다.
- **REI199 D9** 실제 최신 head cce008b3fc32c298185b30550b2cd909d74fcd87, scientific content 407f2a30efcdd0c4c4afb180cbe7d31f90cc0401. 저장 175×153점과 35 reference의 component shear budget; D9 max scaled residual 1.473444875767401e-6, 기준 2e-5. 새 IVP/callback/moment recomputation 0. PR 본문의 이전 AGE source와 현재 D9 head를 구별한다. 현재 HEAD checks 2개 failure이며 green CI로 기록하지 않는다.
- **REI217 / BASS179 ACCEL03** metadata de73c5d6c7bd3498571e9226298040d8259e1acf / d14652646c1da77b6513e7f97725ff5dd5c1db4e, 실행 source 39ae7c843ea2d1adf580aff44aebfdedea7e33aa. 이미 완료된 warm NCP 5×2step, 15 unique samples의 portable aggregation과 response derivative 재사용; 이 단위 새 ODE/build/benchmark 0. a=.1999→.2를 z20→2 진화로 해석하지 않는다. REI217 현재 HEAD checks 2개 failure.
- **CR94/95** NCP 및 sandbox의 같은 offgrid 4 좌표를 재사용한다. 125 prefix에 합친 unique input은 129이고 133이 아니다. CR94 head 92b1b3f5575375580cf8de8039252ac492353b7e. CR95 branch ref 8dc5031b3323d4be42a7444843294200aa8887f2, PR API head e7729092abc6edbe74f31d0ed82aa84d5506b8d5가 다르며 owner 본문이 이를 metadata lag로 설명한다. NCP addendum 038aa0c8e5790f5c94e596035ea816fbca588a15. byte-identical array comparison은 owner receipt이고, CSV CRLF/LF manifest mismatch 두 건은 보존한다.
- **HH132 PHYS77** source 04bc4a063384b2629a323ad15bdf6618fa8c14f0 / final d43cf2df2c4864ae05049f44301c5b389b80a49c. 첫 one-cell z20→19.534353에서 8개 strict inclusion 및 42 controls/46 containments; 새 ODE/NCP 0. 전체 z20→2 또는 일반적 오류한계로 확장하지 않는다.
- **HE84 HE31** executable 1c8a56aac89d0e40ede8770009f80ebca40a4d2c / final 88e415c45d8607c85cd3ab36aabeaed4378820ce. prepared canonical moment kernel과 실제 donor on/off + original reference 결과가 있다. 최대 full-state absolute 차이 1.803e-12. donor 지원은 z4까지이며 z2의 일반 H/He chemistry solver로 사용하지 않는다.

REC116 4cffa8a0…, REC117 77b71c7d…, REI213 d32cd99f…, REI215 926bae81…, REI216 1f32cc44…, BASS178 215bb3e6…, CR93 0f45d0a4…도 metadata에서 보존·대조했다. warm 729, REC U4B, HE29와 위 완료 결과의 재계산은 수행하지 않았다.

## 바로 재사용 가능한 source

원문 33개는 sources/ 아래와 SOURCE_MANIFEST_BATCH01/02/03.json에 저장했다. 모든 성공 retrieval은 UTF-8 원문을 그대로 저장한 후 원격 Git blob과 재계산 blob이 일치한다. HE root LICENSE 요청 404는 failures에 보존했다. REI150 MIT LICENSE 원문은 확보했으나 이를 다른 저장소의 모든 신규 코드 라이선스로 확장하지 않는다.

1. **PR150 / RUN19 identity·rates**: fd9a21c4d3886a495540b51d780a83b608e72ea2의 rust/rei_microphysics/src/{lithium_rates.rs,isotope_state.rs}, coefficient/threshold JSON. Dropbox 원본 4개 SHA256와 동일하며 RUN19 c9deb933…의 upstream Li blob도 동일하다. Li6/Li7 각 q=0..3을 보존한다. full Li CI/RR/DR bundle 공통 하한은 약 58,022 K이므로 cold pilot에 그대로 적용하면 안 된다. Li8/9 decay closure는 별도 미구현 상태다.
2. **PR210 atomic catalog**: e4c8133288614638792888367ca10b2ded3138ff. source-locked elemental PI/CI/RR와 domain checks를 제공한다. isotope-specific rate의 완성이나 network admission을 의미하지 않는다.
3. **PR151 Unit7 Li I 고에너지 흡수**: 8a4f5df46f6e3058f5e685b85cc1cf3c20926368의 code/li_photo_driver.py와 spec/LI_PHOTO_CONTRACT.json. Verner phfit2의 2s PH2를 64.39eV 아래, PH1을 그 위에 사용하고 1s PH1 partial을 합한다. 기존 photo.dat valence band만 보고 고에너지 provider가 없다고 판정하면 안 된다. 단, 계약은 Auger/fluorescence, relaxation, heat/charge yields를 명시적으로 미포함한다. 따라서 absorption provider를 직접 ionic daughter/열침적 closure와 동일시할 수 없다. Unit8 li_thermal_rates.py에도 Milne/total RR 구현이 있지만 kinetics 검증은 HOLD다.
4. **RUN19 H/He/D finite owner**: research/atomic_nuclear_ssot_20261017/run13/recovered/code/rei_bass_coupled/{owner_src/atomic_provider.rs,owner_src/igm_rates.rs,src/full_network.rs,src/spectral_native.rs}. 유한 prescribed photon-bin one-step/split owner이며 전체 cosmic source history는 별도다.
5. **HE31 prepared evaluator**: sources/HE84/.../src/{prepared_moments.py,electron_donor.py,receiver.py,bindings.py}. canonical geometry 사전계산과 immutable packet moments를 재사용할 수 있다. z4 donor 한계는 유지한다.

RUN19 TARGET_NETWORK_19.json은 요청된 19 reaction identity와 stoichiometry를 보존하지만 선택된 lineage에서 4–19의 native coupling은 UNRESOLVED_NATIVE_COUPLING다. 이를 PR151 등 모든 외부 branch에 provider가 없다는 주장으로 넓히지 않는다. 이번 선택 조회에서 all-isotope/ion + UV + nuclear + thermal + LRS z20→2가 모두 닫힌 단일 실행 owner는 확인하지 못했다. 새 pilot은 채택한 유한 closure를 명시해야 한다.

## 판정과 접근 공백

READ_ONLY 조사이며 새 ODE=0, NCP=0, benchmark=0, Git mutation=0, external post=0. 반환물은 직접 API에서 읽은 metadata/선택 source와 owner의 저장된 receipt를 구별한다. 전체 private 대화, 모든 NCP filesystem, 모든 branch의 모든 파일 내용에는 접근했다고 주장하지 않는다. backup 조사는 별도 INPUT_SURVEY.json과 대조한다. Physical HOLD, global convergence NOT_RUN, production NOT_APPROVED, 원래 REC U5 HOLD는 유지한다.

