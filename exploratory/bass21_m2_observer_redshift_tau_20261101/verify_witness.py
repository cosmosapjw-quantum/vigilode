"""Bounded Git witness check. It does not regenerate the source-full science."""
import csv,math
from pathlib import Path
root=Path(__file__).resolve().parent
with (root/'results'/'GIT_WITNESS_12.csv').open() as f:rows=list(csv.DictReader(f))
assert len(rows)==12
assert len({r['case'] for r in rows})==4
for case in {r['case'] for r in rows}:
    rr={float(r['mu_obs']):r for r in rows if r['case']==case}
    assert set(rr)=={-1.,0.,1.}
    assert all(math.isfinite(float(r['tau_obs_redshift'])) for r in rr.values())
    assert abs(float(rr[-1.]['tau_obs_redshift'])-float(rr[1.]['tau_obs_redshift']))<1e-13
    if case.startswith('s04_'):assert float(rr[1.]['tau_obs_redshift'])<float(rr[0.]['tau_obs_redshift'])
print('GIT_WITNESS_12_ROWS_PARITY_AND_AXIS_SIGN_PASS; FULL_SOURCE_REPLAY_NOT_RUN')
