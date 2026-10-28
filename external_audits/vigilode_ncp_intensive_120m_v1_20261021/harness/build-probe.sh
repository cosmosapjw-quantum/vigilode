#!/bin/bash
set -eu
export PATH=/root/.cargo/bin:/root/.rustup/toolchains/1.94.1-x86_64-unknown-linux-gnu/bin:/usr/bin:/bin
cd /root/vigilode-reaudit-20261021T0549Z/checkout
cargo build --locked --offline -j1 -p rodas5p-integrators --example component_benchmark_native --profile measurement --target-dir /dev/shm/vigilode_ncp_intensive_120m_v1_20261021T062109Z/target
