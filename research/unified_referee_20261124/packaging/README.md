# VigilODE source SDK

The SDK contains the tracked Cargo workspace, solver coefficient fixtures, licenses and a checksummed vendor directory for the locked registry graph. A consumer uses a single direct path dependency on `vigilode`. Enable its `structured` feature for the declared cubic models. Rust 1.94.1 with its host linker, Python 3.11+, GNU tar and gzip are prerequisites. Packaging additionally needs the locked crate sources in Cargo's local cache. In a connected builder, `cargo fetch --locked` fills that cache.

From the repository root, build a commit-pinned tarball and unpack it into a fresh directory.

```sh
python3 research/unified_referee_20261124/packaging/pack.py --revision HEAD --output /tmp/vigilode-sdk.tar.gz
mkdir -p /tmp/vigilode-sdk-extracted
tar -xzf /tmp/vigilode-sdk.tar.gz -C /tmp/vigilode-sdk-extracted
cd /tmp/vigilode-sdk-extracted/vigilode-sdk
python3 research/unified_referee_20261124/packaging/smoke.py .
```

The smoke program creates a new application, an empty Cargo home and a clean target directory. Its two offline locked runs check a native stiff exponential decay and a source-declared cubic against analytic solutions. A downstream offline application should use a directory-source replacement pointing to the extracted `vendor/` from its own Cargo configuration. Copying this SDK's root Cargo.lock into a new application seeds the resolution. Cargo then writes the application's own lock. Preserve it for subsequent `--locked` builds.

Qualification covers the Linux source consumer with default and `structured` features. Optional integrator research-authority features, research datasets and tests that load historical experiments require the complete repository.
