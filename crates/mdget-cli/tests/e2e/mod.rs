use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_prints_help() {
    Command::cargo_bin("mdget")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetch a web page"));
}

#[test]
fn cli_prints_version() {
    Command::cargo_bin("mdget")
        .unwrap()
        .arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("mdget"));
}

#[test]
fn cli_missing_url_exits_with_error() {
    Command::cargo_bin("mdget")
        .unwrap()
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required argument"));
}

#[test]
fn cli_init_requires_claude_flag() {
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--claude"));
}

#[test]
fn cli_init_deinit_project_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    // init installs skill and updates CLAUDE.md
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["init", "--claude"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Installed skill"))
        .stderr(predicate::str::contains("Updated CLAUDE.md"));

    // SKILL.md was created
    assert!(dir.path().join(".claude/skills/mdget/SKILL.md").exists());

    // CLAUDE.md contains managed section
    let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("<!-- mdget:start -->"));
    assert!(claude_md.contains("<!-- mdget:end -->"));

    // deinit removes skill and strips managed section
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["deinit"])
        .current_dir(dir.path())
        .assert()
        .success();

    // SKILL.md is gone
    assert!(!dir.path().join(".claude/skills/mdget/SKILL.md").exists());

    // CLAUDE.md no longer contains managed section (or was removed)
    if dir.path().join("CLAUDE.md").exists() {
        let after = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(!after.contains("<!-- mdget:start -->"));
    }
}

#[test]
fn cli_init_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    for _ in 0..2 {
        Command::cargo_bin("mdget")
            .unwrap()
            .args(["init", "--claude"])
            .current_dir(dir.path())
            .assert()
            .success();
    }

    // Managed section should appear exactly once
    let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert_eq!(
        claude_md.matches("<!-- mdget:start -->").count(),
        1,
        "managed section should appear exactly once after double init"
    );
}

#[test]
fn cli_deinit_idempotent() {
    let dir = tempfile::tempdir().unwrap();

    // deinit on a clean directory should not error
    Command::cargo_bin("mdget")
        .unwrap()
        .args(["deinit"])
        .current_dir(dir.path())
        .assert()
        .success();
}
