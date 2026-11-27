from pathlib import Path
from collections import Counter
import re,json,subprocess
p=Path(__file__).parent
fs=[Path(x)for x in subprocess.check_output(['git','ls-files','crates'],text=True).splitlines()if x.endswith('.rs')]
src=[x for x in fs if '/src/'in str(x)]
tests=[x for x in fs if '/tests/'in str(x)]
token=re.compile(r'\b[A-Za-z_][A-Za-z_0-9]*\b')
decl=re.compile(r'(?m)^\s*pub(?:\(crate\))?\s+(?:const\s+|async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)\s*(?:<|\()')
S=Counter();T=Counter();defs={}
for f in src:
 s=f.read_text();S.update(token.findall(s))
 for m in decl.finditer(s):defs.setdefault(m.group(1),[]).append({'path':str(f),'line':1+s[:m.start()].count('\n')})
for f in tests:T.update(token.findall(f.read_text()))
candidates=[{'symbol':k,'definition':d[0],'source_mentions':S[k],'test_mentions':T[k]}for k,d in defs.items()if len(d)==1 and S[k]==1 and T[k]>0]
candidates.sort(key=lambda x:(-x['test_mentions'],x['symbol']))
out={'scope':'lexical source/test token count across tracked Rust crates, unique definition with one source mention and tests; macro/external callers/overloads not resolved, not proof of unused code','source_files':len(src),'test_files':len(tests),'candidates':candidates,'selected_underused':'Dictionary::phi_all_fast (production wrapper phi_batch_into previously mostly exercised in tests; now used by slab_workspace_green)'}
(p/'MODULE_CANDIDATE_INVENTORY.json').write_text(json.dumps(out,indent=2)+'\n')
print('INVENTORY',len(src),len(tests),len(candidates),[(x['symbol'],x['test_mentions'])for x in candidates[:15]])
