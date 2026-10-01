#!/usr/bin/env python3
"""Deterministic artifact validation; no scientific run and no third-party dependency.
Implements exactly the JSON Schema keywords used by AUDIT_BUNDLE.schema.json,
then checks IDs, DAG, numerical census, source and frozen inputs.
"""
from pathlib import Path
import argparse,hashlib,json,re,subprocess
ap=argparse.ArgumentParser();ap.add_argument('--repo',type=Path,required=True);ap.add_argument('--output',type=Path);a=ap.parse_args()
root=Path(__file__).resolve().parent;repo=a.repo.resolve();checks=[]
def j(p):return json.loads(p.read_text(),parse_constant=lambda x:(_ for _ in ()).throw(ValueError('Nonstandard JSON constant '+x)))
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check(condition,label):
 if not condition:raise AssertionError(label)
 checks.append(label)
def schema(v,s,path='$'):
 used={'$schema','title','description','type','required','properties','additionalProperties','items','enum','const','minItems','minLength','pattern'}
 check(set(s)<=used,'schema_keywords:'+path)
 if 'type'in s:
  ok={'object':isinstance(v,dict),'array':isinstance(v,list),'string':isinstance(v,str),'boolean':isinstance(v,bool),'integer':isinstance(v,int) and not isinstance(v,bool),'number':isinstance(v,(int,float))and not isinstance(v,bool)}
  check(ok[s['type']],'schema_type:'+path)
 if 'const'in s:check(v==s['const'] and (not isinstance(s['const'],bool) or type(v) is bool),'schema_const:'+path)
 if 'enum'in s:check(v in s['enum'],'schema_enum:'+path)
 if 'minItems'in s:check(len(v)>=s['minItems'],'schema_minItems:'+path)
 if 'minLength'in s:check(len(v)>=s['minLength'],'schema_minLength:'+path)
 if 'pattern'in s:check(re.search(s['pattern'],v)is not None,'schema_pattern:'+path)
 if isinstance(v,dict):
  check(set(s.get('required',[]))<=set(v),'schema_required:'+path)
  if s.get('additionalProperties') is False:check(set(v)<=set(s.get('properties',{})),'schema_extra:'+path)
  for k,ss in s.get('properties',{}).items():
   if k in v:schema(v[k],ss,path+'.'+k)
 if isinstance(v,list)and 'items'in s:
  for i,x in enumerate(v):schema(x,s['items'],path+f'[{i}]')
b=j(root/'AUDIT_BUNDLE.json');schema(b,j(root/'AUDIT_BUNDLE.schema.json'))
check(b['source_commit']=='1c54194123ee6abc6daa512e8574922f510b4e2c','audited_source_pin')
check(b['preregistration_commit']=='b2914f3e3c03eda60a8547619db6284aac8250a2','prereg_pin')
for field in ['findings','development','claims']:
 ids=[x['id']for x in b[field]];check(len(ids)==len(set(ids)),field+'_ids_unique')
nodes={x['id']:x for x in b['development']};done=set()
for name in b['topological_order']:
 check(name in nodes and name not in done,'topological_member:'+name)
 check(set(nodes[name]['depends_on'])<=done,'topological_dependencies:'+name);done.add(name)
check(done==set(nodes),'dag_complete')
check(len(b['closure'])==21 and len({x['node']for x in b['closure']})==21,'21_unique_prior_nodes')
check(len(b['findings'])==9,'9_findings');check(sum(x['severity']=='P2'for x in b['findings'])==8,'8_P2')
check(len(b['development'])==26 and len(b['claims'])==15,'26_tasks_15_claims')
for field,file,key in [('findings','FINDINGS.json','findings'),('closure','CLOSURE_MATRIX.json','rows'),('claims','RESEARCH_CLAIMS.json','claims'),('development','NEXT_DEVELOPMENT_DAG.json','nodes')]:check(b[field]==j(root/file)[key],'bundle_matches:'+file)
counts=b['runtime']['test_counts'];check(sum(counts.values())==735 and counts['PASS']==671,'runtime_735_census')
check(b['numeric_results']==j(root/'NUMERIC_RESULTS.json'),'numeric_results_match')
for lane,file,expected in [('time','native_raw.jsonl',36),('polynomial','native.jsonl',78),('homotopy','native.jsonl',9),('statistics','native.stdout.jsonl',21)]:
 rows=[json.loads(x)for x in(root/lane/file).read_text().splitlines()if x.strip()];check(len(rows)==expected,'raw_row_count:'+lane)
for rel,digest in j(root/'INPUTS_SHA256.json')['files_sha256'].items():check(sha(root/rel)==digest,'frozen_input:'+rel)
for rel,digest in j(root/'evidence/source/SOURCE_MANIFEST.json')['files_sha256'].items():check(sha(repo/rel)==digest,'source_hash:'+rel)
for rel in [b['report'],b['reproduce'],b['independent_review']]:check((root/rel).is_file(),'required_artifact:'+rel)
json_files=0
for p in root.rglob('*.json'):
 if '__pycache__' in p.parts or 'state' in p.parts:continue
 j(p);json_files+=1
check(True,f'strict_json_files:{json_files}')
result={'schema':'vigilode.r4.artifact_validation.v1','status':'PASS','checks_count':len(checks),'json_files':json_files,'schema_engine':'stdlib implementation of all and only used JSON Schema keywords; structural plus semantic checks','source_hashes':'MATCH','preregistered_bytes':'MATCH','census':'MATCH','dag':'ACYCLIC_COMPLETE','scientific_tests_rerun':False,'checks':checks}
if a.output:a.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items()if k!='checks'},ensure_ascii=False))
