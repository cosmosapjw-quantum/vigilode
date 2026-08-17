#!/usr/bin/env python3
from __future__ import annotations
import hashlib, json, math
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

ROOT=Path(__file__).resolve().parents[1]
SHARDS=ROOT/'results/attempt_shards'
OUT=ROOT/'results/attempt_causality'
PLOTS=ROOT/'plots'
CONTRACT=ROOT/'contracts/ATTEMPT_CAUSAL_TRACE_CONTRACT.json'
V25=ROOT.parent/'generic_policy_redesign_v25' if False else Path('research/generic_policy_redesign_v25')
THRESHOLD=0.012790399606947056
POLICIES={'v25-frozen':1,'k2-development-diagnostic':2,'safety-first-development-diagnostic':3}
TINY=1e-300

def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def load_dim(n):
    rows=[]; accepted=[]; traj=[]
    files=sorted(SHARDS.glob(f'n{n}_*.json'))
    if len(files)!=6: raise RuntimeError(f'expected 6 n{n} shards, found {len(files)}')
    for p in files:
        d=json.load(open(p));
        assert d['schema']=='g4-s5b0-rjf-attempt-trace-v1'
        assert d['switching_active'] is False
        rows += d['attempt_rows']; accepted += d['accepted_rows']; traj += d['trajectories']
    return (pd.DataFrame(rows).sort_values(['trajectory_id','attempt_index']).reset_index(drop=True),
            pd.DataFrame(accepted).sort_values(['trajectory_id','step_index']).reset_index(drop=True),
            pd.DataFrame(traj).sort_values('trajectory_id').reset_index(drop=True))

def add_feature(acc):
    a=acc.copy(); a['log_error']=np.log10(np.maximum(a.rodas_embedded_error.astype(float),TINY))
    two=a.groupby('trajectory_id').log_error.shift(2)
    a['two_step_error_drop']=-(a.log_error-two)
    return a

def parity_against_v25(n, acc):
    shard_dir = Path(
        "research/generic_policy_redesign_v25/results/"
        + ("calibration128_shards" if n == 128 else "crossdim512_shards")
    )
    old_rows = []
    files = sorted(shard_dir.glob("*.json"))
    if len(files) != 6:
        raise RuntimeError(f"n{n}: expected six v2.5 raw shards, found {len(files)}")
    shard_hashes = {}
    for path in files:
        payload = json.load(open(path))
        old_rows.extend(payload["rows"])
        shard_hashes[path.name] = sha(path)
    old = pd.DataFrame(old_rows).sort_values(["trajectory_id", "step_index"]).reset_index(drop=True)
    keys = ["trajectory_id", "family", "dimension", "rtol", "step_index"]
    fields = [
        "t_start", "h", "transition_level", "rodas_embedded_error",
        "rodas_rhs_evaluations", "rodas_jvp_vectors", "rodas_linear_matvecs",
    ]
    if not acc[keys].equals(old[keys]):
        raise RuntimeError(f"n{n} accepted keys drift")
    exact = {
        f: bool(np.array_equal(acc[f].to_numpy(), old[f].to_numpy()))
        for f in fields
    }
    if not all(exact.values()):
        raise RuntimeError(f"n{n} accepted numeric drift against raw v2.5 shards {exact}")
    return {
        "exact": True, "rows": len(acc), "fields": exact,
        "v25_raw_shard_sha256": shard_hashes,
    }

def frozen_probe_decisions(n):
    path = Path(
        "research/generic_policy_redesign_v25/results/"
        + (
            "calibration128_analysis/SELECTED_POLICY_PROBES.csv"
            if n == 128
            else "crossdim512_analysis/FROZEN_POLICY_PROBES.csv"
        )
    )
    probes = pd.read_csv(path)
    return {(str(r.trajectory_id), int(r.decision_step)) for r in probes.itertuples()}, sha(path)

def replay_events(n, attempts, accepted, policy, k, frozen_decisions=None):
    acc=add_feature(accepted)
    events=[]
    for tid,g in acc.groupby('trajectory_id',sort=False):
        g=g.sort_values('step_index'); at=attempts[attempts.trajectory_id==tid].sort_values('attempt_index')
        streak=0; latched=False
        for rec in g.to_dict('records'):
            v=rec['two_step_error_drop']
            sig=v is not None and math.isfinite(float(v)) and float(v)>=THRESHOLD
            if not sig:
                streak=0; latched=False; continue
            if latched: continue
            streak += 1
            if streak < k: continue
            latched = True
            if frozen_decisions is not None and (str(tid), int(rec['step_index'])) not in frozen_decisions:
                continue
            # Locate the accepted attempt that produced this committed step.
            producer=at[(at.accepted==True)&(at.accepted_steps_before==int(rec['step_index']))]
            if len(producer)!=1: raise RuntimeError(f'producer ambiguity {tid} step {rec["step_index"]}: {len(producer)}')
            prod=producer.iloc[0]
            nxt=at[at.attempt_index>int(prod.attempt_index)].head(1)
            if nxt.empty: continue
            target=nxt.iloc[0]
            # Eventually accepted next step on the accepted-grid replay.
            future=at[(at.attempt_index>int(prod.attempt_index))&(at.accepted==True)].head(1)
            if future.empty: continue
            final=future.iloc[0]
            events.append({
                'dimension':n,'policy':policy,'trajectory_id':tid,'family':rec['family'],
                'decision_accepted_step':int(rec['step_index']),'feature_value':float(v),
                'producer_attempt_index':int(prod.attempt_index),
                'runtime_target_attempt_index':int(target.attempt_index),
                'runtime_target_h':float(target.h),'runtime_target_accepted':bool(target.accepted),
                'runtime_target_error_norm':None if pd.isna(target.error_norm) else float(target.error_norm),
                'accepted_grid_target_attempt_index':int(final.attempt_index),
                'accepted_grid_target_h':float(final.h),
                'rejections_before_accepted_grid_target':int(final.attempt_index-target.attempt_index),
                'accepted_h_over_first_proposed_h':float(final.h)/float(target.h),
            })
    return pd.DataFrame(events)

def main():
    OUT.mkdir(parents=True,exist_ok=True); PLOTS.mkdir(parents=True,exist_ok=True)
    contract=json.load(open(CONTRACT)); assert contract['schema']=='gvjf-policy-redesign-v26-attempt-causal-trace-contract-v1'
    all_events=[]; summaries=[]; family=[]
    for n in [128,512]:
        attempts,acc,tr=load_dim(n)
        assert tr.success.all(); assert len(attempts)==int(tr.attempts.sum()); assert int((~attempts.accepted).sum())==int(tr.rejected_steps.sum())
        parity = parity_against_v25(n, acc)
        frozen_decisions, frozen_probe_sha = frozen_probe_decisions(n)
        for name, k in POLICIES.items():
            # v25-frozen uses the exact budget-filtered probe decisions that were
            # frozen before the N=512 cross-dimension replay.  The k=3 line is
            # explicitly diagnostic and replays the persistence event only.
            decisions = frozen_decisions if name == "v25-frozen" else None
            ev = replay_events(n, attempts, acc, name, k, decisions)
            all_events.append(ev)
            rejected=int((~ev.runtime_target_accepted).sum()) if len(ev) else 0
            summaries.append({
                'dimension':n,'policy':name,'k':k,'events':len(ev),
                'events_targeting_rejected_first_attempt':rejected,
                'rejected_target_fraction':rejected/len(ev) if len(ev) else 0.0,
                'max_rejections_before_accepted_grid_target':int(ev.rejections_before_accepted_grid_target.max()) if len(ev) else 0,
                'median_accepted_h_over_first_proposed_h':float(ev.accepted_h_over_first_proposed_h.median()) if len(ev) else 1.0,
                'min_accepted_h_over_first_proposed_h':float(ev.accepted_h_over_first_proposed_h.min()) if len(ev) else 1.0,
                'accepted_rows_exact_v25': parity['exact'],
                'frozen_probe_decision_sha256': frozen_probe_sha if name == 'v25-frozen' else None,
                'total_r_rejected_attempts':int(tr.rejected_steps.sum()),
            })
            if len(ev):
                f=ev.groupby('family').agg(events=('family','size'),rejected_targets=('runtime_target_accepted',lambda x:int((~x).sum()))).reset_index()
                f['dimension']=n; f['policy']=name; family.append(f)
    events=pd.concat(all_events,ignore_index=True); summary=pd.DataFrame(summaries); fam=pd.concat(family,ignore_index=True)
    events.to_csv(OUT/'EVENT_TO_ATTEMPT_ROWS.csv',index=False); summary.to_csv(OUT/'EVENT_CAUSALITY_SUMMARY.csv',index=False); fam.to_csv(OUT/'EVENT_CAUSALITY_FAMILY.csv',index=False)
    # Plot fraction of events whose first runtime target is rejected.
    fig,ax=plt.subplots(figsize=(7.2,4.7)); x=np.arange(2); width=.36
    for i,(name,g) in enumerate(summary.groupby('policy',sort=False)):
        gg=g.set_index('dimension').loc[[128,512]]
        ax.bar(x+(i-.5)*width,gg.rejected_target_fraction,width,label=name)
    ax.set_xticks(x,['N=128','N=512']); ax.set_ylabel('Fraction of raw events whose first next R attempt rejects'); ax.set_ylim(0,1); ax.grid(True,axis='y',alpha=.25); ax.legend(); ax.set_title('Accepted-grid replay misses speculative work after rejected next proposals'); fig.tight_layout(); fig.savefig(PLOTS/'ATTEMPT_CAUSALITY_REJECTED_TARGETS.png',dpi=180); plt.close(fig)
    # h correction plot for only affected events.
    affected=events[~events.runtime_target_accepted]
    fig,ax=plt.subplots(figsize=(7.2,4.7))
    if len(affected):
        for (n,name),g in affected.groupby(['dimension','policy']):
            ax.scatter(g.runtime_target_h,g.accepted_grid_target_h,label=f'N={n} {name}',alpha=.8)
        lo=min(affected.runtime_target_h.min(),affected.accepted_grid_target_h.min()); hi=max(affected.runtime_target_h.max(),affected.accepted_grid_target_h.max()); ax.plot([lo,hi],[lo,hi],linestyle='--')
    ax.set_xlabel('First runtime-proposed h'); ax.set_ylabel('Eventually accepted-grid h'); ax.set_title('Rejected next proposals change the interval an event would actually probe'); ax.grid(True,alpha=.25); ax.legend(); fig.tight_layout(); fig.savefig(PLOTS/'ATTEMPT_CAUSALITY_H_SHIFT.png',dpi=180); plt.close(fig)
    result={
      'schema':'gvjf-policy-redesign-v26-attempt-causality-result-v1',
      'contract_sha256':sha(CONTRACT),
      'new_solver_runs':12,'e_k_evaluations':0,'prefix_evaluations':0,'active_switching':False,'n2048_executed':False,
      'summary':summary.to_dict('records'),
      'primary_verdict':'accepted-grid runtime causality fails if any exact frozen-policy probe decision targets a rejected first next attempt; k=3 is diagnostic only',
      'causality_failure_observed':bool((summary.events_targeting_rejected_first_attempt>0).any()),
    }
    (OUT/'RESULT_SUMMARY.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__': main()
