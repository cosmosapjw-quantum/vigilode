#!/usr/bin/env python3
"""Independent comparison of original N4 Python/standalone native reference and
2026-10-29 native integrator callback-bound source, using recorded actual stdout.
Does NOT claim bound on physical global integration error.
"""
import ast, re, json, pathlib
root=pathlib.Path(__file__).resolve().parents[1]
a=(root/'n4c_original_independent_rodas8.log').read_text()
b=(root/'n4c_native_test_outputparity.log').read_text()
old={}
for n,h,x in re.findall(r'N4_RUST_RAWU8 n=(\d+) h=(\d+\.\d+) .*?output6=(\[[^\]]+\])',a):
    old[(int(n),float(h))]=ast.literal_eval(x)
native={}
for n,h,x in re.findall(r'N4C_NATIVE_OUTPUT n(\d+)_h([\d.e-]+) output6=(\[[^\]]+\])',b):
    native[(int(n), 0.25 if h=="025" else float(h))]=ast.literal_eval(x)
assert (12,.25) in old and (12,.25) in native, (old,native)
assert (12,0.) in old and (12,1e-8) in native
pairs=[((12,.25),(12,.25)),((12,0.),(12,1e-8))]
rows=[]
for ia,ib in pairs:
    diffs=[abs(x-y) for x,y in zip(old[ia],native[ib])]
    rows.append({'reference_case':str(ia),'native_case':str(ib),'max_first6_abs_diff':max(diffs),'all_six_under_2e-13':all(x<2e-13 for x in diffs),'n_values':len(diffs)})
assert all(x['all_six_under_2e-13'] for x in rows)
out={'scope':'first 6 real components of original raw-U direct independent reference versus installed native sequential_step','cases':rows,'status':'2/2_CASES_PASS_SCOPED','reference_output_exact_from_log':True,'full_state_parity':'NOT_RUN'}
print(json.dumps(out,indent=2))
(root/'stage'/'native_vs_reference_results.json').write_text(json.dumps(out,indent=2)+'\n')
