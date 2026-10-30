"""Restore frozen BASS-owned data from transport and nested predecessor archives, without IVPs."""
from __future__ import annotations
import base64, hashlib, json, re, pathlib, zipfile, ast
R=pathlib.Path(__file__).resolve().parents[1]
OWNER='cosmosapjw-quantum/bass@cc602e209e7deeb511ab2a1f3b2268d55206c43c'
PARTS=['BASS_OWNER_RAW_HISTORY_4_CASES_BATCH01_20261030.txt',
       'BASS_OWNER_RAW_HISTORY_6_CASES_BATCH02_20261030.txt',
       *[f'BASS_OWNER_4_CASES_TRANSFER_G{i:02d}_20261030.txt' for i in range(3,9)],
       'BASS_OWNER_4_CASES_TRANSFER_G09_20261030_RETRY.txt',
       'BASS_OWNER_4_CASES_TRANSFER_G10_20261030.txt']

def sha(data):return hashlib.sha256(data).hexdigest()
def gitsha(data):return hashlib.sha1(b'blob '+str(len(data)).encode()+b'\x00'+data).hexdigest()

def restore():
    R.joinpath('inputs').mkdir(exist_ok=True)
    if (R/'results'/'SOURCE_INPUT_MANIFEST.json').is_file() and len(list((R/'inputs').glob('s??_m??_r??.npz')))==50:
        manifest=json.loads((R/'results'/'SOURCE_INPUT_MANIFEST.json').read_text())
        for c,m in manifest['sources'].items():
            b=(R/'inputs'/(c+'.npz')).read_bytes()
            assert sha(b)==m['sha256'] and gitsha(b)==m['git_blob'],(c,'REPLAY_ORIGINAL_SOURCE_MISMATCH')
        print(json.dumps({'verified_existing_originals':50,'new_ivps':0}))
        return manifest
    source=R/'source'/'previous_PR122'
    tree=ast.parse((source/'code'/'paired_replay.py').read_text())
    PIN=ast.literal_eval(next(n.value for n in tree.body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='PIN' for t in n.targets)))
    rows={};cases=[]; bad_transport=[]
    for file in PARTS:
        path=R.parent/file
        assert path.is_file(),f'INPUT_FILE_MISSING {path}'
        for n,line in enumerate(path.read_text(encoding='utf-8-sig').splitlines()):
            try: obj=json.loads(line)
            except json.JSONDecodeError:
                # Batch02 was interrupted on Docs quota, first line was never a JSON object.
                if file=='BASS_OWNER_RAW_HISTORY_6_CASES_BATCH02_20261030.txt' and n==0:
                    bad_transport.append({'file':file,'line':n,'status':'INCOMPLETE_DISCARDED'})
                    continue
                raise
            case=obj['case']; assert re.fullmatch(r's0[04]_m0[0-4]_r0[0-4]',case),case
            data=base64.b64decode(obj['trajectory_base64'],validate=True)
            expected=obj['result']['outputs']['trajectory.npz']
            assert len(data)==expected['bytes'],(case,'bytes',len(data),expected['bytes'])
            assert sha(data)==expected['sha256'],(case,'sha')
            assert gitsha(data)==obj['git_blob'],(case,'gitblob')
            assert obj['result']['status']=='COMPLETE' and obj['result']['config']['case_id']==case
            assert obj['result']['config']['samples']==193 and obj['result']['config']['opacity'] is True
            with zipfile.ZipFile(__import__('io').BytesIO(data)) as z:
                assert z.testzip() is None,(case,'CRC')
            assert case not in rows,('DUPLICATE_TRANSPORT',case)
            rows[case]={'sha256':sha(data),'git_blob':obj['git_blob'],'bytes':len(data),
                        'original_result_git_blob':obj['result_git_blob'],
                        'scientific_identity_sha256':obj['result']['scientific_identity_sha256'],
                        'source':'same-commit GitHub fetch_file base64 and RESULT.json; '+file}
            (R/'inputs'/(case+'.npz')).write_bytes(data)
    assert len(rows)==40,('UNEXPECTED_NEW_CASE_COUNT',len(rows),sorted(rows))
    # Previous PR122 stored original 10 actual trajectory bytes and pin source Git blobs.
    with zipfile.ZipFile(source/'source'/'BASS21_PR121_PARENT_FULL.zip') as z:
        prev=z.read('original/trajectory.npz')
    for c,(rel,hsh,blob) in PIN.items():
        data=prev if c=='s00_m00_r00' else (source/rel).read_bytes()
        assert sha(data)==hsh and gitsha(data)==blob,(c,'PARENT_LOCK_MISMATCH')
        assert c not in rows
        with zipfile.ZipFile(__import__('io').BytesIO(data)) as z:
            assert z.testzip() is None
        rows[c]={'sha256':hsh,'git_blob':blob,'bytes':len(data),'source':'source-full predecessor PR122, original BASS source lock'}
        (R/'inputs'/(c+'.npz')).write_bytes(data)
    assert len(rows)==50 and {f's{s:02d}_m{m:02d}_r{r:02d}' for s in [0,4] for m in range(5) for r in range(5)}==set(rows)
    assert not ({x for x in rows if x.startswith('s01_')})
    manifest={'owner':OWNER,'sources':rows,'discarded_incomplete_transport':bad_transport,'count':len(rows),
              'newly_restored':40,'prior_restored':10,'epochs_per_case':193,'new_ivps':0,
              'source_model_sha256':'baecce07aded483fa60006cc576feff67d068c0d70394f7095abacc6d47eaa80'}
    (R/'results'/'SOURCE_INPUT_MANIFEST.json').write_text(json.dumps(manifest,sort_keys=True,indent=2)+'\n')
    print(json.dumps({'restored':len(rows),'new':40,'prior':10,'bytes':sum(v['bytes'] for v in rows.values()),'discarded':bad_transport,'manifest_sha':sha((R/'results'/'SOURCE_INPUT_MANIFEST.json').read_bytes())}))
    return manifest
if __name__=='__main__':restore()
