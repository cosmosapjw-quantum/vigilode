#!/usr/bin/env python3
"""Build the Korean design document with pandoc and XeLaTeX.
Set VIGILODE_DOC_FONT_DIR to a directory containing NotoSansKR-Regular.ttf and
NotoSansKR-Bold.ttf. Fonts are not vendored; source is recorded in SOURCE_NOTES.md.
"""
import os, subprocess, re
from pathlib import Path
P=Path(__file__).resolve().parent
fonts=Path(os.environ.get('VIGILODE_DOC_FONT_DIR','/tmp/vigilode-doc-fonts'))
for n in ['Regular','Bold']:
 if not (fonts/f'NotoSansKR-{n}.ttf').exists(): raise SystemExit('Missing NotoSansKR static fonts; see SOURCE_NOTES.md')
header=r'''\usepackage{fontspec}
\usepackage{needspace}
\microtypesetup{protrusion=false,expansion=false}
\setmainfont[Path=FONTPATH/,BoldFont=NotoSansKR-Bold.ttf,ItalicFont=NotoSansKR-Regular.ttf,ItalicFeatures={FakeSlant=0.15}]{NotoSansKR-Regular.ttf}
\setsansfont[Path=FONTPATH/,BoldFont=NotoSansKR-Bold.ttf]{NotoSansKR-Regular.ttf}
\setmonofont{DejaVu Sans Mono}[Scale=0.78]
\XeTeXlinebreaklocale "ko"
\XeTeXlinebreakskip=0pt plus 0.12em
\setlength{\emergencystretch}{3em}
\setlength{\parindent}{0pt}
\setlength{\parskip}{5pt plus 1pt minus 1pt}
\allowdisplaybreaks
\renewcommand{\contentsname}{목차}
\renewcommand{\figurename}{그림}
\renewcommand{\tablename}{표}
\usepackage{fancyhdr}
\pagestyle{fancy}
\fancyhf{}
\fancyhead[L]{\footnotesize VigilODE 수학적 설계}
\fancyhead[R]{\footnotesize 범용 상미분방정식 적분}
\fancyfoot[C]{\thepage}
\setlength{\headheight}{15pt}
'''.replace('FONTPATH',str(fonts))
(P/'header.tex').write_text(header)
subprocess.run(['pandoc','DESIGN_KO.md','-s','--top-level-division=section','-H','header.tex','-V','documentclass=article','-V','classoption=dvipsnames','-o','DESIGN.tex'],cwd=P,check=True)
t=(P/'DESIGN.tex').read_text()
t=t.replace(r'\begin{longtable}',r'\Needspace{8\baselineskip}'+'\n'+r'\begin{longtable}',1)
for name in ['dopri5_stability','accepted_state_flow','causal_finite_propagation','quotient_defect_contraction']:
 t=t.replace(f'figures/{name}.png',f'figures/{name}.pdf')
# Keep the implementation mapping heading with its table.
t=t.replace(r'\subsection{수학적 결과와 실행 코드의',r'\Needspace{12\baselineskip}'+'\n'+r'\subsection{수학적 결과와 실행 코드의')
# Scale the complete rational tableau as a unit, retaining its equation label.
pattern = r"\\\[\s*(\\renewcommand\{\\arraystretch\}.*?\\end\{array\})\s*\\tag\{4\}\s*\\\]"
def tableau(match):
    return r"\begin{equation}\resizebox{0.98\textwidth}{!}{$" + match.group(1) + r"$}\tag{4}\end{equation}"
t, count = re.subn(pattern, tableau, t, flags=re.S)
if count != 1: raise SystemExit('Expected exactly one DOPRI5 tableau')
(P/'DESIGN.tex').write_text(t)
for _ in range(3):
 r=subprocess.run(['xelatex','-interaction=nonstopmode','-halt-on-error','DESIGN.tex'],cwd=P,capture_output=True,text=True)
 if r.returncode: print(r.stdout[-6000:]);raise SystemExit(r.returncode)
print(P/'DESIGN.pdf')
