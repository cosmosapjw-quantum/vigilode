use std::path::Path;
use std::process::Command;

/// Run `git -C root args...`; `None` when git is absent, fails, or prints non-UTF-8.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|text| text.trim().to_string())
}

fn is_lower_hex_revision(revision: &str) -> bool {
    revision.len() == 40
        && revision
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether git at `root` describes this source tree: the repository's work
/// tree must be rooted at the workspace itself and track the workspace
/// manifest, lockfile and this build script. Git discovery walks up to any
/// enclosing repository, so an untracked export inside another repository
/// otherwise inherited that repository's commit and a clean flag (external
/// audit VIG-A03).
fn repository_is_rooted_at(root: &Path) -> bool {
    let Some(top_level) = git(root, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    let same_root = match (Path::new(&top_level).canonicalize(), root.canonicalize()) {
        (Ok(top_level), Ok(root)) => top_level == root,
        _ => false,
    };
    same_root
        && git(
            root,
            &[
                "ls-files",
                "--error-unmatch",
                "--",
                "Cargo.toml",
                "Cargo.lock",
                "crates/rodas5p-fair-ab/build.rs",
            ],
        )
        .is_some()
}

/// Embed build provenance without requiring a git worktree (audit F-067).
///
/// A source tree without git (a `git archive` export, a vendored copy), or
/// one whose enclosing repository is not rooted at this workspace, builds
/// with revision `unknown` and dirty flag `unknown`. The canonical v2 runner
/// refuses both at run time, so no receipt can claim a revision it was not
/// built from. The dirty flag covers tracked files only: an untracked scratch
/// file no longer marks the build dirty.
fn main() {
    println!("cargo:rerun-if-env-changed=VIGILODE_CODE_REVISION");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    if !repository_is_rooted_at(&root) {
        println!("cargo:rustc-env=VIGILODE_DETECTED_GIT_REVISION=unknown");
        println!("cargo:rustc-env=VIGILODE_SOURCE_DIRTY_AT_BUILD=unknown");
        return;
    }

    for git_path in ["HEAD", "index"] {
        if let Some(path) = git(&root, &["rev-parse", "--git-path", git_path]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(symbolic_ref) = git(&root, &["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git(&root, &["rev-parse", "--git-path", &symbolic_ref])
    {
        println!("cargo:rerun-if-changed={path}");
    }
    let tracked = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z"])
        .output()
        .ok()
        .filter(|output| output.status.success());
    if let Some(tracked) = tracked {
        for relative in tracked
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            if let Ok(relative) = std::str::from_utf8(relative) {
                println!("cargo:rerun-if-changed={}", root.join(relative).display());
            }
        }
    }

    let revision = git(&root, &["rev-parse", "HEAD"])
        .filter(|revision| is_lower_hex_revision(revision))
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=VIGILODE_DETECTED_GIT_REVISION={revision}");

    let dirty = match git(&root, &["status", "--porcelain=v1", "--untracked-files=no"]) {
        Some(status) if status.is_empty() => "false",
        Some(_) => "true",
        None => "unknown",
    };
    println!("cargo:rustc-env=VIGILODE_SOURCE_DIRTY_AT_BUILD={dirty}");
}
