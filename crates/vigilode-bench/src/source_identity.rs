//! Content identity for the repository-owned dependency closure.
//!
//! Uses the same `sha256sum` tool already required by the native build. Git is
//! optional context, never the content identity. The snapshot deliberately
//! includes all workspace crates (a superset of this binary's path dependencies),
//! locks, fixtures, native bridges and literal Rust compile-time includes.
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Manifest wire format; paths are relative to the repository root.
pub const SCHEMA: &str = "vigilode-bench.source.v1";

fn safe_path(path: &Path) -> Result<String, String> {
    let text = path.to_str().ok_or("non-UTF8 source path")?;
    if text.contains(['\n', '\r', '\t', '\\']) {
        return Err(format!("unsupported source path: {path:?}"));
    }
    Ok(text.to_owned())
}

fn visit(root: &Path, path: &Path, files: &mut BTreeSet<String>) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.file_type().is_symlink() {
        return Err(format!("source symlink refused: {}", path.display()));
    }
    if meta.is_dir() {
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            // Build products and Git internals are not source inputs.
            if entry.file_name() != "target" && entry.file_name() != ".git" {
                visit(root, &entry.path(), files)?;
            }
        }
    } else if meta.is_file() {
        files.insert(safe_path(
            path.strip_prefix(root).map_err(|e| e.to_string())?,
        )?);
    } else {
        return Err(format!("non-regular source input: {}", path.display()));
    }
    Ok(())
}

fn quoted(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start().strip_prefix('"')?;
    let end = text.find('"')?;
    (!text[..end].contains('\\')).then_some((&text[..end], &text[end + 1..]))
}

// Literal includes are resolved relative to the containing Rust file. The only
// supported computed includes are Cargo-manifest-relative paths and this build's
// own generated source manifest. Unrecognized expressions fail closed.
fn includes(root: &Path, relative: &str, text: &str) -> Result<Vec<PathBuf>, String> {
    let source = root.join(relative);
    let mut result = Vec::new();
    for name in ["include", "include_str", "include_bytes"] {
        let marker = format!("{name}!");
        let mut rest = text;
        while let Some(index) = rest.find(&marker) {
            rest = &rest[index + marker.len()..];
            let Some(body) = rest.trim_start().strip_prefix('(') else {
                continue;
            };
            let body = body.trim_start();
            if let Some((literal, _)) = quoted(body) {
                result.push(source.parent().unwrap().join(literal));
            } else {
                let end = body.find("));").unwrap_or(body.len());
                let expression: String =
                    body[..end].chars().filter(|c| !c.is_whitespace()).collect();
                let manifest_prefix = "concat!(env!(\"CARGO_MANIFEST_DIR\"),";
                if let Some(tail) = expression.strip_prefix(manifest_prefix) {
                    let (suffix, _) =
                        quoted(tail).ok_or("unsupported manifest-relative include")?;
                    let crate_dir = source
                        .ancestors()
                        .find(|p| p.join("Cargo.toml").is_file())
                        .ok_or("include has no containing crate")?;
                    result.push(crate_dir.join(suffix.trim_start_matches('/')));
                } else if relative == "crates/vigilode-bench/src/main.rs"
                    && expression.starts_with("concat!(env!(\"OUT_DIR\"),\"/source_manifest.txt\"")
                {
                    // Generated from this very snapshot and embedded by rustc.
                } else {
                    return Err(format!(
                        "unsupported computed include in {relative}: {name}"
                    ));
                }
            }
        }
    }
    Ok(result)
}

fn repository_input(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            part => {
                normalized.push(part.as_os_str());
                if std::fs::symlink_metadata(&normalized).is_ok_and(|m| m.file_type().is_symlink())
                {
                    return Err(format!("input symlink refused: {}", normalized.display()));
                }
            }
        }
    }
    if !normalized.starts_with(root) {
        return Err("source input outside repository".into());
    }
    Ok(normalized)
}

// Cargo path entries in this repository use ordinary quoted TOML strings. We
// bind target paths too (a conservative superset), and refuse unsupported path
// expressions rather than silently omit a dependency. Registry dependencies
// are bound by their lock checksums, which Cargo verifies.
fn manifest_paths(manifest: &Path, text: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.starts_with('#') {
            continue;
        }
        let mut rest = line;
        while let Some(index) = rest.find("path") {
            let before = &rest[..index];
            rest = &rest[index + 4..];
            if before
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                continue;
            }
            let Some(value) = rest.trim_start().strip_prefix('=') else {
                continue;
            };
            let (path, _) =
                quoted(value).ok_or("unsupported Cargo path entry; use an ordinary quoted path")?;
            paths.push(manifest.parent().unwrap().join(path));
        }
    }
    Ok(paths)
}

/// Inventory independent of Git. Reject symlinks and external compile-time assets.
pub fn source_files(root: &Path) -> Result<Vec<String>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut files = BTreeSet::new();
    for required in ["Cargo.toml", "Cargo.lock", "crates", "fixtures"] {
        visit(&root, &root.join(required), &mut files)?;
    }
    for optional in ["rust-toolchain.toml", "rust-toolchain", ".cargo"] {
        if root.join(optional).exists() {
            visit(&root, &root.join(optional), &mut files)?;
        }
    }
    // Include closure: included Rust source can itself include further assets.
    let mut scanned = BTreeSet::new();
    loop {
        let pending: Vec<_> = files
            .iter()
            .filter(|p| (p.ends_with(".rs") || p.ends_with("Cargo.toml")) && !scanned.contains(*p))
            .cloned()
            .collect();
        if pending.is_empty() {
            break;
        }
        for relative in pending {
            scanned.insert(relative.clone());
            let text = std::fs::read_to_string(root.join(&relative)).map_err(|e| e.to_string())?;
            let inputs = if relative.ends_with("Cargo.toml") {
                manifest_paths(&root.join(&relative), &text)?
            } else {
                includes(&root, &relative, &text)?
            };
            for path in inputs {
                let normalized = repository_input(&root, &path)?;
                visit(&root, &normalized, &mut files)?;
            }
        }
    }
    Ok(files.into_iter().collect())
}

/// SHA-256 of supplied bytes using the pinned native-build tool interface.
pub fn sha256_bytes(bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("sha256sum unavailable: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("sha256sum stdin unavailable")?
        .write_all(bytes)
        .map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "sha256sum failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let text = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
    let digest = text.split_whitespace().next().ok_or("missing SHA-256")?;
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid SHA-256 output".into());
    }
    Ok(digest.to_owned())
}

/// SHA-256 of a binary/corpus; no Git executable or metadata is required.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    sha256_bytes(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// Canonical sorted content manifest, including paths (not only concatenated bytes).
pub fn snapshot(root: &Path) -> Result<String, String> {
    let files = source_files(root)?;
    let mut manifest = format!("{SCHEMA}\n");
    for chunk in files.chunks(128) {
        let out = Command::new("sha256sum")
            .current_dir(root)
            .args(["--binary", "--"])
            .args(chunk)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "source hash failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        let text = String::from_utf8(out.stdout).map_err(|e| e.to_string())?;
        let lines: Vec<_> = text.lines().collect();
        if lines.len() != chunk.len() {
            return Err("source hash count mismatch".into());
        }
        for (line, path) in lines.iter().zip(chunk) {
            if line.len() < 66
                || !line[..64].bytes().all(|b| b.is_ascii_hexdigit())
                || &line[64..66] != " *"
                || &line[66..] != path
            {
                return Err("source hash path mismatch".into());
            }
            manifest.push_str(line);
            manifest.push('\n');
        }
    }
    Ok(manifest)
}

/// Refuse stale binaries and inputs added, removed or changed since build time.
pub fn verify(root: &Path, built_manifest: &str) -> Result<(), String> {
    if !built_manifest.starts_with(&format!("{SCHEMA}\n")) {
        return Err("unrecognized build source manifest".into());
    }
    if snapshot(root)? != built_manifest {
        return Err(
            "SOURCE_MISMATCH: source differs from executable build; rebuild before running".into(),
        );
    }
    Ok(())
}

/// State captured before a measured campaign. Verification is outside all solver timers.
pub struct RunGuard {
    root: PathBuf,
    built_manifest: String,
    corpus: PathBuf,
    executable: PathBuf,
    /// Hash of the exact corpus bytes consumed by the run.
    pub corpus_sha256: String,
    /// Hash of the executable being run.
    pub binary_sha256: String,
    /// Explicit opt-in needed when Git reports a dirty repository.
    pub dirty_source_allowed: bool,
}

impl RunGuard {
    /// Verify the build source, then bind corpus and binary before execution.
    pub fn begin(
        root: &Path,
        built_manifest: &str,
        corpus: &Path,
        executable: &Path,
        corpus_bytes: &[u8],
        allow_dirty: bool,
    ) -> Result<Self, String> {
        verify(root, built_manifest)?;
        if !allow_dirty && git_context(root).is_some_and(|c| !c.1.is_empty()) {
            return Err("DIRTY_SOURCE: commit the source or explicitly pass --allow-dirty-source; content hashes remain mandatory".into());
        }
        let corpus_sha256 = sha256_bytes(corpus_bytes)?;
        if sha256_file(corpus)? != corpus_sha256 {
            return Err("CORPUS_CHANGED_BEFORE_RUN".into());
        }
        Ok(Self {
            root: root.to_owned(),
            built_manifest: built_manifest.to_owned(),
            corpus: corpus.to_owned(),
            executable: executable.to_owned(),
            corpus_sha256,
            binary_sha256: sha256_file(executable)?,
            dirty_source_allowed: allow_dirty,
        })
    }

    /// Refuse results if any bound input changed. Returns optional Git context.
    pub fn finish(&self) -> Result<Option<(String, String)>, String> {
        verify(&self.root, &self.built_manifest)?;
        if sha256_file(&self.corpus)? != self.corpus_sha256 {
            return Err("CORPUS_CHANGED_DURING_RUN".into());
        }
        if sha256_file(&self.executable)? != self.binary_sha256 {
            return Err("EXECUTABLE_CHANGED_DURING_RUN".into());
        }
        let context = git_context(&self.root);
        if !self.dirty_source_allowed && context.as_ref().is_some_and(|c| !c.1.is_empty()) {
            return Err("DIRTY_SOURCE_AFTER_RUN".into());
        }
        Ok(context)
    }
}

/// Optional Git context, never a substitute for `snapshot`.
pub fn git_context(root: &Path) -> Option<(String, String)> {
    let command = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
    };
    Some((
        command(&["rev-parse", "HEAD"])?,
        command(&["status", "--porcelain", "--untracked-files=all"])?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "vigilode-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(p.join("crates/a/src")).unwrap();
            std::fs::create_dir_all(p.join("fixtures")).unwrap();
            for (name, text) in [
                ("Cargo.toml", "[workspace]"),
                ("Cargo.lock", "lock"),
                ("crates/a/Cargo.toml", "[package]"),
                ("crates/a/src/lib.rs", "pub fn f() {}"),
                ("fixtures/coeff.json", "[1]"),
            ] {
                std::fs::write(p.join(name), text).unwrap();
            }
            Self(p)
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn known_sha256_vectors() {
        assert_eq!(
            sha256_bytes(b"").unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_bytes(b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn exact_snapshot_works_without_git_and_detects_dependency_fixture_lock_changes() {
        for file in ["crates/a/src/lib.rs", "fixtures/coeff.json", "Cargo.lock"] {
            let tree = Tree::new();
            let before = snapshot(&tree.0).unwrap();
            assert!(verify(&tree.0, &before).is_ok());
            assert!(git_context(&tree.0).is_none());
            std::fs::write(tree.0.join(file), "changed").unwrap();
            assert!(
                verify(&tree.0, &before)
                    .unwrap_err()
                    .starts_with("SOURCE_MISMATCH")
            );
        }
    }
    #[test]
    fn additions_removals_native_bridge_and_build_products() {
        let tree = Tree::new();
        let before = snapshot(&tree.0).unwrap();
        std::fs::create_dir_all(tree.0.join("crates/a/target")).unwrap();
        std::fs::write(tree.0.join("crates/a/target/binary"), "ignored").unwrap();
        assert_eq!(snapshot(&tree.0).unwrap(), before);
        std::fs::write(tree.0.join("crates/a/bridge.c"), "int x;").unwrap();
        assert!(verify(&tree.0, &before).is_err());
        let before = snapshot(&tree.0).unwrap();
        std::fs::remove_file(tree.0.join("crates/a/src/lib.rs")).unwrap();
        assert!(verify(&tree.0, &before).is_err());
    }
    #[test]
    fn literal_include_closure_is_bound() {
        let tree = Tree::new();
        std::fs::create_dir_all(tree.0.join("research")).unwrap();
        std::fs::write(tree.0.join("research/proof.json"), "proof").unwrap();
        let include = ["include_bytes", "!(\"../../../research/proof.json\");"].concat();
        std::fs::write(tree.0.join("crates/a/src/lib.rs"), include).unwrap();
        let before = snapshot(&tree.0).unwrap();
        assert!(before.contains("research/proof.json"));
        std::fs::write(tree.0.join("research/proof.json"), "other").unwrap();
        assert!(verify(&tree.0, &before).is_err());
    }
    #[test]
    fn unsupported_dynamic_includes_are_refused() {
        let tree = Tree::new();
        std::fs::write(
            tree.0.join("crates/a/src/lib.rs"),
            ["include_str", "!(env!(\"OTHER\"));"].concat(),
        )
        .unwrap();
        assert!(snapshot(&tree.0).is_err());
    }
    #[test]
    #[cfg(unix)]
    fn source_symlinks_are_refused() {
        let tree = Tree::new();
        std::os::unix::fs::symlink("lib.rs", tree.0.join("crates/a/src/link.rs")).unwrap();
        assert!(snapshot(&tree.0).is_err());
    }
    #[test]
    fn run_boundary_rejects_corpus_and_executable_replacement() {
        for mutate in ["corpus.json", "binary"] {
            let tree = Tree::new();
            std::fs::write(tree.0.join("corpus.json"), "corpus").unwrap();
            std::fs::write(tree.0.join("binary"), "binary").unwrap();
            let manifest = snapshot(&tree.0).unwrap();
            let guard = RunGuard::begin(
                &tree.0,
                &manifest,
                &tree.0.join("corpus.json"),
                &tree.0.join("binary"),
                b"corpus",
                false,
            )
            .unwrap();
            assert!(guard.finish().unwrap().is_none());
            std::fs::write(tree.0.join(mutate), "changed").unwrap();
            assert!(guard.finish().is_err());
        }
    }
    #[test]
    fn path_dependency_outside_crates_is_bound_and_external_paths_are_refused() {
        let tree = Tree::new();
        std::fs::create_dir_all(tree.0.join("extra/src")).unwrap();
        std::fs::write(tree.0.join("extra/Cargo.toml"), "[package]").unwrap();
        std::fs::write(tree.0.join("extra/src/lib.rs"), "pub fn dep() {}").unwrap();
        std::fs::write(
            tree.0.join("crates/a/Cargo.toml"),
            "[dependencies]\ndep = { path = \"../../extra\" }",
        )
        .unwrap();
        let before = snapshot(&tree.0).unwrap();
        assert!(before.contains("extra/src/lib.rs"));
        std::fs::write(tree.0.join("extra/src/lib.rs"), "changed").unwrap();
        assert!(verify(&tree.0, &before).is_err());
        std::fs::write(
            tree.0.join("crates/a/Cargo.toml"),
            "dep = { path = \"../../../outside\" }",
        )
        .unwrap();
        assert!(snapshot(&tree.0).is_err());
    }
}
