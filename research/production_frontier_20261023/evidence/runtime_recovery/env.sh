# Task-local Rust recovery. Source from any checkout; no user-global install.
export VIGILODE_TREE_RUST_PREFIX="/workspace/scratch/0dc1e721cf82/task_runtime_tree1023/rust-1.94.1"
export PATH="$VIGILODE_TREE_RUST_PREFIX/bin:$PATH"
export LD_LIBRARY_PATH="$VIGILODE_TREE_RUST_PREFIX/lib:${LD_LIBRARY_PATH:-}"
export CARGO_HOME="/workspace/scratch/0dc1e721cf82/task_runtime_tree1023/cargo"
export CARGO_TARGET_DIR="/workspace/scratch/0dc1e721cf82/task_runtime_tree1023/target"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export MKL_NUM_THREADS=1
