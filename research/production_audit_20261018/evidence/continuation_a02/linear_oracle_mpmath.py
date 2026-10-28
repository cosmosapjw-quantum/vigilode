"""Frozen 120-digit analytic chi=kappa=0 oracle for exact binary64 inputs."""
import json
import math
import struct
from pathlib import Path
import mpmath as mp

mp.mp.dps = 120
xi = [-0.5, 0.0, 0.5]
w = [0.25, 0.5, 0.25]
z = [(0.25, -0.125), (0.125, 0.0625), (-0.0625, 0.25)]
width = 1.0
times = [0.0, 0.03125, 0.09375, 0.15625, 0.25]

def exact(v):
    num, den = v.as_integer_ratio()
    return mp.mpf(num) / den

def bits(v):
    return struct.unpack('>Q', struct.pack('>d', v))[0]

def enclose(x):
    f = float(x)
    y = exact(f)
    if y < x:
        return [f, math.nextafter(f, math.inf)]
    if y > x:
        return [math.nextafter(f, -math.inf), f]
    return [f, f]

rows=[]
for t in times:
    modes=[]
    mean=mp.mpc(0)
    for weight, freq, (re, im) in zip(w, xi, z):
        value = mp.mpc(exact(re), exact(im)) * mp.exp(1j * exact(width) * exact(freq) * exact(t))
        modes.append([enclose(value.real), enclose(value.imag)])
        mean += exact(weight) * value
    rows.append({'t_bits':bits(t),'modes':modes,'mean':[enclose(mean.real),enclose(mean.imag)]})
output={'method':'mpmath 120 dps analytic exp(i width xi t) on exact binary64 rational inputs; adjacent-f64 enclosure','input_bits':{'width':bits(width),'xi':[bits(v) for v in xi],'weights':[bits(v) for v in w],'z':[[bits(re),bits(im)] for re,im in z]},'rows':rows}
path=Path(__file__).with_name('linear_oracle_mpmath.json')
path.write_text(json.dumps(output,indent=2)+'\n')
print('wrote',path,'rows',len(rows),'mpmath',mp.__version__)
