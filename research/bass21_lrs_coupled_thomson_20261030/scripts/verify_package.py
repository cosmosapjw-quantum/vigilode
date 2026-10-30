#!/usr/bin/env python3
"""Clean-package content identity, source authority and numerical science replay."""
import hashlib,json,pathlib,subprocess,sys,tempfile,zipfile
import numpy as np

root=pathlib.Path(__file__).resolve().parent.parent
manifest=json.loads((root/'FILE_MANIFEST.json').read_text())
for rel,sha in manifest['files'].items():
    p=root/rel
    assert p.exists() and hashlib.sha256(p.read_bytes()).hexdigest()==sha,(rel,'HASH_DRIFT')
print('MANIFEST_SHA256_PASS',len(manifest['files']))
src=root/'source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip'
assert hashlib.sha256(src.read_bytes()).hexdigest()=='6aa610316a117d97744e6f35738e226bff9c4cd1a39d03e22b2d334252616dd8'
with zipfile.ZipFile(src) as z:
    assert z.testzip() is None
print('ORIGINAL_BASS21_SOURCE_SHA_CRC_PASS')
pkg=root/'results/LRS50_CPLD_193x17_POLARIZED_RESPONSE.npz'
with np.load(pkg,allow_pickle=False) as a:
    assert a['Q_coupled'].shape==(50,193,17) and a['deltaI'].shape==(50,193,17)
    assert a['paired_deltaQ'].shape==(25,193,17)
    assert np.all(a['I_full']>0) and np.all(np.abs(a['Q_coupled'])<=a['I_full'])
    assert not np.any(a['Q_coupled'][:,:,16]) and not np.any(a['Q_coupled'][:,0,:])
print('PHYSICS_ARRAY_SHAPES_AND_POSITIVITY_PASS')
subprocess.run([sys.executable,'-m','unittest','discover','-s','tests','-v'],check=True,cwd=root)
with tempfile.TemporaryDirectory() as tmp:
    subprocess.run([sys.executable,str(root/'code/coupled_thomson_volterra.py'),'--archive',str(src),'--out',tmp],check=True,cwd=root,stdout=subprocess.DEVNULL)
    with np.load(pkg,allow_pickle=False) as old, np.load(pathlib.Path(tmp)/pkg.name,allow_pickle=False) as new:
        for key in ('Q_coupled','deltaI','I_full','Q_first_gain','paired_deltaQ','paired_feedback_deltaQ'):
            gap=float(np.max(np.abs(old[key]-new[key])))
            assert gap<1e-12,(key,gap)
            print('CLEAN_REPLAY_NUMERICAL_PASS',key,gap)
    r=json.loads((pathlib.Path(tmp)/'SCIENCE_SUMMARY.json').read_text())
    assert r['max_absolute_fixed_operator_residual']<1e-11
    assert r['original_cases']==50 and r['new_ivps']==0
    print('REPLAY_OPERATOR_RESIDUAL_PASS',r['max_absolute_fixed_operator_residual'])
print('ALL_SCOPED_VERIFICATION_PASSED')
