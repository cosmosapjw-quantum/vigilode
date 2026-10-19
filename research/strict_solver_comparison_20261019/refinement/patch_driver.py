"""Add only a named banded benchmark arm; never edit production crates."""
import hashlib,json,sys
from pathlib import Path
node=Path(__file__).resolve().parents[1];p=node/'src/main.rs';s=p.read_text();b=s.encode()
before=hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()
marker='#[path = "../refinement/banded.rs"]\nmod banded;'
if marker in s:
    print(json.dumps({'status':'ALREADY_APPLIED','blob':before}));sys.exit(0)
if before!='165b4da0acaed7f683d942debdc4fc335ac6ba03':raise RuntimeError('UNEXPECTED_SOURCE:'+before)
edits=[
 ('use serde_json::{json, Value};','use serde_json::{json, Value};\n'+marker),
 ('"vigilode-fast"=>native(c,times,rtol,h,audit),','"vigilode-fast"=>native(c,times,rtol,h,audit),"vigilode-banded"=>banded::solve(c,times,rtol,h,audit),'),
 ('["vigilode-fast","diffsol-bdf","netlib-dvode","netlib-dvode-land"].contains','["vigilode-fast","diffsol-bdf","netlib-dvode","netlib-dvode-land","vigilode-banded"].contains'),
 ('"arms":["vigilode-fast","diffsol-bdf","netlib-dvode"]','"arms":["vigilode-fast","diffsol-bdf","netlib-dvode","vigilode-banded"]'),
 ('if arm=="vigilode-fast"||arm.ends_with("-land")','if arm=="vigilode-fast"||arm=="vigilode-banded"||arm.ends_with("-land")')]
for a,z in edits:
    if s.count(a)!=1:raise RuntimeError('SOURCE_ANCHOR_COUNT:'+a)
    s=s.replace(a,z,1)
p.write_text(s)
print(json.dumps({'status':'APPLIED','before_blob':before,'after_sha256':hashlib.sha256(s.encode()).hexdigest(),'production_code_changed':False}))
