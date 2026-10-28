# Task-local recovery; no user-global environment modified.
export VIGILODE_RUST1024_PREFIX="/workspace/scratch/0dc1e721cf82/runtime1024/rust-1.94.1"
export PATH="$VIGILODE_RUST1024_PREFIX/bin:$PATH"
export LD_LIBRARY_PATH="$VIGILODE_RUST1024_PREFIX/lib:${LD_LIBRARY_PATH:-}"
export CARGO_HOME="/workspace/scratch/0dc1e721cf82/runtime1024/cargo"
export CARGO_TARGET_DIR="/workspace/scratch/0dc1e721cf82/runtime1024/target"
export CARGO_BUILD_JOBS=2
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export RAYON_NUM_THREADS=1
export OMP_NUM_THREADS=1
export OPENBLAS_NUM_THREADS=1
export MKL_NUM_THREADS=1
