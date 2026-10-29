#!/usr/bin/env bash
set -euo pipefail
vigil_poly_report_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
vigil_poly_repo_dir="$(cd -- "$vigil_poly_report_dir/../../.." && pwd)"
vigil_poly_build_dir="${1:-$vigil_poly_report_dir/build}"
mkdir -p "$vigil_poly_build_dir"
vigil_poly_build_dir="$(cd -- "$vigil_poly_build_dir" && pwd)"
# Reuse the immutable REVTeX support bytes already archived with PR92.
python3 - "$vigil_poly_repo_dir" "$vigil_poly_build_dir" <<'PY'
from pathlib import Path
import sys, tarfile
repo, build = map(Path, sys.argv[1:])
archive = repo / 'research/production_solver_closure_20261027/evidence/raw_measurements.tar.xz'
prefix = 'research/production_solver_closure_20261027/report/tex_support/'
support = build / 'tex_support'
support.mkdir(exist_ok=True)
with tarfile.open(archive) as tar:
    for item in tar:
        if item.isfile() and item.name.startswith(prefix):
            name = item.name[len(prefix):]
            if not name or '/' in name or '\\' in name or name in ('.', '..'):
                raise SystemExit('invalid REVTeX support member')
            (support / name).write_bytes(tar.extractfile(item).read())
if not (support / 'revtex4-2.cls').is_file():
    raise SystemExit('missing inherited REVTeX support')
PY
cd "$vigil_poly_report_dir"
export TEXINPUTS="$vigil_poly_build_dir/tex_support//:${TEXINPUTS-}"
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_poly_build_dir" polyalgorithm_alternatives.tex
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_poly_build_dir" polyalgorithm_alternatives.tex
