# Diffsol (`0.16.1`) Comparative Benchmark Harness

This directory contains the standalone benchmark harness comparing VigilODE (`rodas5p_fast` and `rodas5p_fast_small`) against the standard Rust ODE package `diffsol` (`0.16.1`) across Robertson, van der Pol ($\mu=1000$), HIRES, and Brusselator 1D problems.

## Why Standalone?

Under VigilODE's **E4 Offline Dependency Freeze** governance rule, root `Cargo.toml` and `Cargo.lock` must remain strictly frozen and offline reproducible. External comparators requiring external registry packages (like `diffsol` or `nalgebra`) are isolated from the main workspace to protect offline reproducibility.

## How to Run

To run this benchmark locally:

```bash
# 1. Create a scratch directory outside or inside your build tree
mkdir -p /tmp/diffsol_bench/src
cp tools/benchmarks/diffsol/Cargo.toml.example /tmp/diffsol_bench/Cargo.toml
cp tools/benchmarks/diffsol/main.rs /tmp/diffsol_bench/src/main.rs

# 2. Build and run in release mode
cd /tmp/diffsol_bench
cargo run --release
```

## Results & Report

The empirical results and comprehensive architectural analysis are documented in:
- Report: [`docs/DIFFSOL_BENCHMARK_REPORT.md`](../../docs/DIFFSOL_BENCHMARK_REPORT.md)
- Raw Dataset: [`docs/artifacts/diffsol_benchmark_results.json`](../../docs/artifacts/diffsol_benchmark_results.json)
