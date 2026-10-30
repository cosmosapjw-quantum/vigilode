# VigilODE BASS21 E2 spin-2 source-qualified science | 2026-10-30

**목적:** 원본 BASS21의 동일 50×193 역사에서 기존 frozen-background Thomson `Q`를 spin-2 모멘트로 투영하여 원본 BASS collision `E2`와 계수 대응 및 전단–고차 모멘트 결합을 실제 데이터로 수치 판별한다. 새로운 원본 IVP **0개**.

- **원본 owner:** `cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c`; 원본 23,770,654-byte source archive SHA256 `6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8`.
- **즉시 부모:** PR133 HEAD `24a0c534adca54c700bd050cd8358ce3638e3756`; 33,479,324-byte archive SHA256 `ffd7623ba2e2bd99233a1244ac7945470f6eeed348a63a4fd02f5967ad449b14`.
- **전제:** metric `(-,+,+,+)`, 비틀림 없는 normal-comoving cold Thomson electron, `x=ln(a/a_ref)`, 정상 편광 기준 `Q=I_theta-I_phi`, 원본 `b,H,n_e,tau,Jℓ` frozen. `g=c σT n_e/H`.

### Exact physical equations

`B_Q=2∫_0^1 (1−μ²) Q dμ`, `E2cand=3B_Q/(4√6)`, `c_ℓ=∫_0^1 Q P_ℓ² dμ / [ (ℓ+2)!/((2ℓ+1)(ℓ−2)!) ]`, ℓ=2,4,6,8,10.

`dE2/dx=b'[(10/7)E2+(4√6/7)c4]-g[(√6/10)(J2_orig+δJ2)+(2/5)E2]`.

`δ(dJ2/dx)|polarization=−g(√6/10)E2=−3gB_Q/40`. **여기서 E2는 source collision 정규화 후보**이며 BASS의 source-evolved Eℓ과 독립 동일성 증명은 하지 않았다.

### Full original source results

- `50 × 193 × 5 = 48,250` 신규 spin-2 계수; `25 × 193 × 5 = 24,125` paired 계수 차이; `9,650` E2 진화 진단 시점, `4,825` paired E2 시점.
- 끝점 E2 평균 `+0.0016137252924255763`; matched 평균 ΔE2 `+0.0032267938218122926` (**25/25 positive**). 끝점 c4 평균 `−3.9102139223170216e−5`; J2 편광 collision feedback 끝점 평균 `−7.145382299422289e−6` per x.
- E2 끝점 37/50 양수, 13/50 음수. 따라서 **모든 50 case의 E2가 같은 부호라고 말하면 오류**. s04에서 끝점 `median |c4/c2| = 0.023773662817124716`.
- 평균 끝점 shear source `+2.9112494639479958e−5`, collision source `+0.0004621387500271969`. 저장 이력 적분 평균 shear `+5.410463643906042e−5`, collision `+0.0015596162296362083`, 합과 E2 평균의 차이는 해당 수치 적분 오차 규모.
- angular GL64/96 max `6.071532165918825e−18`; collision J2 crosswalk max `1.3552527156068805e−20`; E2 angular projection max `3.903127820947816e−18`. 저장 시점 E2 local derivative diagnostic 최대 차이 `8.806325989430853e−6`, integrated Simpson 최대 `9.032401512143592e−9`: **global BDF convergence NOT_CERTIFIED**.

본 연구는 원본 Jℓ을 변화시키는 완전한 동시진화 Einstein–Boltzmann 해가 아니고, 관측 CMB E/B or z=0 τ가 아니다. 자세한 counterexample과 claim gate는 `reports/BLIND_REFEREE_KO.md`에 기록.

### Deliverables and status

`results/BASS21_E2_SPIN2_50x193x5.npz` = full arrays (Git text mirror에는 없음), `code/e2_spin2_atlas.py`, `source_code/` exact PR132 source excerpts, `results/SCIENCE_SUMMARY.json`, `results/PHYSICS_INTERPRETATION.json`, endpoint CSV ×3, tests, independent stored calculation replay. Research DAG **VigilODE 7/23; REI central 28/88, working 33/88; BASS CE1 9/15; successor 0/12; official delta 0**, auxiliary scoped data 1/1 HOLD. Native independent R6/C9 NOT_RUN; production HOLD; main merge/force push NO.
