#!/usr/bin/env bash
# Byte-checked completion of the independent research archive.
# Usage: bash research/cumulative_20261020/import_remaining_archives.sh /folder/with/four-zips
set -euo pipefail
ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"
BRANCH=$(git symbolic-ref --quiet --short HEAD || true)
[[ "$BRANCH" == "research/cumulative-evidence-20261020" ]] || {
  echo >&2 "Refuse: switch to research/cumulative-evidence-20261020"; exit 2;
}
if [[ -n $(git status --porcelain) ]]; then
  echo >&2 "Refuse: worktree must be clean before importing original archives"; exit 3
fi
SOURCE_DIR=$1
ROOT_INPUT=$(cd "$SOURCE_DIR" && pwd)
export ROOT_INPUT
python3 - <<'PY'
import hashlib,os,pathlib,stat,zipfile,json
root=pathlib.Path.cwd()
source=pathlib.Path(os.environ['ROOT_INPUT'])
dest=root/'research'/'cumulative_20261020'/'original_archives'
dest.mkdir(parents=True,exist_ok=True)
expected={
'VigilODE_Integration_Research_20261020.zip':'54f4f4a99d552388fc28d01f5f367176e301c2af15c3cd8a9c7a30b4e5dfd4f1',
'VigilODE_DerivativeFree_FVM_Research_20261020.zip':'a17864dd8fa1951c7ddcd236740c564c10b323248b1bb30c92eea32bc1e745e7',
'VigilODE_VODE_Diffsol_Comparison_20261019.zip':'790b4621e09dff21e196cc548b7851ca751d09f7b0287c52e0bc857fca2ad9ae',
'VigilODE_Hybrid_AD_Recovery_20261020.zip':'aea357e42f53e027b18ab012a5cb9fb554bd3d0f2ab2a641a7eaa27b6632c656',
}
records=[]
for name,want in expected.items():
    z=source/name
    if not z.is_file():raise SystemExit(f'Missing {z}')
    data=z.read_bytes()
    got=hashlib.sha256(data).hexdigest()
    if got!=want:raise SystemExit(f'ZIP SHA256 mismatch: {name}: {got}')
    target=dest/name
    if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest()!=want:
        raise SystemExit(f'Existing target has different bytes: {target}')
    if not target.exists():target.write_bytes(data)
    with zipfile.ZipFile(z) as archive:
        for info in archive.infolist():
            path=pathlib.PurePosixPath(info.filename)
            if path.is_absolute() or '..' in path.parts or not path.parts:
                raise SystemExit(f'Unsafe path in {name}: {path}')
            if ((info.external_attr>>16)&0o170000)==stat.S_IFLNK:
                raise SystemExit(f'No symlinks from original archive: {path}')
            if info.is_dir():continue
            if info.file_size>16*1024*1024:raise SystemExit(f'Unusually large file: {path}')
            content=archive.read(info)
            path_out=dest/name.removesuffix('.zip')/path
            if path_out.exists():
                if hashlib.sha256(path_out.read_bytes()).digest()!=hashlib.sha256(content).digest():
                    raise SystemExit(f'Existing source conflict: {path_out}')
            else:
                path_out.parent.mkdir(parents=True,exist_ok=True)
                path_out.write_bytes(content)
            records.append((name,str(path_out.relative_to(root)),len(content),hashlib.sha256(content).hexdigest()))
assert len(records)==72,len(records)
receipt={'status':'ORIGINAL_FOUR_ZIPS_AND_72_COMPONENT_FILES_VERIFIED',
         'archives':expected,'components':[dict(archive=z,path=p,bytes=n,sha256=h) for z,p,n,h in records]}
out=dest/'ARCHIVE_TRANSFER_RECEIPT.json'
out.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n')
print('Imported',len(records),'files plus four ZIPs; all sha256 verified')
PY
git add -- research/cumulative_20261020/original_archives
if git diff --cached --quiet; then
    echo "No new files: already byte-identical."
else
    git commit -m "research: publish four original bundles and 72 byte-verified study files"
    git push origin HEAD:refs/heads/research/cumulative-evidence-20261020
    echo "Pushed original bundles, sources and transfer receipt."
fi
