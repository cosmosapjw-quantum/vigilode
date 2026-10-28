# Final archive closure: deterministic reference restored exactly

The numerical benchmark run39355449893 completed successfully, but frozen closeout39357244608 failed because REFERENCES.json was absent from Git. Its16024519bytes lay below16MiB yet above15MiB. GNU find's `-size -16M` predicate rounds file size to MiB units and excluded it, while the Python splitter acted only at >=16MiB. The failure was reproduced locally with a15728641-byte temporary file. This is a packaging defect, not a numerical disagreement.

Recovery run39357596507 rebuilt ONLY the reference executable from the actual measured source `f08d152a35fb2db748692570969cb1723c287c26`, reran its10deterministic reference modes and the unchanged independent Python reference calculations, and reconstructed the original JSON document. It did not run candidate solvers or produce new timing samples. The original command-path field was checked against a retained raw record, not guessed or silently changed.

The regenerated reference SHA256 is **exactly** the pre-existing recorded SHA256:

`93d840405fbe3a095346bcadf089b6ee89cf844927be26f9879034ee0fc63091`

Its compressed byte-preserving copy is [reference_recovery/39357596507/REFERENCES.json.gz](reference_recovery/39357596507/REFERENCES.json.gz). The gzip container need not have the old container bytes; the decompressed original JSON bytes match. [RECOVERY.json](reference_recovery/39357596507/RECOVERY.json) records `PASS_EXACT_REFERENCE_RECOVERY`. With those exact bytes in a temporary copy, all101listed archived hashes passed and the saved numerical audit was reproduced. [VALIDATION.json](reference_recovery/39357596507/VALIDATION.json) additionally checks33passed tests, unchanged measured numerical source, and unchanged upstream libraries. Historical scores, timings, manifests and original SHA256SUMS were not edited.

The recovery publisher uses the byte-exact predicate `-size -16777216c`. Historical measurement workflows remain as executed artifacts; future publishers must use the byte predicate rather than copying their rounded-MiB test. Do not rerun a historical whole benchmark merely to recreate this reference.

## Readback using a new temporary copy

From the repository root, Python stdlib is sufficient:

```bash
python3 - <<'PY'
from pathlib import Path
import gzip,hashlib,shutil,subprocess,sys,tempfile
node=Path('research/coupled_economy_20261020')
source=node/'runs/39355449893'
work=Path(tempfile.mkdtemp(prefix='vigilode-economy-readback-'))/'run'
shutil.copytree(source,work)
b=gzip.decompress((node/'reference_recovery/39357596507/REFERENCES.json.gz').read_bytes())
assert hashlib.sha256(b).hexdigest()=='93d840405fbe3a095346bcadf089b6ee89cf844927be26f9879034ee0fc63091'
(work/'REFERENCES.json').write_bytes(b)
for line in (work/'SHA256SUMS').read_text().splitlines():
    digest,path=line.split('  ',1)
    assert hashlib.sha256((work/path).read_bytes()).hexdigest()==digest,path
subprocess.run([sys.executable,str(node/'audit.py'),str(work)],check=True)
print(work)
PY
```

This command follows the executed recovery verification logic; the historical directories are not overwritten. It recalculates saved scores/ratios, not integration trajectories. A candidate's root completion remains distinct from embedded timestep acceptance and global accuracy. The initially failed closeout stays preserved.
