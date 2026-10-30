#!/usr/bin/env python3
"""Text-only Git repository research evidence; does not claim original binary replay."""
import csv,hashlib,json,pathlib
ROOT=pathlib.Path(__file__).resolve().parents[1]
def digest(p):
    with open(p,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def main():
    j=json.loads((ROOT/'GIT_MANIFEST.json').read_text())
    for relative,expected in j['files_sha256'].items():
        p=ROOT/relative
        assert p.is_file() and digest(p)==expected,('GIT_TEXT_ORIGINAL_SHA_DIFFERENT',relative)
    s=json.loads((ROOT/'results/SCIENCE_SUMMARY.json').read_text())
    assert s['original_cases']==50 and s['saved_epochs']==193
    assert s['new_high_coefficients']==38600 and s['new_generator_values']==106150 and s['new_upper_flux_values']==38600
    assert s['new_ivps']==0 and s['dag']['vigilode']=='7/23' and s['dag']['official_delta']==0
    assert s['max_GL64_96_gap']<1e-10 and s['max_reuse_E_l2_16']<1e-10
    with open(ROOT/'results/ENDPOINT_50_HIGH_E.csv',newline='') as f: endpoints=list(csv.DictReader(f))
    with open(ROOT/'results/MATCHED_25_HIGH_E.csv',newline='') as f: pairs=list(csv.DictReader(f))
    with open(ROOT/'results/TIME193_HIGH_E_MEANS.csv',newline='') as f: times=list(csv.DictReader(f))
    assert len(endpoints)==50 and len(set(x['case'] for x in endpoints))==50
    assert len(pairs)==25 and len(times)==193
    print(f'GIT_SOURCE_FREE_SCOPED_PASS: {len(j["files_sha256"])} text/CSV/SVG hashes, 50 source cases, 25 matched pairs, 193 epochs; full source binary replay NOT_RUN_BY_GITHUB_CI')
if __name__=='__main__':main()
