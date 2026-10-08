"""PRED vs the uniform set-point rival and vs I, per h0 seed (1e-6, 1e-5, 1e-4) at matched accuracy (JVP)."""
import sys; sys.path.insert(0, '.')
import ana, numpy as np
SFX = {'1e-6': '', '1e-5': 's5', '1e-4': 's4'}
for pb in ('hires', 'robertson', 'vdp', 'bruss50'):
    d = ana.load(pb); Es = ana.egrid_for(pb, d)
    for a, b in (('C3P', 'A3'), ('R3U', 'A3'), ('C3P', 'R3U'), ('A3', 'A0'), ('LUP', 'LU')):
        cells = []
        fr_all, ch_all = [], []
        for h0, s in SFX.items():
            wk = 'jvp' if a != 'LUP' else 'att'
            m = ana.matched(d, a + s, b + s, wk, Es)
            fs, cs, nin, nx = ana.summarize_matched(m)
            fr = [r for (E, r, x, c) in m if r and not x]; ch = [c for (E, r, x, c) in m if c]
            fr_all.append(ana.gm(fr)); ch_all.append(ana.gm(ch))
            cells.append(f'h0={h0}: {fs} / {cs}')
        med_f = np.median([x for x in fr_all if x]); med_c = np.median([x for x in ch_all if x])
        print(f'{pb:9s} {a:4s}/{b:4s} [{"att" if a=="LUP" else "jvp"}] median over seeds frontier {med_f:.2f} cheapest {med_c:.2f} | ' + ' | '.join(cells))
    # rejections
    for a in ('A0', 'A3', 'C3P', 'R3U'):
        rj = []
        for h0, s in SFX.items():
            rows = ana.arm_rows(d, a + s)
            rj.append(sum(o['rej'] for o in rows if ana.ok(o)) / max(1, sum(o['att'] for o in rows if ana.ok(o))))
        print(f'{pb:9s} {a:4s} rejected fraction over the ladder per seed: ' + ' '.join(f'{x:.3f}' for x in rj))
