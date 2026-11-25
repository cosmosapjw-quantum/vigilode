"""MathHarness generates combinations and guides measured-cost selection."""
import json,random,sys,itertools
from pathlib import Path
from mathharness_general.search.primitives import bo_maximize
P=Path(__file__).resolve().parent
def dump(n,v):(P/n).write_text(json.dumps(v,indent=2,sort_keys=True,allow_nan=False)+'\n')
def generate():
    routes=json.loads((P/'HARNESS_ROUTES.json').read_text())['routes'];ids={r['candidate_id']:r for r in routes}
    mapping={'R001':'interval state and reset enclosure','R008':'interval Newton on source guard','R021':'affine residual certificate','R042':'original source and reset clock authority','R045':'duplicate-root/anti-Zeno counterexamples','R049':'dictionary Green transport and root hints'}
    candidates=[
        ('Represented',['retro_event_poly','native accepted control'],'represented-only'),
        ('Carry',['affine_event_certificate','retro_causal::flow','retro_physical_event::contract_root_once'],'physical'),
        ('LegacyFlow',['retro_green::RetroTable','retro_causal::flow/gain','retro_physical_event::contract_root_once'],'physical'),
        ('DictionaryFlow',['retro_dictionary','retro_green::RetroTable','retro_causal::flow/gain','retro_physical_event::contract_root_once'],'physical'),
        ('Hinted',['retro_dictionary','retro_causal::flow/gain','retro_event_poly','retro_physical_event::contract_root_once'],'physical'),
        ('Transport',['retro_dictionary','retro_causal::flow/gain','retro_green::RetroTable','correlated reset transport'],'physical'),
        ('Combined',['affine_event_certificate','retro_dictionary','retro_causal::flow/gain','retro_event_poly','retro_physical_event::contract_root_once','correlated reset transport'],'physical'),
        ('Adaptive',['retro_portfolio','same-contract full attempts charged'],'physical')]
    dump('CANDIDATES.json',{'routes':[{'id':k,'modules':v,'contract':c} for k,v,c in candidates],'catalog_mappings':[{**ids[k],'mapping':v} for k,v in mapping.items()],'non_claim':'represented control excluded from physical selection; prepared RHS and WARP not required by scalar fresh epochs'})
    print('generated',len(candidates),'candidate routes')
def select():
    d=json.loads((P/'TRAINING_AUDIT.json').read_text());c=[v for v in json.loads((P/'CANDIDATES.json').read_text())['routes'] if v['contract']=='physical'];out={}
    for g in sorted({r['group'] for r in d['rows']}):
        out[g]={}
        for key,tol in [('1e-6',1e-6),('1e-9',1e-9)]:
            vals=[]
            for v in c:
                rs=[r for r in d['rows'] if r['group']==g and r['candidate']==v['id'] and r['time_tol']==tol];assert rs
                vals.append({'route':v['id'],'qualified':all(r['qualified'] for r in rs),'cases':len(rs),'pass_count':sum(r['qualified'] for r in rs),'median_cost_ns':sum(r['median_ns'] for r in rs),'charged_rhs':sum(r.get('charged_rhs',0) for r in rs),'max_bracket':max((r.get('max_width',0) for r in rs),default=0)})
            good=[v for v in vals if v['qualified']]
            best=min(good,key=lambda v:(v['median_cost_ns'],v['route'])) if good else None
            visited=[]
            def obj(x):
                v=vals[min(len(vals)-1,max(0,round(x[0])))];visited.append(v['route'])
                return -float(v['median_cost_ns']) if v['qualified'] else -1e30+v['pass_count']*1e20
            x,score,n=bo_maximize(obj,[0.],[float(len(vals)-1)],random.Random(1729),n_init=7,n_iter=14,x0=[0.])
            bo=vals[min(len(vals)-1,max(0,round(x[0])))]
            out[g][key]={'selected':best['route'] if best else None,'bo_proposal':bo['route'] if bo['qualified'] else None,'matches_exhaustive':(best and best['route']==bo['route']) or (best is None and not bo['qualified']),'calls':n,'visited':visited,'routes':vals}
            print(g,key,out[g][key]['selected'],[(v['route'],v['pass_count'],v['cases']) for v in vals])
    dump('SELECTION.json',{'groups':out,'method':'all required physical bounds, then measured whole-run cost including reset and rejected attempts; GP-UCB measured finite lookup not fewer executions','holdout':'NOT_RUN'})
if __name__=='__main__':{'generate':generate,'select':select}[sys.argv[1]]()
