# Source and reproducibility notes

The Korean reader document describes the selected general ODE interface and method-specific observation interfaces. It contains no application-specific physics or timing comparisons. Numerical assumptions are stated next to each proposition.

## Primary literature

- Dormand and Prince (1980), DOI 10.1016/0771-050X(80)90013-3: publisher search metadata confirms title, authors and embedded RK5(4) family. Direct publisher fetch returned HTTP 403. The exact tableau used in this document was read from the repository, then checked with exact-rational arithmetic; a full article read is not claimed.
- Steinebach (2023), DOI 10.1007/s10543-023-00967-x: retrieved via SciSpace search and the Springer publisher full-text page. Used for method attribution, intended order, stiff-order caveats and continuous extension order. No performance ranking from the paper was transferred to this implementation.
- Shampine (1986), DOI 10.1090/S0025-5718-1986-0815836-3: bibliographic attribution and actual interpolation coefficient table are recorded in `crates/rodas5p-integrators/src/dopri5_dense.rs`; publisher fetch returned HTTP 403. No independent full-paper read is claimed.

## Mathematical and source checks

The DOPRI5 tableau, 17 fifth-order rooted-tree identities, eight fourth-order embedded identities, scalar stability polynomial and oscillator inequality were checked against `research/general_ode_selection_20261024/analysis/NUMERICS_CHECK.json` and its reproducible `verify_numerics.py`. The root-tree proof in the document uses exact rational coefficients, not a floating-point tolerance test. The RODAS5P stage count and raw/transformed coordinate conventions were read from the coefficient snapshot and native implementation. The formal document does not infer nonlinear fifth order of an arbitrary approximate-Jacobian implementation from a scalar stability-series check.

The general API and session contracts were checked directly against `crates/vigilode/src/selection.rs` and `crates/vigilode/src/session.rs`. The derivative permissions, dense materialization ceiling and session termination semantics are reflected in `SPECIFICATION.json`.

## Writing tools and their limitations

The `my-writing-style-research:my-writing-style` skill and its `WRITING_PROTOCOL_V4.md` were read. The Korean academic register uses explicit premises and equation-led exposition; English stylistic frequencies were not imposed on Korean prose.

Academic Writing Toolkit was actually used for paragraph logic, IEEE citation consistency, reading-note format and BibTeX syntax validation. Raw responses are in `editorial/TOOLKIT_REVIEW.json`. The paragraph tool's short-paragraph warnings include compact Korean prose, tables and displayed mathematics; the theorem structure was retained after manual inspection. The initial citation input used a malformed reading-note format. Single-source reruns use the toolkit template: source 1 passes, while source 2 and 3 snippets report missing earlier numeric citations because they are isolated snippets. The complete document has references [1], [2], [3] in order. The BibTeX check reports zero syntax issues; it performs no online metadata validation and is not represented as such.

## Rendering

Run `python make_figures.py` then `python render.py` from this directory (or use absolute script paths). Dependencies: Python with matplotlib/numpy, pandoc, XeLaTeX with fontspec/needspace/fancyhdr, static NotoSansKR regular and bold font files. Set `VIGILODE_DOC_FONT_DIR` if they are not in `/tmp/vigilode-doc-fonts`.

Font source: `https://raw.githubusercontent.com/google/fonts/main/ofl/notosanskr/NotoSansKR%5Bwght%5D.ttf`, licensed under SIL Open Font License. Static faces were generated using fontTools `instantiateVariableFont` with wght=400 and 700. The font is embedded in the PDF and not redistributed separately in this directory. To regenerate the static files, load the downloaded variable font with `TTFont`, call `instantiateVariableFont(font, {'wght': weight}, inplace=True)`, and save `NotoSansKR-Regular.ttf` or `NotoSansKR-Bold.ttf` into the configured font directory.

`DESIGN.tex` is generated from `DESIGN_KO.md`. Its header records the local font path from the build; `render.py` regenerates that path for another host. Figures are provided as PDF, SVG and PNG; their values are algebraic quantities from the stated equations, not benchmark observations.
