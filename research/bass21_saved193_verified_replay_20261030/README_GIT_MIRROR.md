# BASS21 source-owned 193 epoch replay (scoped data)

This is a Git text mirror of the verified single-node physical replay; the **byte-locked full source including binary NPZ and matrices** is archived separately.

- Original BASS owner: `cc602e209e7deeb511ab2a1f3b2268d55206c43c`
- Original trajectory SHA-256: `37d1bab173e81449cf474b21a678077edd0150cf0b2cb386d014764580678086`
- Complete archive: https://drive.google.com/file/d/1JKljLvS5mR_PXooULAC2bHc7ijj6FgLY/view
- Archive bytes: 3,151,209; SHA-256: `19513fec19e53d5f2da8da50ba22aa066c634dee79ff94cff31d07b4cfb8f907`
- New tests 20/20 exit 0; parent 13/13 exit 0; 193 F/J native source calls; no new IVP.
- `VERIFY.sh` requires complete archive extraction, not standalone Git mirror checkout; full source binaries are deliberately external.
- Native R6/C9 NOT_RUN, PR118 research-node CI FAIL remains; original VigilODE 7/23, REI central 28/88, successor 0/12, production HOLD.
- PR117 RHS/value-only derivative-free priority and original gates/tolerances kept intact.
