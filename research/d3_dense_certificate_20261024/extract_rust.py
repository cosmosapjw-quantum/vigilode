from pathlib import Path, PurePosixPath
import tarfile,hashlib,json,os
R=Path(__file__).resolve().parent
archive=Path('/workspace/scratch/6a9ff47f24f5/project_sources/06-rust-1.94.1-x86_64-unknown-linux-gnu.tar.xz')
digest=hashlib.file_digest(archive.open('rb'),'sha256').hexdigest()
assert digest=='294b3d81fa72e62581276290c60c81eb8b58498d333d422ca1dfc432877d0c40'
root=R/'toolchain';count=0;size=0
with tarfile.open(archive,'r|xz') as tar:
 for m in tar:
  p=PurePosixPath(m.name);parts=p.parts
  if len(parts)<4 or not m.isfile():continue
  comp=parts[1];rel=PurePosixPath(*parts[2:])
  include=(comp=='rustc' and (str(rel)=='bin/rustc' or str(rel).startswith('lib/'))) or (comp=='rust-std-x86_64-unknown-linux-gnu' and str(rel).startswith('lib/'))
  if not include:continue
  assert '..' not in rel.parts and not rel.is_absolute()
  dest=root/str(rel);dest.parent.mkdir(parents=True,exist_ok=True)
  data=tar.extractfile(m).read();dest.write_bytes(data);dest.chmod(m.mode & 0o777)
  count+=1;size+=len(data)
print(json.dumps({'archive_bytes':archive.stat().st_size,'sha256':digest,'extracted_files':count,'extracted_bytes':size,'compiler':str(root/'bin/rustc'),'signature_verification':'NOT_RUN; supplied archive provenance'}))
