from pathlib import Path
import json
root=Path(__file__).resolve().parents[1]; sd=root/'stages'; sd.mkdir(parents=True,exist_ok=True)
stage_desc={
 'R1':'source-owned physical endpoint atlas question, no new IVP',
 'R2':'real Verner/Badnell source rates and BASS saved original 127-case reuse',
 'R3':'proper-time s, electron charge q, optical x and isotope inventory conventions',
 'R4':'compare stored-epoch data vs new IVP (selected stored source) and REI17 mismatch',
 'R5':'source charge decomposition, baryon invariant, Li contribution, paired shear τ result',
 'R6':'real different-author physics-review dispatch absent',
 'R7':'Wolfram charge identities and original 127 endpoint source observations',
 'R8':'conditionally physical BASS pilot interpretation; cross-model and production claims held',
 'R9':'BASS21 F,J and electron τ bridge with unit/clock/provenance and state-source mapping',
 'R10':'source-inclusive reproducible artifact and exact next trajectory restoration',
 'C1':'reproduce original source with 127 stored endpoint baseline and 5 native evaluations',
 'C2':'mapping native AtomicNetwork.evaluate and BASS compact33/62 saved outputs',
 'C3':'evaluate source-native F/J and derived electron/absorption/opacity without IVP',
 'C4':'bounded offline source-replay and target fixtures, no full solver changes',
 'C5':'actual Python and native model physical outputs, 127 Git postprocess',
 'C6':'13/13 tests PASS; test-only charge neutral JVP RED repaired',
 'C7':'mass, charge, nonnegative, isotope, photo-RR/DR, τ/H physical checks',
 'C8':'source-selected reproducibility and 25 matched stored-shear contrasts; no speed claims',
 'C9':'real different-author code-review dispatch absent',
 'C10':'Git+Drive checkpoints, ZIP, held production and executable next handoff'}
scoped=0;allst=[]
for s in list(f'R{i}' for i in range(1,11))+list(f'C{i}' for i in range(1,11)):
 status='NOT_RUN_INDEPENDENT' if s in ('R6','C9') else 'SCOPED_COMPLETE'
 if status=='SCOPED_COMPLETE':scoped+=1
 v={"schema":"vigilode.bass21.actual-physics.stage-checkpoint.v1","date":"2026-10-29","stage":s,"status":status,"scoped_completed_so_far":scoped,"stage_total":20,"stage_completion_fraction":f'{scoped}/20',"source_owner":"BASS PR234@cc602e209e7deeb511ab2a1f3b2268d55206c43c","actual_work":stage_desc[s],"verified_science":{"saved_endpoint_cases":127,"source_native_full_FJ_cases":5,"matched_physical_histories":25,"new_IVPs":0,"targeted_tests":"13/13 PASS","native_independent_R6_C9":"NOT_RUN"},"DAG_at_this_stage":{"VIGILODE_original":"7/23","N03_historical":"17/20 scoped","REI_central":"28/88","REI_working_separate":"33/88","successor_admitted":"0/12","new_auxiliary_atlas":"1/1 science scoped (review HOLD)","production":"HOLD"},"publication_note":"post-execution retrospective stage receipt; separate Git/Drive ACK to be recorded in final receipt; not 20 contemporaneous science-time ZIP archives"}
 (sd/(s+'.json')).write_text(json.dumps(v,indent=2,ensure_ascii=False)+'\n');allst.append({'stage':s,'status':status,'scoped_progress':f'{scoped}/20'})
(root/'STAGE_LEDGER.json').write_text(json.dumps({'schema':'vigilode.bass21.physical-stage-ledger.v1','stages':allst,'scoped_complete':scoped,'total':20,'R6_independent':'NOT_RUN','C9_independent':'NOT_RUN','auxiliary': '1/1 scoped','original':'7/23','REI':'28/88','successor':'0/12'},indent=2)+'\n')
print('stage files',len(allst),'scoped',scoped)
