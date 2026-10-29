from pathlib import Path
import json,ast,re,hashlib
r=Path(__file__).resolve().parents[3];e=r/'research/production_hardening_20261023/evidence'
raw=json.loads((e/'DENSE_ORDER_WOLFRAM_RAW.json').read_text());text=raw['content'][0]['text'];result=json.loads(json.loads(text.split('= ',1)[1]));
from fractions import Fraction

def expr(s):
 def walk(x):
  if isinstance(x,ast.List):return [walk(y) for y in x.elts]
  if isinstance(x,ast.Constant):return Fraction(str(x.value))
  if isinstance(x,ast.UnaryOp) and isinstance(x.op,ast.USub):return -walk(x.operand)
  if isinstance(x,ast.BinOp) and isinstance(x.op,ast.Div):return walk(x.left)/walk(x.right)
  raise ValueError(ast.dump(x))
 return walk(ast.parse(s.strip(),mode='eval').body)
def rust_table(file,name):
 s=file.read_text().split('pub const '+name+':',1)[1].split('=',1)[1].split('];',1)[0]+']'
 s=re.sub(r'\[0\.0;\s*(\d+)\]',lambda m:'['+','.join(['0']*int(m[1]))+']',s)
 s=re.sub(r'(\d+)\.0',r'\1',s)
 return expr(s)
wl=(e/'DENSE_ORDER_CHECK.wl').read_text()
def wl_table(name):return expr(wl.split(name+'=',1)[1].split(';',1)[0].replace('{','[').replace('}',']'))
p=r/'crates/rodas5p-integrators/src/dopri5_dense.rs';a=r/'crates/rodas5p-integrators/src/dopri5.rs'
ident={'P':rust_table(p,'DOPRI5_DENSE_P')==wl_table('pp'),'A':rust_table(a,'DOPRI5_A')==wl_table('aa'),'c':rust_table(a,'DOPRI5_C')==wl_table('cc')}
assert all(ident.values());result.update({'schema':'vigilode.dopri5-dense-exact-order.v1','tool':'Wolfram Language evaluator','symbolic_arithmetic':'exact rationals, polynomial identities for arbitrary theta','source_coefficient_literal_match':ident,'source_files_at_review':{str(x.relative_to(r)):hashlib.sha256(x.read_bytes()).hexdigest() for x in [p,a]},'check_code_sha256':hashlib.sha256((e/'DENSE_ORDER_CHECK.wl').read_bytes()).hexdigest(),'conditions':['b(theta).1=theta','b(theta).c=theta^2/2','b(theta).c^2=theta^3/3','b(theta).A.c=theta^3/6','b(theta).c^3=theta^4/4','b(theta).diag(c).A.c=theta^4/8','b(theta).A.c^2=theta^4/12','b(theta).A^2.c=theta^4/24'],'preregistration_commit':'83c79b936efc613b73c41897b647fffa6e6d0a1d','old_endpoint_order5_reverified':False,'interpretation':'quartic continuous extension order4, local interpolation error O(h^5); conditional bounded-time global O(H^5) requires an independent fifth-order nodal error bound and smooth bounded stages, not an automatic tolerance certificate','parser_development_note':'First literal matcher accidentally matched DOPRI5_ADAPTIVE_METHOD when searching DOPRI5_A; adding exact colon delimiter resolved the source-extraction error. Wolfram calculation was not rerun or changed.'})
(e/'DENSE_ORDER_CHECK.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,indent=2))
