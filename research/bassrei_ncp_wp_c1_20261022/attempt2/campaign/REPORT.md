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

- S1-G1: ds_tsit45 rtol 1e-06 0.000702s, vg_client_exact rtol 1e-04 0.00201s, vg_banded rtol 1e-04 0.00483s, vg_dense rtol 1e-04 0.00526s, vg_client_krylov rtol 1e-04 0.0108s, ds_bdf_dense rtol 1e-07 0.0261s, py_dop853 rtol 1e-04 0.0833s, py_vode_bdf_mf24 rtol 1e-07 0.0887s
- S1-G2: ds_tsit45 rtol 1e-04 0.00132s, vg_client_exact rtol 1e-03* 0.00706s, vg_banded rtol 1e-03* 0.0286s, vg_client_krylov rtol 1e-03* 0.0321s, vg_dense rtol 1e-03* 0.0692s, py_dop853 rtol 1e-04 0.0719s, py_vode_adams_mf10 rtol 1e-06 0.126s, py_vode_bdf_mf24 rtol 1e-07 0.215s
- S1-G3: ds_tsit45 rtol 1e-04 0.00797s, vg_client_exact rtol 1e-03* 0.0239s, vg_client_krylov rtol 1e-03* 0.123s, py_vode_adams_mf10 rtol 1e-06 0.228s, py_vode_bdf_mf24 rtol 1e-06 0.266s, py_dop853 rtol 1e-03* 0.342s, vg_banded rtol 1e-03* 0.395s, py_vode_bdf_mf25 rtol 1e-06 1.14s
- S2-G1: vg_client_exact rtol 1e-03* 0.00108s, vg_dense rtol 1e-03* 0.00351s, vg_banded rtol 1e-03* 0.00384s, vg_client_krylov rtol 1e-03* 0.0179s, ds_bdf_dense rtol 1e-06 0.0282s, py_vode_bdf_mf24 rtol 1e-06 0.149s, py_vode_bdf_mf21 rtol 1e-06 0.169s, py_vode_bdf_mf25 rtol 1e-06 0.172s
- S2-G2: vg_client_exact rtol 1e-03* 0.00772s, vg_banded rtol 1e-03* 0.0393s, vg_client_krylov rtol 1e-03* 0.0796s, vg_dense rtol 1e-03* 0.0884s, py_vode_bdf_mf24 rtol 1e-06 0.233s, py_vode_bdf_mf25 rtol 1e-06 0.263s, ds_bdf_dense rtol 1e-06 1.94s, py_dop853 rtol 1e-08 45s
- S2-G3: vg_client_exact rtol 1e-03* 0.0199s, vg_client_krylov rtol 1e-03* 0.239s, vg_banded rtol 1e-03* 0.423s, py_vode_bdf_mf24 rtol 1e-06 0.77s, py_vode_bdf_mf25 rtol 1e-06 1.25s, py_dop853 rtol 1e-08 140s, ds_bdf_dense rtol 1e-05 147s
- S2-G4: vg_client_exact rtol 1e-03* 0.0787s, vg_client_krylov rtol 1e-03* 1.27s, py_vode_bdf_mf24 rtol 1e-05 4.19s, vg_banded rtol 1e-03* 6.3s, py_vode_bdf_mf25 rtol 1e-05 10.7s
- S3-G1: vg_bordered_client rtol 1e-07 0.0102s, vg_bordered_dense rtol 1e-07 0.0358s, ds_bdf_dense rtol 1e-09 0.101s, py_vode_bdf_mf21 rtol 1e-09 0.288s, py_vode_bdf_mf25 rtol 1e-10 0.377s, py_vode_bdf_mf24 rtol 1e-10 0.447s, py_vode_bdf_mf22 rtol 1e-10 0.916s
- S3-G2: vg_bordered_client rtol 1e-07 0.0412s, vg_bordered_dense rtol 1e-07 0.716s, py_vode_bdf_mf24 rtol 1e-10 0.965s, ds_bdf_dense rtol 1e-09 4.88s
- S4N1: ds_tsit45 rtol 1e-05 0.00587s, vg_chem_dense rtol 1e-03* 0.0538s, py_vode_adams_mf10 rtol 1e-06 0.0755s, py_dop853 rtol 1e-06 0.397s, py_vode_bdf_mf21 rtol 1e-07 0.54s, ds_bdf_dense rtol 1e-05 1.26s, py_vode_bdf_mf22 rtol 1e-07 1.61s, py_dop853_tight rtol 3e-11 28.1s
- S4R: vg_chem_dense rtol 1e-03* 0.173s, ds_bdf_dense rtol 1e-06 2.4s, py_vode_bdf_mf21 rtol 1e-07 2.46s, py_vode_bdf_mf22 rtol 1e-10 35.4s
- S4S: vg_chem_dense rtol 1e-04 0.139s, py_vode_bdf_mf21 rtol 1e-07 1.97s, ds_bdf_dense rtol 1e-08 3.04s, py_vode_bdf_mf22 rtol 1e-07 10.9s, py_dop853 rtol 1e-08 13.5s

## ABBA (B/A solve-time medians, diagnostic_only)

- S1-G2:vg_client_exact@1e-03~ds_tsit45@1e-04: 0.30900979038539544 over 7 OK blocks {'OK': 14}
- S1-G2:vg_client_exact@1e-03~py_dop853@1e-04: 13.421697028592854 over 7 OK blocks {'OK': 14}
- S1-G2:vg_client_exact@1e-03~py_vode_adams_mf10@1e-06: 24.846004279198088 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~ds_tsit45@1e-04: 0.24543830604814432 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~py_dop853@1e-03: 8.27958260413808 over 7 OK blocks {'OK': 14}
- S1-G3:vg_client_exact@1e-03~py_vode_adams_mf10@1e-06: 19.126270469634267 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~ds_bdf_dense@1e-06: 114.24494232015986 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~py_dop853@1e-08: 5804.680703638444 over 7 OK blocks {'OK': 14}
- S2-G2:vg_client_exact@1e-03~py_vode_bdf_mf24@1e-06: 27.238236994423392 over 7 OK blocks {'OK': 14}
- S2-G3:vg_client_exact@1e-03~ds_bdf_dense@1e-05: skipped (matched run 147 s > 120.0 s)
- S2-G3:vg_client_exact@1e-03~py_dop853@1e-08: skipped (matched run 140 s > 120.0 s)
- S2-G3:vg_client_exact@1e-03~py_vode_bdf_mf24@1e-06: 18.204077992437377 over 7 OK blocks {'OK': 14}
- S2-G4:vg_client_exact@1e-03~py_vode_bdf_mf24@1e-05: 32.36259812246658 over 7 OK blocks {'OK': 14}
- S3-G2:vg_bordered_client@1e-07~ds_bdf_dense@1e-09: 126.71065736038666 over 7 OK blocks {'OK': 14}
- S3-G2:vg_bordered_client@1e-07~py_vode_bdf_mf24@1e-10: 20.93067156051274 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~ds_tsit45@1e-05: 0.09725895820997044 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~py_dop853@1e-06: 7.201605458082357 over 7 OK blocks {'OK': 14}
- S4N1:vg_chem_dense@1e-03~py_vode_adams_mf10@1e-06: 1.7563149058738785 over 7 OK blocks {'OK': 14}
- S4R:vg_chem_dense@1e-03~ds_bdf_dense@1e-06: 17.20760089782183 over 7 OK blocks {'OK': 14}
- S4R:vg_chem_dense@1e-03~py_vode_bdf_mf21@1e-07: 15.792658841453163 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~ds_bdf_dense@1e-08: 20.282913347803618 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~py_dop853@1e-08: 75.98483591260393 over 7 OK blocks {'OK': 14}
- S4S:vg_chem_dense@1e-04~py_vode_bdf_mf21@1e-07: 11.618444545035038 over 7 OK blocks {'OK': 14}
