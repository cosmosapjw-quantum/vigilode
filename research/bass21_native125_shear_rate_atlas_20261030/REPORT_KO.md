# BASS21 native125 / matched25 원본 물리 데이터 보고서

**Date:** 2026-10-30 KST. **Model scope:** original source-owned BASS finite21 + prescribed thermal/UV bath, z20 to z2, shear 5 x matter 5 x radiation 5. Existing VigilODE PR122 five full-history pairs not invalidated. No new IVP or observed CMB inference.

## Generated scientific data

- `results/BASS125_NATIVE_ENDPOINT_CHEMISTRY.csv`: 125 saved endpoint native proper-time F/charge source, ne, nb, T, UV, photo, RR, DR and power components, original forward tau and local scattering rate.
- `results/BASS125_ACTUAL_NATIVE_FJ.npz`: 125x21 proper s^-1 original F, 125x21x21 original analytic J proper s^-1, 125x21 original Y and case-id array, all original `AtomicNetwork.evaluate`.
- `results/BASS25_MATCHED_SHEAR_OPACITY.csv`: 25 matched initial Sigma0->.6 original stored Delta tau and exact endpoint Delta gamma geometry/ionization split.
- `results/BASS25_SHEAR_SECANT_CURVATURE.json`: all five source-defined initial shear levels, four correct nonuniform secant slopes per cell, three divided curvatures.
- `results/SCIENCE_RESULTS.json` plus two plotted PNGs; exact original source CSV byte and nested immutable parent source ZIP.

Full grid 125/125 and 25/25 pairs. Photon power components do not imply actual thermal gas heating. Endpoint electron differences are tiny; integrated PR122 five-pair chemistry is still substantial. Sampled concavity 25/25 in secant slope sense. Low-shear small sign changes 7/25 are **numerical observations only** (strict solver-error promotion blocked). Source-native photo, RR and net electron rates are specific to this model at original z=2; not new data from experiment.

## Actual validation

`python code/endpoint_atlas.py --write` executed; `python -m unittest discover -s tests -p 'test_*.py' -v` **14/14 PASS** (proper source hash, 127 original rows/125 grid, full native F/J, independent analytic-J formula at 7 representative states, charge/mass, 25 exact Fraction product identities, 25 nonuniform secant slopes, original PR118 25-pair mean, PR122 5-pair exact comparisons, no IVP path). `python code/plot_physics.py` created two source-output visualizations. All 125 native source electron densities identical to original saved numbers to binary64 representation; max baryon sum residual 1.78e-15, max A·F 6.69e-34 s^-1, max charge source balance 6.18e-33 m^-3s^-1. New result's own source-level tests are not native C6/C7 solver tests.

Independent R6 and C9 different-author review NOT_RUN: current runtime native subagent action unavailable. Wolfram symbolic exact polarization evaluated separately, output 0; local 25 rational checks are implementation independent from float arithmetic but share the same original CSV inputs. No claim of source coefficient JSON whole-file raw byte lock beyond previous numeric projection.

## Publication/backup/status

Local source-full archive, original owner repo and frozen parent identities must be kept immutable. Historical PR122 research-ledger CI failure due missing true prereg/ledger witness is NOT retroactively converted into PASS. GitHub Draft publication contains only actually pushed files with actual connector ACKs. Remote full archive backup and restoration have separate receipts; a local zip and provider upload metadata are not independent deep scientific audit. The chapter's single node does not increase original authoritative DAG numerators.

**Progress:** original VigilODE **7/23**, central REI **28/88**, REI working **33/88**, successor **0/12**; current auxiliary native125/matched25 **1/1 SCOPED_DATA, independent review HOLD**. Next physically substantive action is to restore 40 additional original `trajectory.npz` files and extend the full time-history ionization/geometry integral from **5/25 to 25/25** or establish a common-source actual REI event/clock provider without model identity shortcut.
