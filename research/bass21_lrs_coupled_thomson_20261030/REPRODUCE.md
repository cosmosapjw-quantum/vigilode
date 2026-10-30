# Offline replay

Requirements Python 3.11+ numpy/scipy. In a clean directory containing *this complete ZIP*:

```bash
bash VERIFY.sh
python code/coupled_thomson_volterra.py --archive source/VIGILODE_BASS21_FULL25_NATIVE193_20261030_SOURCE_FULL.zip --out /tmp/bass21_coupled_replay
```

`VERIFY.sh` requires the raw 23,770,654 byte original BASS21 ZIP nested under `source/` and PR130's source-owned original reader under `source_code/`. GitHub's text-only mirror cannot reproduce the numerical results without the full archive. New arrays are in `results/LRS50_CPLD_193x17_POLARIZED_RESPONSE.npz`; their scientific identity is checked by array comparison with predeclared tolerance in `VERIFY.sh`, not by guessing ZIP timestamp bytes. External source 50 SHA checks occur on every replay. Computational angle GL 24/48. Mathematical and physical scope in `THEORY_KO.md`.
