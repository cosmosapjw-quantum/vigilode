#!/usr/bin/env python3
"""Independently replay 50 cases from immutable parent archive and inspect typed identity."""
import pathlib,sys,hashlib,zipfile,subprocess,tempfile,json,os,numpy as np
root=pathlib.Path(__file__).resolve().parent

def h(p):
 with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

manifest=json.loads((root/'FILE_MANIFEST.json').read_text())
for name,want in manifest['files'].items():
 p=root/name
 if not p.is_file() or h(p)!=want:raise ValueError(f'FILE_SHA_MISMATCH {name}')
print('FILE_MANIFEST_PASS',len(manifest['files']))
p=root/'parent/VIGILODE_BASS21_COUPLED_THOMSON_20261030_FULL_SOURCE.zip'
if h(p)!='ff17ce0c5883de8363898e83ebcb8b30322d35402d0cdbe4c678437493f6103a':raise ValueError('PARENT_DRIFT')
for zfile in [p]:
 with zipfile.ZipFile(zfile) as z:
  if z.testzip() is not None:raise ValueError('BAD_PARENT_CRC')
print('PARENT_SOURCE_BYTES_AND_CRC_PASS')
with tempfile.TemporaryDirectory() as temp:
 target=pathlib.Path(temp)/'regen'
 env=os.environ.copy();env.update(OPENBLAS_NUM_THREADS='1',OMP_NUM_THREADS='1',MKL_NUM_THREADS='1')
 call=[sys.executable,str(root/'code/radiation_stress_atlas.py'),'--parent-zip',str(p),'--out',str(target)]
 subprocess.run(call,check=True,env=env,stdout=subprocess.DEVNULL)
 subprocess.run([sys.executable,str(root/'code/export_physics_csv.py'),'--results',str(target)],check=True,env=env,stdout=subprocess.DEVNULL)
 with np.load(root/'results/STRESS_50x193x6_SAME_OWNER.npz',allow_pickle=False) as a, np.load(target/'STRESS_50x193x6_SAME_OWNER.npz',allow_pickle=False) as b:
  assert a.files==b.files
  for k in a.files:
   if a[k].dtype.kind in 'fi':
    gap=float(np.max(abs(a[k]-b[k])))
    if gap>2e-14:raise ValueError(f'NUMERIC_REPLAY_DIFFERENCE {k} {gap}')
    print('REPLAY_NUMERIC_PASS',k,gap)
   elif not np.array_equal(a[k],b[k]):raise ValueError(f'METADATA_SOURCE_MISMATCH {k}')
 for p2 in ['ENDPOINT_50_STRESS.csv','MATCHED_25_STRESS.csv','FULL50_NUMERIC_AUDIT.csv','FULL50_EPOCH_MULTIPOLE_ROWS.csv','MATCHED25_EPOCH_MULTIPOLE_ROWS.csv','FULL50_EPOCH_STRESS.csv','SCIENCE_SUMMARY.json']:
  if h(root/'results'/p2)!=h(target/p2):raise ValueError('TEXT_OUTPUT_SHA_DIFFERENCE '+p2)
 print('CSV_AND_JSON_REPLAY_PASS',7)
 s=json.loads((root/'results/SCIENCE_SUMMARY.json').read_text())
 assert s['new_deltaJ_rows']==57900 and s['paired_deltaJ_rows']==28950 and s['new_ivps']==0
 assert s['max_energy_work_gap_relative']<5e-4 and s['max_angular_moment_difference']<1e-14
 print('PHYSICS_SCOPE_50x193x6_PASS',s['max_energy_work_gap_relative'],s['max_angular_moment_difference'])
print('ALL_SCOPED_CHECKS_PASS')
