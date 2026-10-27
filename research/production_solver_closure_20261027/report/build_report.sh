#!/usr/bin/env bash
set -euo pipefail
vigil_prod_report_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
vigil_prod_output_dir="${1:-$vigil_prod_report_dir/build}"
vigil_prod_repo_dir="$(cd -- "$vigil_prod_report_dir/../../.." && pwd)"
if [[ ! -f "$vigil_prod_report_dir/tex_support/revtex4-2.cls" ]]; then
  python3 "$vigil_prod_report_dir/../verify_package.py" --extract-evidence
fi
mkdir -p "$vigil_prod_output_dir"
cd "$vigil_prod_report_dir"
export TEXINPUTS="$vigil_prod_report_dir/tex_support//:${TEXINPUTS-}"
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_prod_output_dir" production_solver_closure.tex
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_prod_output_dir" production_solver_closure.tex
