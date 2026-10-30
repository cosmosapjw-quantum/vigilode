#!/usr/bin/env python3
import zipfile,json,hashlib,pathlib,sys,os
root=pathlib.Path(__file__).resolve().parents[1]
def h(path):
 with open(path,'rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
exclude={'GIT_MANIFEST.json','FILE_MANIFEST.json'}
text_types={'.md','.json','.py','.sh','.yml','.yaml','.svg','.csv','.log','.txt'}
allpaths=sorted(p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() and p.suffix not in {'.pyc'} and '__pycache__' not in p.parts)
files={n:h(root/n) for n in allpaths if n not in exclude}
gitfiles={n:d for n,d in files.items() if pathlib.PurePosixPath(n).suffix in text_types and not n.startswith('source/PARENT_PR143_FULL_SOURCE.zip') and not n.startswith('source/PR141_EHARM_50x193.npz') and not n.startswith('results/HIGH_E_HIERARCHY_50x193.npz')}
(root/'GIT_MANIFEST.json').write_text(json.dumps({'schema':'vigilode.bass21.highrank.git_mirror.v1','parent':'15d68af701fc9267e2c4b04cc81e2550a9bbbd50','prereg':'c7fc51f2a8007659db095073b83c39a46fea064c','files_sha256':gitfiles},sort_keys=True,ensure_ascii=False,indent=2)+'\n')
files['GIT_MANIFEST.json']=h(root/'GIT_MANIFEST.json')

(root/'FILE_MANIFEST.json').write_text(json.dumps({'schema':'vigilode.bass21.highrank.full_source.v1','file_count':len(files),'files_sha256':dict(sorted(files.items()))},sort_keys=True,ensure_ascii=False,indent=2)+'\n')
print('SEALED',len(files),'GIT',len(gitfiles))
