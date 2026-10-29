#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
# Install Noto Sans CJK KR, or set VIGILODE_CJK_FONT to an installed font family.
font="${VIGILODE_CJK_FONT:-Noto Sans CJK KR}"
pandoc RESEARCH_REPORT_KO.md --from=markdown+tex_math_dollars+raw_tex \
 --include-in-header=header.tex --variable papersize=a4 \
 --variable geometry:margin=23mm --variable fontsize=10pt \
 --variable colorlinks=true --variable mainfont="$font" \
 --variable sansfont="$font" --variable mainfontoptions=AutoFakeBold=2 --variable mainfontoptions=AutoFakeSlant=0.2 \
 --toc --number-sections --standalone --output VIGILODE_RESEARCH_REPORT_KO.tex
xelatex -interaction=nonstopmode -halt-on-error VIGILODE_RESEARCH_REPORT_KO.tex
xelatex -interaction=nonstopmode -halt-on-error VIGILODE_RESEARCH_REPORT_KO.tex
xelatex -interaction=nonstopmode -halt-on-error VIGILODE_RESEARCH_REPORT_KO.tex
