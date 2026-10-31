#!/usr/bin/env python3
"""Check published source-free physical data; does NOT replay private original ZIP."""
import csv,hashlib,json,pathlib,collections,math
ROOT=pathlib.Path(__file__).resolve().parents[1]

def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for b in iter(lambda:f.read(1<<20),b''):h.update(b)
    return h.hexdigest()

def rows(name):
    with (ROOT/'results'/name).open() as f:return list(csv.DictReader(f))

def check():
    manifest=json.loads((ROOT/'GIT_MANIFEST.json').read_text())
    for path,sha in manifest['sha256'].items():
        file=ROOT/path
        assert file.is_file(),path
        assert digest(file)==sha, (path,'SHA_MISMATCH')
    s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
    assert s['physical_source_cases']==50 and s['physical_source_epochs']==193
    assert s['output_mode_count']==77 and s['owner_ivps']==0
    assert s['count_complex_E']==s['count_complex_B']==3715250
    assert s['DAG']['vigilode']=='7/23' and s['production']=='HOLD'
    assert s['max_weight1_control_B_physical']<1e-12
    assert s['max_abs_B_by_frame']['x005']>1e-3
    assert s['max_abs_B_by_frame']['rest']<1e-12
    assert s['first_order_B21_over_beta']['absolute_error']<2e-7
    endpoint=rows('ENDPOINT_50x5_NONAXIAL_EB.csv')
    pairs=rows('MATCHED25x5_NONAXIAL_EB.csv')
    epoch=rows('EPOCH193x5_NONAXIAL_EB.csv')
    assert len(endpoint)==250 and len(pairs)==125 and len(epoch)==965
    groups=collections.defaultdict(dict)
    for row in endpoint:
        groups[row['case']][row['frame']]=row
    assert len(groups)==50 and all(len(g)==5 for g in groups.values())
    max_delta=0.
    for g in groups.values():
        x=float(g['x005']['B_21_final_imag'])
        y=float(g['negx005']['B_21_final_imag'])
        max_delta=max(max_delta,abs(x+y))
        assert abs(float(g['rest']['B_21_final_imag']))<1e-12
    assert max_delta<1e-12
    assert all(math.isfinite(float(r['s04_mean_B21_imag'])) for r in epoch)
    print(json.dumps({'status':'PASS_SOURCE_FREE_ONLY','git_manifest_files':len(manifest['sha256']),'endpoints':len(endpoint),'pairs':len(pairs),'epoch_rows':len(epoch),'symmetric_max':max_delta,'DAG':'7/23'},sort_keys=True))
if __name__=='__main__':check()
