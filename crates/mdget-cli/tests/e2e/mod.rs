use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn cli_runs_and_prints_greeting() {
    Command::cargo_bin("mdget")
        .unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello from mdget!"));
}
