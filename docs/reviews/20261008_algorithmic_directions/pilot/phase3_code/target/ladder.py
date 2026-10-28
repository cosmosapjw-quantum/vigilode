"""Run arms x rtol ladder on one problem; append one JSON line per run to res/<problem>.jsonl."""
import json, sys, time, numpy as np
sys.path.insert(0, '.')
import rep, tprobs, arms
prob_name = sys.argv[1]; arm_names = sys.argv[2].split(','); rtols = [float(x) for x in sys.argv[3].split(',')]
tag = sys.argv[4] if len(sys.argv) > 4 else ''
mk = tprobs.PROBLEMS[prob_name]
for rtol in rtols:
    for an in arm_names:
        a, tg = arms.arm(an); p = mk()
        t0 = time.time()
        try:
            r = rep.integrate(p, rtol, a, tgt=tg, record=(tag == 'rec'))
        except Exception as e:
            print(prob_name, an, rtol, 'ERROR', e, flush=True)
            with open(f'res/{prob_name}.jsonl', 'a') as fo:
                fo.write(json.dumps(dict(prob=prob_name, arm=an, rtol=rtol, error=str(e))) + '\n')
            continue
        recs = r.pop('recs', None)
        o = {k: (v.tolist() if isinstance(v, np.ndarray) else v) for k, v in r.items()}
        o.update(prob=prob_name, arm=an, rtol=rtol, sec=time.time()-t0,
                 flops=rep.flops(p, r, a['kind'], a.get('form', 'scaled')),
                 flops_fd=rep.flops(p, r, a['kind'], a.get('form', 'scaled'), jvp_model='fd'))
        if recs is not None:
            R = np.array([x['res'] for x in recs]); Bw = np.array([x['bw'] for x in recs])
            o['res_med'] = np.median(R, axis=0).tolist(); o['res_p90'] = np.percentile(R, 90, axis=0).tolist()
            o['res_max'] = R.max(axis=0).tolist(); o['By_med'] = float(np.median([x['By'] for x in recs]))
            o['By_sum'] = float(np.sum([x['By'] for x in recs])); o['bw_med'] = np.median(Bw, axis=0).tolist()
        with open(f'res/{prob_name}{tag}.jsonl', 'a') as fo:
            fo.write(json.dumps(o) + '\n')
        print(f"{prob_name} {an:10s} rtol={rtol:g} att={r['att']} acc={r['acc']} rej={r['rej']} lf={r['lin_fail']} jvp={r['jvp']} "
              f"JVP/acc={r['jvp']/r['acc']:.1f} dots={r['dots']} floor={r['floor_bind']} rbind={r['roundoff_bind']} [{time.time()-t0:.0f}s]", flush=True)
