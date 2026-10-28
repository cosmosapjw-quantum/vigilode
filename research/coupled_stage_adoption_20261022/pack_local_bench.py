#!/usr/bin/env python3
"""Archive every raw campaign output, including failures and empty stderr logs."""
import argparse,gzip,hashlib,json,pathlib,tarfile

def main():
    ap=argparse.ArgumentParser();ap.add_argument('campaign');args=ap.parse_args()
    folder=pathlib.Path(args.campaign)
    paths=sorted([*folder.glob('[0-9][0-9][0-9].stdout.json'),*folder.glob('[0-9][0-9][0-9].stderr.log')])
    if not paths:raise SystemExit('No raw campaign output files; refusing empty archive')
    entries=[{'path':p.name,'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths]
    archive=folder/'RAW_OUTPUTS.tar.gz'
    with archive.open('wb') as raw:
        with gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as gz:
            with tarfile.open(mode='w',fileobj=gz,format=tarfile.USTAR_FORMAT) as tar:
                for path in paths:
                    info=tarfile.TarInfo(path.name);info.size=path.stat().st_size;info.mode=0o644;info.mtime=0
                    with path.open('rb') as content:tar.addfile(info,content)
    with tarfile.open(archive,'r:gz') as tar:
        assert sorted(tar.getnames())==sorted(x['path'] for x in entries)
        for entry in entries:
            content=tar.extractfile(entry['path']).read()
            assert len(content)==entry['bytes'] and hashlib.sha256(content).hexdigest()==entry['sha256']
    manifest={'schema':'vigilode.raw-output-archive.v1','all_raw_outputs_included':True,'raw_files_retained_locally':True,
              'archive':archive.name,'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),
              'archive_bytes':archive.stat().st_size,'decompressed_bytes':sum(x['bytes'] for x in entries),'files':entries}
    (folder/'RAW_OUTPUTS_MANIFEST.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (folder/'.gitignore').write_text('# Reconstruct all ignored raw files from the verified archive.\n[0-9][0-9][0-9].stdout.json\n[0-9][0-9][0-9].stderr.log\n')
    (folder/'RAW_OUTPUTS_README.md').write_text('# Complete raw benchmark records\n\n`RAW_OUTPUTS.tar.gz` contains every numbered stdout/stderr file, including failed configurations, warm-up records, and all three timed repeats. `RAW_OUTPUTS_MANIFEST.json` records each uncompressed SHA256 and the archive SHA256. Originals are retained locally and ignored in Git to avoid duplicate storage.\n\nFrom this directory, recover the exact files with:\n\n```sh\ntar -xzf RAW_OUTPUTS.tar.gz\n```\n\nThe filenames in `RECEIPTS.json` and `SUMMARY.json` then resolve directly. Re-run `../../analyze_local_bench.py` with this directory as its argument to regenerate the numeric summary and figures. No benchmark rerun is required to inspect evidence.\n')
    print(json.dumps({'files':len(paths),'raw_bytes':manifest['decompressed_bytes'],'archive_bytes':manifest['archive_bytes'],'verified':True}))

if __name__=='__main__':main()
