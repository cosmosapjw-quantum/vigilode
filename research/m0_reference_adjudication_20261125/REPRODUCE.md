# Commands and evidence
Run from new directory with same relative layout and pinned preflight source.
Source-lock: receipts/BASE.txt, environment.txt, shared_lib_sha256.txt, bridge_sha256.txt, *preregister.sha256 and independent_scripts.sha256. Absolute paths in bridge/Cargo.toml and analysis scripts are owner-host paths; for relocation explicitly substitute the same pinned sources and archive identities. All experiments originally thread-pinned, OMP/OPENBLAS/MKL/RAYON/JULIA threads=1. No timing authority.
- Original replay: cargo +stable build --locked --offline --release --manifest-path crates/vigilode-bench/Cargo.toml; binary references --corpus intake/{canonical,spatial}.corpus.json --out NEW/{canonical,spatial}.json --source-root PINNED_REPO --require-independent-reference (timeout180, CPU2).
- Shared model bridge: cargo +stable build --locked --offline --release --manifest-path bridge/Cargo.toml, copy cdylib as bridge/model_bridge.so. Previous model source diff empty relative to measured 7b8ea57e.
- python3 independent/run_scipy.py (CPU3, 240s total, 120 per trajectory)
- julia --startup-file=no --project=source_inputs independent/run_julia.jl PINNED_REPO spatial (CPU4) and big (CPU5); original big attempt intentionally retained. Equivalent scalar-atol retry run_julia_scalar_atol.jl PINNED_REPO big.
- python3 independent/make_wolfram.py; wolframscript -file independent/{robertson,hires,van-der-pol-mu1000,oregonator}.wls (CPU6); original optional route stopped, see WOLFRAM_STOP.json. Do not overwrite archived outputs.
- python3 independent/run_bruss_capped.py; julia --startup-file=no --project=source_inputs independent/run_bruss_capped.jl PINNED_REPO spatial.
- python3 independent/run_canonical_extra.py
- python3 independent/make_radau7.py; julia --startup-file=no --project=source_inputs independent/oregonator_hp.jl
- PYTHONPATH=PINNED_MATHHARNESS python3 independent/check_radau_exact.py (new ledger path)
- python3 analyze.py; python3 finalize.py; PYTHONPATH=PINNED_MATHHARNESS python3 reproduce_analysis.py. This last command only re-reduces evidence; no solver invocation.
