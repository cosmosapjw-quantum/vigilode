"""Build a no-dependencies native microbenchmark from the repository tableau."""
import argparse
import json
import os
import platform
import subprocess
import tempfile
from pathlib import Path

from check_modal import tableau

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("repo", type=Path)
parser.add_argument("--rustc", default="rustc")
args = parser.parse_args()
here = Path(__file__).resolve().parent
g, a, c, b = tableau(args.repo)
with tempfile.TemporaryDirectory() as temp:
    work = Path(temp)
    tab = work / 'table.rs'
    tab.write_text('\n'.join([
        f"const G: f64 = {float(g)!r};",
        f"const A: [[f64; 8]; 8] = {[list(map(float,row)) for row in a]!r};",
        f"const C: [[f64; 8]; 8] = {[list(map(float,row)) for row in c]!r};",
        f"const B: [f64; 8] = {list(map(float,b))!r};"
    ]))
    env = {**os.environ, "VIGILODE_MODAL_TABLE": str(tab)}
    compiler = subprocess.check_output([args.rustc, '--version'], text=True).strip()
    subprocess.run([args.rustc, '--edition=2024', '-C','opt-level=3','-C','target-cpu=native',
                    str(here/'bench.rs'),'-o',str(work/'bench')], env=env, check=True)
    out = subprocess.check_output([str(work/'bench')], text=True)
    print(json.dumps(dict(compiler=compiler, system=platform.platform(), rows=out.splitlines(),
                          scope='single native thread, fixed-step, modal input; timings exclude basis transforms and adaptive control'),indent=2))
