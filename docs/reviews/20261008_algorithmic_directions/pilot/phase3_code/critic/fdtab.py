"""FD-JVP table: per problem/rtol, arms A0f (production FD as calibrated), A0f8s (production form rtol_lin 1e-8 +
FD stall backstop, B1's FD rival), Sf (stack as calibrated, FD), Sfg8 (B1's FD-aware stack), vs exact-JVP S and A0."""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__)); sys.path.insert(0, HERE)
import cana
for pn in sys.argv[1:]:
    d = cana.load(pn); syn = cana.SYN(pn)
    rts = sorted({r for (a, r) in d if a in ('A0f', 'Sfg8')}, reverse=True)
    print(f'== {pn} (err: {"WRMS tol units" if syn else "cw rel / rtol"})')
    for rt in rts:
        cells = []
        for a in ('A0', 'S', 'A0f', 'A0f8s', 'Sf', 'Sfg8'):
            o = d.get((a, rt))
            if o is None: cells.append(f'{a}:-'); continue
            if 'error' in o: cells.append(f'{a}:ERR'); continue
            e = cana.metric(o, syn)/rt
            g = o.get('gap') or {}
            cells.append(f"{a}: e={e:.3g} att={o['att']} lf={o['lin_fail']}{' CAP t=%.3g' % o['t'] if o.get('capped') else ''} "
                         f"F={o['flops_fd']/1e6:.3g}M g={o['guard_q']}/{o['guard_over']}/{o['guard_false']} tp>10={g.get('tp_gt10', '-')}")
        print(f'  {rt:.0e} | ' + ' | '.join(cells))
