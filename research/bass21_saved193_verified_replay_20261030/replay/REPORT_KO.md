# VigilODE × BASS21 193-epoch saved source replay: 실측된 모델 이력 검산

2026-10-30 KST. **범위:** `SOURCE_OWNED_SAVED_193_EPOCH_BASS_HISTORY_REPLAY` 단일 보조 연구 노드. BASS의 원래 저장된 193 시점과 원자 반응 모형을 동일 source 및 동일 proper clock에서 직접 재평가했다. 새로운 IVP 없음.

## 원본과 실제 계산

BASS 원본 commit `cc602e209e7deeb511ab2a1f3b2268d55206c43c`. 193 상태 `trajectory.npz` 실제 바이트 254301 SHA256 `37d1bab173e81449cf474b21a678077edd0150cf0b2cb386d014764580678086`; Git blob `57a5375c5183f83d3eb3dd094cac31a212c39de3`. BASS original `RESULT.json` Git blob `4ee306f3af3ef5391d6309794cccfa1459067441` 내 `samples=193`, `BDF`, 원래 `rtol=2e-8`, 원본 최종 `τ_T=0.06934076449489847` 확인. 원본 BASS atomic network Python bytes sha `baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80`를 직결한다.

모든 193 saved `x,Y,H,n_b,T,UV` 시점에서 원본 AtomicNetwork.evaluate를 **직접 193회** 실행. 21-dim native F와 21×21 native J를 저장. 별도 F/channel sums, analytic rank-one electron-feedback Jacobian, photo/RR/DR/weak electron source, H/He3/He4/Li6/Li7 photo rates, source photon power를 모두 계산했다. 원본 archived 193-row `photo_absorbed`, `primary_photoelectron`, `unresolved_core`, `uv_energy`, `li1s_events` 다섯 진단량은 binary64 값으로 정확히 일치한다. C-order native Y/nb evaluation의 최대 ne 차이는 0.0 m^-3; A·Y-1은 최대 9.99e-16, chemical A·F 최대 4.55e-32 s^-1, native J의 A·J 최대 2.12e-24 s^-1 (finite matrix assembly roundoff).

## 발견된 실제 물리 이력

- z=20→2, x=0→ln7, proper elapsed 9.75955460329061e16 s. 이 기록은 관측 데이터가 아니라 원본 조건부 BASS 모형 계산의 저장 이력이다.
- 초기 전자분율 0.00015099503200860547, 후기 0.8774564518451784. 처음 x_e≥0.5인 저장 index 70, z=9.33024529994081 (`interpolated physical threshold redshift` 아님). 최대 native τ_x=0.09632327786862802, index78 z=8.525728868092976.
- Photo electron source peak +2.2733086979188697e-13 m^-3 s^-1 at index71 z=9.22607755; RR electron sink 가장 큰 절댓값 1.8219776738433127e-13 m^-3 s^-1 at index73 z=9.02088268. 초기 net chemical electron source -1.0809151085404077e-18 m^-3 s^-1, 후기 +3.3326075098379702e-21 m^-3 s^-1.
- 저장된 193 point source에서 integrated photon-ionization events per baryon 약 4.986531593057093, integrated RR events per baryon 4.109226145829888, DR events per baryon 2.5741774676656985e-18. 이는 여러 ionization/recombination cycles의 누적 event exposure이지 독립 전자수나 photon budget posterior가 아니다. 누적 net chemical source로 Simpson이 복원한 전자분율 0.8774564422592093; saved final 0.8774564518451784, 차이 -9.585969129588534e-9. 낮은 오차는 실제 저장 수치 이력의 postprocessing consistency이지 BDF 독립 오차 bound가 아니다.

## optical depth: source와 quadrature의 분리

같은 저장 H와 native `ne`로 직접 계산한 `tau_x=(cσ_T)ne/H`가 저장된 `gamma_T_per_H`와 **193/193 binary64 exact**. source 속도/단위/clock 불일치 발견되지 않음. 누적 `τ_T`의 비교는 다음과 같다.

| 193 sampled source postprocessing | 최종 | 저장 최종값에 대한 차이 |
|---|---:|---:|
| BDF source 저장 τ_T | 0.06934076449489847 | baseline |
| 사다리꼴 | 0.06934051590351172 | -2.485913867533718e-7 |
| Simpson | 0.06934076446308475 | -3.181371832639002e-11 |

Simpson endpoint 상대 차이 4.58802532076649e-10, 그러나 **전 시점의 최댓값** |Δτ|=6.788747238667403e-8 (index67, z=9.649158503658121). 사다리꼴 최대 |Δτ|=5.88043022451204e-6. *두 postprocessing 적분의 최종 우연한 근접을 BDF 통합 오차 인증이라고 해석하면 안 된다*. 원본과 같은 `dt/dx=1/H`를 적분한 proper elapsed 최종 Simpson 상대 차이 5.515824664974934e-9.

**오차 분류:** 화학 source/clock/opacity identity = verified within native model; 저장 누적값 vs reconstructed integrals = numerical postprocessing quadrature+interpolation and original BDF integration combined, 정확한 error decomposition은 unresolved; 새로운 독립 물리모형 mismatch에 대한 evidence 없음. 원본 tolerance를 변경하거나 재실행하지 않음.

## 실제 검증/승격 상태

- 신규 `PYTHONDONTWRITEBYTECODE=1 python -m unittest discover -s replay/tests -p 'test_*.py' -v`: **20/20 PASS**, exit code 0 (현재 실행). 이전 PR118 `bash new_physics/VERIFY.sh`: **13/13 PASS**, exit code 0, source manifest 및 previous output byte-identity PASS. 동일 물리 source source SHA 및 original 127 endpoint archive pinned.
- Origin authority: BASS original bytes + previous exact source, previous PR118 source ZIP SHA `ba3f07a4f881ae8b02fd7b9690242ee29cfd1ae9f2b885b2601487010408508b`. Original 129 BASS coefficient numerical binary64 projection matches earlier verified identity, **original raw coefficient JSON bytes are NOT locked**.
- 새 보조 193 saved-epoch replay data unit: 1/1 **SCIENCE_DATA_SCOPED**, independent review HOLD. 새 R/C 18/20 *scoped execution roles*, 다른 저자 R6/C9 0/2, NOT_RUN. Original VigilODE 7/23 unchanged; historical N03 17/20 scoped; REI central28/88 unchanged (working 33/88 distinct), successor 0/12 admitted, production HOLD. PR118 prereg/ledger CI remains FAIL and retroactively fixed/passed 주장 금지.
- 모델은 prescribed T/UV, test-fluid, finite21 photo/RR/DR/weak; UV absorption ≠ gas heating. Native full RODAS, owner epoch transaction, actual independent R6/C9, observational CMB/likelihood/production all NOT_RUN.

## 산출물

`results/NATIVE_BASS_193_EPOCH_HISTORY.csv` (193 rows, photo/He/Li/electron/τ history); `results/ACTUAL_FJ_ALL_193_EPOCHS.npz` (193×21 F / 193×21×21 J); `results/SUMMARY.json` (실제 수치, source identities, 논리적 status); `code/replay.py` (실행 재현); `tests/test_replay.py` (20 checks), `VERIFY.sh`, 원본/원천 소스와 checksum manifest, replay-only plots, raw test/negative evidence. 제약 위반 없음.
