# Causal Moment Packet: RVJ 독립 연구12

원래서로다른모드의초기값과주파수를모두보존하면서collectivefeedback을scalarVolterra equation으로계산한다. Kernel approximation과실제원n-mode모델사이오차를인증한다. Core는Python표준라이브러리만사용하며onlineJ/JVP/VJP/finite-difference/Krylov/factorization호출이없다.

읽기순서: RESEARCH_NOTE_KO.md → CLAIM_LEDGER.json → COST_CONTRACT.json → FINAL_VERIFICATION.json → HANDOFF_KO.md.

`bash REPLAY.sh`는unit/exactalgebra를실행한다. `bash REPLAY.sh --study`는추가numericalcampaign을실행한다. Core는임의ODEsolver가아니며kappa0,knownfrequencypacket,mean-fieldcubicclient다. Commonkappa>=0의이론은있지만dampedclient는미구현이다.

`rvj_packet/packet.py`:exactcomplexrational,causalpolynomialconvolution,momentcompiler,candidate/certificate,reconstruction.
`rvj_packet/convergence.py`:명시된analyticball에서사전오차·자원충분조건.
`tests/`:정확계약및negativecontrol.
`run_studies.py`,`extra_studies.py`:원모델oracle와same-mapoperation-count비교.

NoVigiloderepo/PRmutation. Noindependentreview. PromotionHOLD.
