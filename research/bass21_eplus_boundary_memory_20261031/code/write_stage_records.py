#!/usr/bin/env python3
"""Retrospective stage-evidence index, not twenty independent experiments."""
from __future__ import annotations
import pathlib,json
ROOT=pathlib.Path(__file__).resolve().parents[1]
DATA={
'R1':('Scope, sources and locked permissions','THEORY_AND_INTERPRETATION_KO.md:50 original cases ×193 x, 4 cutoffs, no source IVP'),
'R2':('Literature and source reuse','Challinor 2000, Pontzen-Challinor 2007, Pitrou 2009, exact PR145/PR143 restored'),
'R3':('Geometry, screen and ownership consistency','(-,+,+,+), plus E convention, x, g, tau, original BASS owner not VigilODE'),
'R4':('Two competing physical diagnostics','Signed attenuated M and unsigned G source-only kernel, reject total-error interpretation'),
'R5':('Constructive causal Volterra formula','Direct M=exp(-tau) ∫exp(tau)F and |M|≤G; original bprime high-rank operator'),
'R6':('Self-adversarial audit of physics claim','HOSTILE_REVIEW_KO.md; distinct-author R6 NOT_RUN'),
'R7':('Discriminating physical calculation','38,600 signed +38,600 unsigned real original-source values; 25 matched contrasts'),
'R8':('Physical claim and release decision','Source-frozen only; no full E_l truncation certificate; production HOLD'),
'R9':('Math-to-code mapping','Flux bprime*AE*E from saved source, tau damping, independent recurrence and Simpson'),
'R10':('Checkpoint and handoff','HANDOFF_NEXT_THREAD_KO.md, source-inclusive full ZIP and manifest'),
'C1':('Code baseline and acceptance','PR145 corrected SHA, original source exact 50/193, no IVP, strict identity'),
'C2':('Map old arrays to source code','PR145 E_l0_plus + boundary_flux; PR143 tau, g, bprime; case/x exact'),
'C3':('Causal quadrature formula','composite source-grid weighted trapezoid for M and G; no ODE solver'),
'C4':('Bounded compute/backup routing','single 50×193×4 CPU run, separated binary backups and Git source-free mirror'),
'C5':('Implementation and physical artifacts','code/compute_boundary_memory.py; real NPZ, CSV, two plots'),
'C6':('Execution and regression tests','tests/test_memory.py 19/19 PASS; original/parent sha; 38,600 flux identities'),
'C7':('Physical checks and counterexamples','|M|<=G, zero source, positive g, matched 25 pairs and unresolved s00'),
'C8':('Grid quadrature and output replay','independent recursive trapezoid within 2.3e-26; Simpson gap 3.14e-16; 9/9 byte replay'),
'C9':('Same-author hostile implementation review','HOSTILE_REVIEW_KO.md; distinct-author C9 NOT_RUN'),
'C10':('Draft publication and handoff','archive + source-verified restore; no merge/force-push or production promotion')}
assert len(DATA)==20
out=ROOT/'checkpoints'
for code,(task,evidence) in DATA.items():
    obj={'schema':'vigilode.bass21.boundary_memory_stage_v1','stage':code,'task':task,'evidence':evidence,
         'status':('SELF_ADVERSARIAL_ONLY_DISTINCT_AUTHOR_NOT_RUN' if code in ('R6','C9') else 'COMPLETE_SCOPED'),
         'research_unit':'BASS21_LRS_HIGH_E_BOUNDARY_MEMORY_SOURCE_FROZEN',
         'source_sha256':'c7626d6f7e01e578e4e9825e60ea8e1631e6ec79cc535867d95fc690b0e0c564',
         'source_history':'50/50','source_epochs':193,'vigilode_dag':'7/23','admitted_successor':'0/12',
         'dag_delta':0,'backup_status_at_local_checkpoint':'local durable file; intended combined create-only Library/Drive upload and independent Git draft publication on closeout',
         'original_new_ivps':0,'production':'HOLD'}
    (out/(code+'.json')).write_text(json.dumps(obj,ensure_ascii=False,indent=2,sort_keys=True)+'\n')
print('STAGES_RECORDED='+str(len(DATA))+'/20; no independent author')
