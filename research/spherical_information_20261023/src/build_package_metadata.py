"""Build machine-readable decisions and next implementation contracts from frozen results."""
from pathlib import Path
import json
ROOT=Path(__file__).resolve().parents[1]
def read(p): return json.loads((ROOT/p).read_text())
def write(p,v): (ROOT/p).write_text(json.dumps(v,ensure_ascii=False,indent=2,allow_nan=False)+'\n')
mesh=read('results/mesh_results.json'); fish=read('results/fisher_results.json')
th=read('results/thomson_results.json'); remap=read('results/remap_budget_results.json')
beam=next(r for r in mesh['records'] if r['profile']=='localized_beam' and r['budget_fraction']==.01)
hard=next(r for r in fish['cases'] if r['statistics']=='MB' and r['z']==8 and r['L']==5)
claims=[
 {'id':'SG01','statement':'Conservative weighted cell means have an exact nonnegative Bregman loss and nested refinement telescope.',
  'basis':['derived','numerically checked','implementation-verified'],'scope':'positive supplied finite angular states; scalar contracted thermal entropy',
  'evidence':['analysis/PAPER_I_METHODS_KO.md','src/spherical_mesh.py','results/mesh_results.json'],
  'decision':'adopt_as_reference','remaining':['native full-occupation remap','continuum unresolved error']},
 {'id':'SG02','statement':'A cellwise lower entropy curvature bounds each declared linear observer error.',
  'basis':['derived','numerically checked'],'scope':'finite-grid error relative to supplied fine state; floating tests are not interval certification',
  'evidence':['RESEARCH_REPORT_KO.md#3-엔트로피-손실을-계산하는-적응형-구면-격자','results/mesh_results.json'],
  'decision':'adopt_observer_bound','remaining':['refine directly to observer tolerance','tightness near vacuum']},
 {'id':'SG03','statement':'Energy-only thermal remap changes particle number; joint MB number-energy entropy is strictly convex and preserves both means.',
  'basis':['derived','numerically checked'],'scope':'massless MB joint closure; general quantum/full-f route separately specified',
  'evidence':['analysis/PAPER_I_METHODS_KO.md','evidence/entropy_geometry_wolfram.json','results/mesh_results.json'],
  'decision':'use_joint_or_full_occupation_for_number_clients','remaining':['BE/FD joint reconstruction','native bin-weight mapping']},
 {'id':'SG04','statement':'a <= W/W0 <= b implies preconditioned entropy Hessian condition <= b/a independently of angular cutoff.',
  'basis':['derived','literature-supported','numerically checked','implementation-verified'],
  'scope':'same positive finite quadrature and scores; H0 positive definite; moment inversion rather than arbitrary ODE Jacobian',
  'evidence':['analysis/PAPER_II_METHODS_KO.md','src/fisher_demo.py','results/fisher_results.json'],
  'decision':'adopt_for_closure_prototype','remaining':['matrix-free native implementation','extreme variance triangular-factor path']},
 {'id':'SG05','statement':'Fixed-metric gradient descent is Newton-free and contracts by (b-a)/(b+a) on a certified admissible solution ball.',
  'basis':['derived'],'scope':'uniform Hessian bounds throughout the stated ball and starting point in that ball',
  'evidence':['analysis/PAPER_II_METHODS_KO.md'],
  'decision':'research_candidate','remaining':['construct practical domain certificate','nonlinear closure experiment']},
 {'id':'SG06','statement':'The existing rank-six FVM collision has an exact small-matrix exponential action, including singular capacitance matrices.',
  'basis':['derived','numerically checked','implementation-verified'],
  'scope':'fixed grid, scalar common collision rate and known optical depth; no general coupled-step order claim',
  'evidence':['analysis/THOMSON_LOW_RANK_KO.md','src/thomson_demo.py','results/thomson_results.json'],
  'decision':'highest_priority_native_candidate','remaining':['cache small decomposition','coupled work-precision experiment']},
 {'id':'SG07','statement':'Real transverse coherency Thomson gain closes on six symmetric moments and admits a PSD-preserving exact subflow.',
  'basis':['derived','literature-supported','numerically checked','implementation-verified'],
  'scope':'cold rest-frame elastic shell, circular V=0, quadrature preserving required angular moments',
  'evidence':['analysis/THOMSON_LOW_RANK_KO.md','evidence/polarized_collision_wolfram.json','results/thomson_results.json'],
  'decision':'adopt_scoped_polarized_reference','remaining':['spin/transport coupling','tilt and energy redistribution','circular polarization extension']},
 {'id':'SG08','statement':'Positive mass-preserving maps yield global Hellinger error <= initial error + sum sqrt(local Poisson KL), with bounded-observable control.',
  'basis':['derived','numerically checked','implementation-verified'],
  'scope':'equal-mass finite MB states, column-stochastic propagators; general F-rho/BE/FD or arbitrary RK steps excluded',
  'evidence':['RESEARCH_REPORT_KO.md','src/remap_budget_demo.py','results/remap_budget_results.json'],
  'decision':'adopt_conditional_budget','remaining':['actual time-step positivity contract','nonlinear stability factors']},
 {'id':'SG09','statement':'Massive shell Poisson multipliers give certified cutoff selection; Laguerre alpha=2 orthonormalizes massless MB radial tangent scores.',
  'basis':['derived','numerically checked','implementation-verified'],
  'scope':'known tail/norm bound; velocity <1 for suppression; no radial velocity suppression for photons',
  'evidence':['analysis/PAPER_II_METHODS_KO.md','src/spectral_tools_demo.py','results/spectral_tools_results.json'],
  'decision':'adopt_spectral_reference','remaining':['state-dependent tail controller','native transforms']}
]
write('DECISIONS.json',{'schema':'spherical-information.decisions.v1','date':'2026-10-23','workflow_version':'1.0.0',
 'objective_status':'completed_scoped_theory_and_lightweight_verification','production_solver_status':'not_assessed_by_this_package',
 'inherited_broad_production_status':'HOLD','native_repo_modified':False,'ncp_used':False,
 'claims':claims,'measured_highlights':{
 'beam_same_allowed_budget':{'fraction':.01,'uniform_cells':beam['uniform']['cells'],'adaptive_cells':beam['adaptive']['cells'],'ratio':beam['cell_count_reduction'],'interpretation':'state count, not PDE wall-time'},
 'fisher_hard_case':{k:hard[k] for k in ['raw_condition','whitened_condition','median_solve_speed_ratio']},
 'fisher_iterations':{k:hard['runs'][k]['iterations'] for k in ['plain_cg','fisher_pcg']},
 'mesh_configurations':len(mesh['records']),'fisher_configurations':len(fish['cases']),
 'remap_configurations':len(remap['cases']),'remap_step_checks':sum(c['steps'] for c in remap['cases'])},
 'general_limits':['finite-grid certificate is not continuum PDE certificate','microtimings are shared-host diagnostics','current native source was only read','published unchanged theories were reused']})
tasks=[
 ('SG-N01',0,[],'Cache exact fixed-grid collision action','Existing sphere_photon_fv_support/thomson6_factors; actual B=V^T U',
  ['small-matrix phi1 or verified symmetric decomposition','rank-deficient fallback','stable large/small optical-depth evaluation'],
  ['compare unchanged FVM oracle at same source hash','mass and positivity at stated float tolerance','same coupled observer error before speed comparison']),
 ('SG-N02',0,[],'Define native conservative information-remap state','Photon bin contents N, radial state-counting weights, angular cell areas',
  ['full occupation f=N/(nu*Omega) mapping','bin-wise conservative positive remap','joint number-energy fallback for explicit thermal model'],
  ['number and fixed bin-energy moments preserved','FD [0,1] and BE/MB positive domains','rho-only number drift regression remains visible']),
 ('SG-N03',1,[],'Port centered Fisher blocks into moment inversion','Entropy closure score and quadrature interface',
  ['centered variance','triangular whitening application instead of ill-conditioned explicit inverse','finite-grid susceptibility certificate'],
  ['original-coordinate residual and known-solution error','isotropic identity and quantum-domain tests','setup-inclusive timing with slower cases retained']),
 ('SG-N04',1,['SG-N02'],'Connect accepted-step remap and observer-tolerance refinement','Integrator accepted-step hook, grid topology and caches',
  ['transactional dimension change','all cache/norm identities refreshed','refinement until declared observer bounds met'],
  ['coarse/fine/coarse invariants','same initial fine data and observer budget across methods','remap budget accumulated separately from ODE error']),
 ('SG-N05',1,['SG-N01'],'Benchmark transport/collision coupling order','Existing native stiff/nonstiff driver plus exact subflow',
  ['explicit choice of Strang2 or justified higher-order IMEX/exponential coupling','time-dependent optical-depth treatment','matter-feedback handling'],
  ['step-halving verifies declared combined order','stiff and nonstiff work-precision','full wall/setup/remap times and physical ledgers']),
 ('SG-N06',2,['SG-N03'],'Test Newton-free fixed Fisher iteration','Certified neighborhood around feasible entropy closure',
  ['uniform a,b throughout admissible ball','fallback when certificate weakens','residual-only iteration'],
  ['measured contraction versus proven q','no disguised Newton solve','failure preserves feasibility/domain information']),
 ('SG-N07',2,[],'Port shellwise cutoff and Laguerre analysis basis','Massive dressed fields with known coefficient/tail bounds',
  ['v=0/v=1 branches','L2 and Linfinity bounds','positive inverse-temperature margin'],
  ['fixed target error at reduced evaluation count','tail provenance recorded','massless photon retains correct fallback']),
 ('SG-N08',2,['SG-N01','SG-N04','SG-N05'],'End-to-end spherical work-precision comparison','Local host only; existing application initial conditions',
  ['anisotropy/beam/vacuum/optical-depth/remap-frequency matrix','unbiased timing including setup','target detector/energy/number and polarization observables'],
  ['raw failures and slower cases preserved','same global observer error','production decision based on full client evidence, not prototype count'])
]
write('NEXT_STEPS.json',{'schema':'spherical-information.next-steps.v1','source_reference':'14c74978b93339a79d7228a324fa77552dae548e',
 'execution_location':'local host; no NCP','tasks':[{'id':i,'priority':p,'depends_on':d,'title':title,'target':target,'deliverables':deliver,'acceptance':accept,'status':'ready' if not d else 'pending_dependencies'} for i,p,d,title,target,deliver,accept in tasks]})
refs=['evidence/INPUTS.json','analysis/PAPER_I_METHODS_KO.md','analysis/PAPER_II_METHODS_KO.md','RESEARCH_REPORT_KO.md',
 'analysis/INDEPENDENT_REVIEW_KO.md','results/mesh_results.json','DECISIONS.json','NEXT_STEPS.json','HANDOFF_KO.md']
theory=[(1,refs[0]),(2,refs[1]),(3,refs[2]),(4,refs[3]),(5,refs[3]),(6,refs[4]),(7,refs[5]),(8,refs[6]),(9,refs[7]),(10,refs[8])]
coding=[(1,refs[0]),(2,refs[3]),(3,refs[2]),(4,refs[7]),(5,'src/spherical_mesh.py'),(6,'results/mesh_validation.json'),(7,'results/thomson_results.json'),(8,'results/fisher_results.json'),(9,refs[4]),(10,refs[8])]
write('WORKFLOW.json',{'schema_version':1,'workflow_version':'1.0.0','task':'both','unit_id':'spherical-information-20261023',
 'scope':'Complete theory derivations and lightweight prototype verification; native port and coupled benchmarks are follow-up tasks.',
 'status':'complete','theory_stages':[{'id':f'R{i}','status':'complete','evidence':e} for i,e in theory],
 'coding_stages':[{'id':f'C{i}','status':'complete','evidence':e} for i,e in coding],
 'reuse_policy':'Unchanged draft and existing solver results reused; new discrete constructions and changed implementations checked.',
 'failure_classification':{'physical':'energy-only remap changes number; remedy derived','numerical':'continuum 0.1 shortcut mismatches finite-cell Gram; actual B used','environment':'SymPy unavailable; exact fractions and Wolfram used','implementation':'no unresolved failing scoped assertion','production':'end-to-end native integration not part of this execution'},
 'storage':'see artifact manifest and separate delivery receipt'})
write('evidence/FIGURE_REVIEW.json',{'schema':'spherical-information.figure-review.v1','reviewer':'primary agent','method':'viewed all three rendered PNG files',
 'files':['figures/mesh_budget.png','figures/fisher_conditioning.png','figures/collision_and_remap.png'],
 'result':'pass_scoped','checks':['labels readable','same allowed budget distinguished from equal realized error','cell count distinct from solver speed','slower preconditioned cases retained','raw accumulated bound shown even above trivial maximum']})
print('Wrote decisions, eight next tasks, R/C workflow, figure review')
