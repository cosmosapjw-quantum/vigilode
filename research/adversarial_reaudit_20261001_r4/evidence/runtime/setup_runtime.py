from pathlib import Path
import tarfile,hashlib,json,os,subprocess,time,zipfile,tomllib
root=Path(__file__).resolve().parents[3];rt=root/'runtime_r4';ev=Path(__file__).resolve().parent;prefix=rt/'rust-1.94.1';prefix.mkdir(exist_ok=True)
record={'archive_file_checks':[],'extracted_files':[],'prior_pass_inherited':False,'signature_verified':False,'signature_note':'No independently trusted signing key supplied; byte hash identity only.'}
for p in [root/'project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz',root/'project_sources/15-GENERIC_VECTOR_JF_OFFLINE_CARGO_VENDOR_20260816-1-.zip',rt/'vendor_archive.tar.zst']:
 with p.open('rb') as f: h=hashlib.file_digest(f,'sha256').hexdigest()
 record['archive_file_checks'].append({'file':str(p),'bytes':p.stat().st_size,'sha256':h})
with zipfile.ZipFile(root/'project_sources/15-GENERIC_VECTOR_JF_OFFLINE_CARGO_VENDOR_20260816-1-.zip') as z:
 sums=z.read('GENERIC_VECTOR_JF_OFFLINE_CARGO_VENDOR_20260816/SHA256SUMS.txt').decode();record['vendor_archive_supplied_checksum']=sums;assert record['archive_file_checks'][-1]['sha256'] in sums
components={'rustc','cargo','rust-std-x86_64-unknown-linux-gnu','rustfmt-preview','clippy-preview'}
with tarfile.open(root/'project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz','r|xz',ignore_zeros=True) as t:
 for m in t:
  ps=Path(m.name).parts
  if len(ps)<4 or ps[1] not in components or ps[2] not in {'bin','lib','libexec'}:continue
  dst=prefix.joinpath(*ps[2:]);dst.parent.mkdir(parents=True,exist_ok=True)
  if m.isdir():dst.mkdir(exist_ok=True);continue
  if not m.isfile():continue
  with t.extractfile(m) as src,dst.open('wb') as f:
   while chunk:=src.read(8*1024*1024):f.write(chunk)
  os.chmod(dst,m.mode);assert dst.stat().st_size==m.size,(str(dst),dst.stat().st_size,m.size)
  record['extracted_files'].append({'member':m.name,'path':str(dst),'bytes':m.size})
(rt/'vendor_unpack').mkdir(exist_ok=True)
p=subprocess.run(['tar','--no-same-owner','--zstd','-xf',str(rt/'vendor_archive.tar.zst'),'-C',str(rt/'vendor_unpack')],capture_output=True)
(ev/'vendor_extract.stdout').write_bytes(p.stdout);(ev/'vendor_extract.stderr').write_bytes(p.stderr);assert p.returncode==0
venders=list((rt/'vendor_unpack').glob('*/vendor'));assert len(venders)==1
vendor=venders[0]
ch=rt/'cargo-home';ch.mkdir(exist_ok=True)
(ch/'config.toml').write_text('[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = '+json.dumps(str(vendor))+'\n[net]\noffline = true\n')
(rt/'env.sh').write_text(f'export PATH="{prefix}/bin:$PATH"\nexport LD_LIBRARY_PATH="{prefix}/lib:${{LD_LIBRARY_PATH:-}}"\nexport CARGO_HOME="{ch}"\nexport CARGO_TARGET_DIR="{rt}/target-shared"\nexport CARGO_INCREMENTAL=0\nexport CARGO_BUILD_JOBS=1\nexport CARGO_PROFILE_DEV_CODEGEN_UNITS=1\nexport CARGO_PROFILE_TEST_CODEGEN_UNITS=1\n')
lock=tomllib.loads((root/'vigilode/Cargo.lock').read_text());available={}
for q in vendor.iterdir():
 if not (q/'Cargo.toml').exists():continue
 d=tomllib.loads((q/'Cargo.toml').read_text());available[d['package']['name'],d['package']['version']]=json.loads((q/'.cargo-checksum.json').read_text())['package']
match=[]
for q in lock['package']:
 if 'checksum' in q:
  found=available.get((q['name'],q['version']));match.append({'name':q['name'],'version':q['version'],'expected_checksum':q['checksum'],'vendor_checksum':found,'match':found==q['checksum']})
(ev/'dependency_match.json').write_text(json.dumps({'locked_count':len(match),'vendor_count':len(available),'matching':sum(q['match'] for q in match),'packages':match},indent=2)+'\n')
assert all(q['match'] for q in match)
record['status']='SETUP_EXTRACTION_COMPLETE';(ev/'extraction_receipt.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'status':record['status'],'extracted_files':len(record['extracted_files']),'locked':len(match),'vendor':len(available)}),flush=True)
