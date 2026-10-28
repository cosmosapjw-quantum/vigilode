# BASS/rei NCP C1 campaign: PASS

| gate | pass |
|---|---|
| G1_references | True |
| G2_reference_parity | True |
| G3_vigilode_rung | True |
| G4_cross_core_identity | True |
| G5_allocation | True |
| G6_coverage | True |
| G7_provenance | True |

## Matched rungs (loosest rtol meeting B_app/B_src; solve seconds diagnostic_only; * = censored at the loosest rung)

- S1-G1: jl_tsit5 rtol 1e-05 0.00067s, jl_vern7 rtol 1e-03* 0.0008s, ds_tsit45 rtol 1e-06 0.000972s, vg_client_exact rtol 1e-04 0.00259s, jl_cvode_bdf_band rtol 1e-07 0.00271s, jl_cvode_bdf_klu rtol 1e-07 0.00399s, vg_banded rtol 1e-04 0.00399s, jl_rodas5p_sparse rtol 1e-03* 0.00665s
- S1-G2: jl_tsit5 rtol 1e-03* 0.00198s, jl_vern7 rtol 1e-03* 0.00236s, ds_tsit45 rtol 1e-04 0.00282s, vg_client_exact rtol 1e-03* 0.00755s, jl_cvode_bdf_band rtol 1e-07 0.0149s, vg_banded rtol 1e-03* 0.0373s, vg_client_krylov rtol 1e-03* 0.0397s, jl_cvode_bdf_klu rtol 1e-07 0.0472s
- S1-G3: jl_tsit5 rtol 1e-03* 0.00631s, ds_tsit45 rtol 1e-04 0.00828s, jl_vern7 rtol 1e-03* 0.00907s, vg_client_exact rtol 1e-03* 0.0203s, jl_cvode_bdf_band rtol 1e-06 0.125s, vg_client_krylov rtol 1e-03* 0.127s, vg_banded rtol 1e-03* 0.244s, jl_cvode_bdf_klu rtol 1e-06 0.264s
- S2-G1: vg_client_exact rtol 1e-03* 0.00176s, vg_banded rtol 1e-03* 0.00281s, jl_cvode_bdf_band rtol 1e-06 0.00301s, vg_dense rtol 1e-03* 0.00449s, jl_cvode_bdf_klu rtol 1e-06 0.00573s, jl_fbdf_sparse rtol 1e-06 0.00926s, jl_rodas5p_sparse rtol 1e-05 0.0148s, vg_client_krylov rtol 1e-03* 0.0177s
- S2-G2: vg_client_exact rtol 1e-03* 0.00777s, jl_cvode_bdf_band rtol 1e-06 0.0159s, vg_banded rtol 1e-03* 0.0408s, jl_cvode_bdf_klu rtol 1e-06 0.0482s, jl_fbdf_sparse rtol 1e-06 0.0711s, jl_rodas5p_sparse rtol 1e-03* 0.0727s, vg_client_krylov rtol 1e-03* 0.077s, vg_dense rtol 1e-03* 0.104s
- S2-G3: vg_client_exact rtol 1e-03* 0.0298s, jl_cvode_bdf_band rtol 1e-06 0.158s, vg_client_krylov rtol 1e-03* 0.256s, vg_banded rtol 1e-03* 0.399s, py_vode_bdf_mf24 rtol 1e-06 0.617s, jl_cvode_bdf_klu rtol 1e-06 0.746s, jl_rodas5p_sparse rtol 1e-03* 0.98s, py_vode_bdf_mf25 rtol 1e-06 1.04s
- S2-G4: vg_client_exact rtol 1e-03* 0.0741s, vg_client_krylov rtol 1e-03* 1.19s, py_vode_bdf_mf24 rtol 1e-05 3.63s, jl_cvode_bdf_band rtol 1e-05 4.69s, vg_banded rtol 1e-03* 6.82s, jl_cvode_bdf_klu rtol 1e-05 7.85s, jl_fbdf_sparse rtol 1e-05 8.38s, jl_rodas5p_sparse rtol 1e-03* 12.7s
- S3-G1: vg_bordered_client rtol 1e-07 0.00899s, jl_cvode_bdf_band rtol 1e-10 0.0182s, jl_cvode_bdf_klu rtol 1e-09 0.02s, jl_fbdf_sparse rtol 1e-07 0.0215s, vg_bordered_dense rtol 1e-07 0.0322s, jl_rodas5p_sparse rtol 1e-06 0.0516s, ds_bdf_dense rtol 1e-09 0.116s, py_vode_bdf_mf21 rtol 1e-09 0.306s
- S3-G2: vg_bordered_client rtol 1e-07 0.0267s, jl_fbdf_sparse rtol 1e-07 0.199s, jl_cvode_bdf_klu rtol 1e-09 0.274s, vg_bordered_dense rtol 1e-07 0.662s, jl_rodas5p_sparse rtol 1e-05 0.745s, py_vode_bdf_mf24 rtol 1e-10 0.868s, ds_bdf_dense rtol 1e-09 6.94s
- S4N1: jl_tsit5 rtol 1e-04 0.00194s, jl_vern7 rtol 1e-03* 0.00306s, ds_tsit45 rtol 1e-05 0.00437s, jl_cvode_bdf_klu rtol 1e-07 0.0432s, vg_chem_dense rtol 1e-03* 0.0496s, py_vode_adams_mf10 rtol 1e-06 0.129s, jl_rodas5p_sparse rtol 1e-03* 0.146s, py_dop853 rtol 1e-06 0.388s
- S4R: vg_chem_dense rtol 1e-03* 0.154s, jl_cvode_bdf_klu rtol 1e-07 0.188s, jl_rodas5p_sparse rtol 1e-03* 0.432s, py_vode_bdf_mf21 rtol 1e-07 1.74s, ds_bdf_dense rtol 1e-06 3.21s, py_vode_bdf_mf22 rtol 1e-10 32.1s
- S4S: jl_cvode_bdf_klu rtol 1e-07 0.105s, vg_chem_dense rtol 1e-04 0.163s, jl_rodas5p_sparse rtol 1e-05 0.671s, py_vode_bdf_mf21 rtol 1e-07 2.04s, ds_bdf_dense rtol 1e-08 3.52s, py_vode_bdf_mf22 rtol 1e-07 12.3s, py_dop853 rtol 1e-08 13.4s

## ABBA (B/A solve-time medians, diagnostic_only)

- S1-G2:vg_client_exact@1e-03~ds_tsit45@1e-04: 0.27968279089682985 over 7 OK blocks {'OK': 14}
- S1-G2:vg_client_exact@1e-03~jl_tsit5@1e-03: 0.27499537577991423 over 7 OK blocks {'OK': 14}
- S1-G2:vg_client_exact@1e-03~py_dop853@1e-04: 13.728179679290703 over 7 OK blocks {'OK': 14}
- S1-G2:vg_client_exact@1e-03~py_vode_adams_mf10@1e-06: 24.044915055427644 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~ds_tsit45@1e-04: 0.2784124761020191 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~jl_tsit5@1e-03: 0.22176701854012026 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~py_dop853@1e-03: 5.331626019845938 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~py_vode_adams_mf10@1e-06: 20.65884032459638 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~ds_bdf_dense@1e-06: 100.73211674488755 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~jl_cvode_bdf_band@1e-06: 2.8590917365654884 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~py_dop853@1e-08: 5843.670796300781 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~py_vode_bdf_mf25@1e-06: 37.74371181575207 over 7 OK blocks {'OK': 14}
- S2-G3:vg_client_exact@1e-03~ds_bdf_dense@1e-05: skipped (matched run 186 s > 120.0 s)
- S2-G3:vg_client_exact@1e-03~jl_cvode_bdf_band@1e-06: 5.835381953438422 over 7 OK blocks {'OK': 14}
- S2-G3:vg_client_exact@1e-03~py_dop853@1e-08: skipped (matched run 211 s > 120.0 s)
- S2-G3:vg_client_exact@1e-03~py_vode_bdf_mf24@1e-06: 18.185240222642797 over 7 OK blocks {'OK': 14}
- S2-G4:vg_client_exact@1e-03~jl_cvode_bdf_band@1e-05: 15.922561008608993 over 7 OK blocks {'OK': 14}
- S2-G4:vg_client_exact@1e-03~py_vode_bdf_mf24@1e-05: 32.708359450857614 over 7 OK blocks {'OK': 14}
- S3-G2:vg_bordered_client@1e-07~ds_bdf_dense@1e-09: 124.32708608814727 over 7 OK blocks {'OK': 14}
- S3-G2:vg_bordered_client@1e-07~jl_fbdf_sparse@1e-07: 5.614135649352438 over 7 OK blocks {'OK': 14}
- S3-G2:vg_bordered_client@1e-07~py_vode_bdf_mf24@1e-10: 19.328588302491468 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~ds_tsit45@1e-05: 0.11335851587663158 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~jl_tsit5@1e-04: 0.04960083781998737 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~py_dop853@1e-06: 7.686000839781264 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~py_vode_adams_mf10@1e-06: 1.7216244681982429 over 7 OK blocks {'OK': 14}
- S4R:vg_chem_dense@1e-03~ds_bdf_dense@1e-06: 17.04973771323588 over 7 OK blocks {'OK': 14}
- S4R:vg_chem_dense@1e-03~jl_cvode_bdf_klu@1e-07: 1.2568776094557375 over 7 OK blocks {'OK': 14}
- S4R:vg_chem_dense@1e-03~py_vode_bdf_mf21@1e-07: 15.578645666084439 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~ds_bdf_dense@1e-08: 20.706868875373928 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~jl_cvode_bdf_klu@1e-07: 0.8907172086979162 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~py_dop853@1e-08: 75.68718932244695 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~py_vode_bdf_mf21@1e-07: 11.201694265229703 over 7 OK blocks {'OK': 14}
