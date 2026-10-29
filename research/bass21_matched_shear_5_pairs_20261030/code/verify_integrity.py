"""Strict original source + manifest/result hash verifier, no IVP or solver calls."""
from pathlib import Path
import json,hashlib,sys
R=Path(__file__).resolve().parents[1]
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
    man=json.loads((R/'MANIFEST.json').read_text())
    for e in man['files']:
        q=R/e['path'];assert q.exists() and q.stat().st_size==e['size'] and digest(q)==e['sha256'],e['path']
    source=json.loads((R/'CONTRACT.json').read_text())
    assert digest(R/'source/BASS21_PR121_PARENT_FULL.zip')=='19513fec19e53d5f2da8da50ba22aa066c634dee79ff94cff31d07b4cfb8f907'
    expected={k:v['sha256'] for k,v in source['source_inputs'].items()}
    for case,sha in expected.items():
        path=(R/'work/original/trajectory.npz') if case=='s00_m00_r00' else (R/(case+'_trajectory.npz'))
        if path.exists():assert digest(path)==sha,case
    print('MANIFEST_PASS',len(man['files']),'source_inputs',len(expected))
if __name__=='__main__':main()
