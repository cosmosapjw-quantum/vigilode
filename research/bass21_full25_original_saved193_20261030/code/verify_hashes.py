"""Check immutable source-input/owned-code SHA and regenerated physical output identity."""
import hashlib,json,pathlib,sys
R=pathlib.Path(__file__).resolve().parents[1]
mode=sys.argv[1] if len(sys.argv)>1 else 'static'
if mode not in ('static','results'):raise SystemExit(2)
name={'static':'SOURCE_STATIC_HASHES.json','results':'RESULT_OUTPUT_HASHES.json'}[mode]
M=json.loads((R/name).read_text());bad=[]
for key,expect in M.items():
 p=R/key
 actual=hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else '<MISSING>'
 if actual!=expect:bad.append((key,expect,actual))
if bad:
 print('SHA256_FAIL',len(bad),bad[:7]);sys.exit(1)
print(f'SHA256_{mode.upper()}_PASS',len(M))
