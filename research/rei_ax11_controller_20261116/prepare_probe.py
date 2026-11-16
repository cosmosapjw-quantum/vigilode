#!/usr/bin/env python3
"""Prepare an append-only diagnostic on a private, source-verified original copy.
This never updates the owner checkout, original checkpoint, config or source identity.
"""
import argparse,hashlib,json,shutil
from pathlib import Path

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def prepare(original,runroot,work):
 unit=Path(__file__).resolve().parent
 lock=json.loads((unit/'results/SOURCE_ORIGINAL_LOCK.json').read_text())
 alloc=json.loads((unit/'PROFILE_ALLOCATION.json').read_text())
 if work.exists():raise ValueError('NEW_DIAGNOSTIC_WORKSPACE_REQUIRED')
 for name,rec in lock['files'].items():
  if sha(original/name)!=rec['sha256'] or (original/name).stat().st_size!=rec['bytes']:
   raise ValueError('ORIGINAL_SOURCE_CORPUS_MISMATCH:'+name)
 work.mkdir(parents=True)
 shutil.copytree(original,work/'owner_source',ignore=shutil.ignore_patterns('target','.git','__pycache__'))
 driver=work/'owner_source/rust/rei_microphysics/src/axbi_driver.rs'
 before=driver.read_bytes()
 driver.write_bytes(before+b'\n'+(unit/'controller_probe_append.rs').read_bytes())
 if driver.read_bytes()[:len(before)]!=before:raise ValueError('ORIGINAL_DRIVER_PREFIX_CHANGED')
 (work/'owner_source/rust/rei_microphysics/examples/vigilode_ax11_error_probe.rs').write_bytes((unit/'native_error_probe.rs').read_bytes())
 for case,rel in [('ZERO','flrw/32d11e733155/ZERO'),('SHEAR_PLUS','paired/32d11e733155/SHEAR_PLUS')]:
  out=work/'inputs'/case;out.mkdir(parents=True)
  for name,key,src in [('config.cfg','config_sha256','config.cfg'),('checkpoint.txt','checkpoint_sha256','checkpoints/00199980.txt')]:
   p=runroot/rel/src
   if sha(p)!=alloc[key][case]:raise ValueError('ORIGINAL_INPUT_MISMATCH:'+case+'/'+name)
   (out/name).write_bytes(p.read_bytes())
 (work/'SOURCE_ORIGINAL_LOCK.json').write_text(json.dumps(lock,indent=2,sort_keys=True)+'\n')
 print('SOURCE_COPIED_AND_PREFIX_PINNED; BIND FRESH FINITE ALLOCATION AND BINARY SHA BEFORE NATIVE EXECUTION')
if __name__=='__main__':
 p=argparse.ArgumentParser()
 for name in ['original_source','original_runroot','new_workspace']:p.add_argument('--'+name,type=Path,required=True)
 a=p.parse_args();prepare(a.original_source,a.original_runroot,a.new_workspace)
