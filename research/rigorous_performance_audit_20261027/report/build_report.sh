#!/usr/bin/env bash
set -euo pipefail
vigil_report_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
vigil_output_dir="${1:-$vigil_report_dir/build}"
mkdir -p "$vigil_output_dir"
cd "$vigil_report_dir"
export TEXINPUTS="$vigil_report_dir/tex_support//:${TEXINPUTS-}"
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_output_dir" vigilode_audit.tex
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_output_dir" vigilode_audit.tex
