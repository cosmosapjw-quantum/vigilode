#!/usr/bin/env python3
"""Independently replay 12 source-owned fixed-observed-redshift tau witnesses.

Requires the byte-locked BASS21 source ZIP, numpy and scipy. This is not a
new BASS/REI initial-value solution or full 3400-row science replay.
"""
from pathlib import Path
import csv, hashlib, io, json, math, sys, zipfile
import numpy as np
from scipy.interpolate import PchipInterpolator
from scipy.optimize import brentq

SOURCE_SHA = '6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
CASES = ('s00_m00_r00','s00_m04_r04','s04_m00_r00','s04_m04_r04')
ROOT = Path(__file__).resolve().parent

def main(source: Path):
    assert hashlib.sha256(source.read_bytes()).hexdigest() == SOURCE_SHA, 'invalid source identity'
    with zipfile.ZipFile(source) as z:
        assert z.testzip() is None
        manifest = json.loads(z.read('results/SOURCE_INPUT_MANIFEST.json'))
        predicted = {}
        for case in CASES:
            raw = z.read('inputs/'+case+'.npz')
            assert hashlib.sha256(raw).hexdigest()==manifest['sources'][case]['sha256']
            with np.load(io.BytesIO(raw),allow_pickle=False) as q:
                x = q['x']; names = [str(v) for v in q['diagnostic_names']]
                d = {name:q['diagnostics'][:,i] for i,name in enumerate(names)}
            b = PchipInterpolator(x,d['b']); t=PchipInterpolator(x,d['tau_T'])
            xe=PchipInterpolator(x,d['ne_m3']/d['nb_m3'])
            temp=PchipInterpolator(x,d['T_K'])
            xo=float(x[-1]); bo=float(b(xo))
            for mu in (-1.,0.,1.):
                def f(v):
                    db=float(b(v)-bo); m2=mu*mu
                    return xo-v+0.5*math.log((1-m2)*math.exp(2*db)+m2*math.exp(-4*db))-math.log(11/3)
                em=brentq(f,float(x[0]),xo,xtol=1e-13,rtol=9e-15)
                predicted[(case,mu)]=(em,float(xe(em)),float(temp(em)),float(t(xo)-t(em)),float(t(xo)-t(math.log(21/11))))
    with (ROOT/'results'/'GIT_WITNESS_12.csv').open() as f:rows=list(csv.DictReader(f))
    assert len(rows)==12
    for row in rows:
        assert float(row['z_target'])==10.
        key=(row['case'],float(row['mu_obs']))
        assert key in predicted
        actual = tuple(float(row[k]) for k in ('emission_x','emission_xe','emission_T_K','tau_obs_redshift','tau_fixed_x_control'))
        assert all(abs(a-b)<4e-12 for a,b in zip(actual,predicted[key])), (key,actual,predicted[key])
    print('SOURCE_OWNED_12_WITNESS_REPLAY_PASS: 12/12; NO_NEW_OWNER_IVP')

if __name__=='__main__':
    main(Path(sys.argv[1]) if len(sys.argv)>1 else ROOT/'source'/'VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip')
