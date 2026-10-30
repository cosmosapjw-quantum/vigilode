# Reproduce physically measured source-frozen boundary-source memory

1. Extract this full ZIP to an empty directory and do NOT update any original owner/source ZIP bytes.
2. Dependencies CPython 3, numpy, scipy, matplotlib; original author used source conditions in included original corrected PR145 ZIP. `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1` for isolation.
3. `bash VERIFY.sh` checks exact manifest, original 50x193 source/parent identity, 19 physical algebra/negative tests, regenerates 9 output files (5 numeric +4 plots) and compares all 9 output SHA256 bytewise.
4. For manual replay: `python3 code/compute_boundary_memory.py --source source/PR145_CORRECTED_FULL_SOURCE.zip --output /tmp/vigilode-replay/results`.
5. `results/SOURCE_FROZEN_BOUNDARY_MEMORY_50x193.npz` has full 50x193x4 `signed_memory`, `gross_memory`, `original_flux`, `simpson_reference`; CSV subsets and SVG/PNG plots are for source-free inspection.
6. `checkpoints/R1..R10,C1..C10` record 20 *retrospective* scope milestones, not new scientific experiments or 20 separate remote upload receipts. The package is checkpointed and published as one research unit; per-stage full-provider acknowledgements, if any, must be reported separately.

Hash mismatch, bad source, negative g, epoch misalignment or source flux alteration must hard-fail. No new IVP. Post-processed `M_L` is not actual global truncation error. Original BDF time/global source error unresolved. Full PSTF/observer boost and real self-consistent J/E dynamics are future candidates, not in this release.
