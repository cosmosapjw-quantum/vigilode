use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=VIGILODE_CODE_REVISION");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git_head_path = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "--git-path", "HEAD"])
        .output()
        .expect("canonical v2 build requires git-path discovery");
    assert!(
        git_head_path.status.success(),
        "git rev-parse --git-path HEAD failed"
    );
    let git_head_path = String::from_utf8(git_head_path.stdout).expect("git HEAD path is UTF-8");
    println!("cargo:rerun-if-changed={}", git_head_path.trim());
    {
        let git_path = "index";
        let output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "--git-path", git_path])
            .output()
            .expect("canonical v2 build requires git metadata discovery");
        assert!(
            output.status.success(),
            "git metadata path discovery failed"
        );
        let path = String::from_utf8(output.stdout).expect("git metadata path is UTF-8");
        println!("cargo:rerun-if-changed={}", path.trim());
    }
    let symbolic_ref = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .expect("canonical v2 build requires symbolic-ref discovery");
    if symbolic_ref.status.success() {
        let symbolic_ref = String::from_utf8(symbolic_ref.stdout).expect("symbolic ref is UTF-8");
        let ref_path = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "--git-path", symbolic_ref.trim()])
            .output()
            .expect("canonical v2 build requires branch-ref path discovery");
        assert!(
            ref_path.status.success(),
            "branch-ref path discovery failed"
        );
        let ref_path = String::from_utf8(ref_path.stdout).expect("branch-ref path is UTF-8");
        println!("cargo:rerun-if-changed={}", ref_path.trim());
    }
    let tracked = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["ls-files", "-z"])
        .output()
        .expect("canonical v2 build requires tracked-file discovery");
    assert!(tracked.status.success(), "git ls-files failed");
    for relative in tracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
    {
        let relative = std::str::from_utf8(relative).expect("tracked path is UTF-8");
        println!("cargo:rerun-if-changed={}", root.join(relative).display());
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("canonical v2 build requires git revision discovery");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    let revision = String::from_utf8(output.stdout).expect("git revision is UTF-8");
    let revision = revision.trim();
    assert!(
        revision.len() == 40
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "detected git revision is not lowercase 40-hex"
    );
    println!("cargo:rustc-env=VIGILODE_DETECTED_GIT_REVISION={revision}");

    let status = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain=v1", "--untracked-files=normal"])
        .output()
        .expect("canonical v2 build requires git status discovery");
    assert!(status.status.success(), "git status --porcelain failed");
    println!(
        "cargo:rustc-env=VIGILODE_SOURCE_DIRTY_AT_BUILD={}",
        if status.stdout.is_empty() {
            "false"
        } else {
            "true"
        }
    );
}
