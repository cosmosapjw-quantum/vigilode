#!/usr/bin/env bash
set -euo pipefail
vigil_df_report_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
vigil_df_output_dir="${1:-$vigil_df_report_dir/build}"
if [[ ! -f "$vigil_df_report_dir/tex_support/revtex4-2.cls" ]]; then
  vigil_df_repo_dir="$(cd -- "$vigil_df_report_dir/../../.." && pwd)"
  tar -xJf "$vigil_df_report_dir/../evidence/raw_measurements.tar.xz" -C "$vigil_df_repo_dir"
fi
mkdir -p "$vigil_df_output_dir"
cd "$vigil_df_report_dir"
export TEXINPUTS="$vigil_df_report_dir/tex_support//:${TEXINPUTS-}"
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_df_output_dir" derivative_free_causal_homotopy.tex
pdflatex -interaction=nonstopmode -halt-on-error -output-directory="$vigil_df_output_dir" derivative_free_causal_homotopy.tex
