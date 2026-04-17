---
title: "Iteration 1 — Project Initialization"
type: iteration
date: 2026-04-17
tags:
  - iteration
  - setup
status: in-progress
branch: iter-1/project-init
---

# Iteration 1 — Project Initialization

## Goal

Bootstrap the mdget Rust CLI project as a workspace with two crates (`mdget-core` and `mdget-cli`), mirroring the structure of hyalo. Start with a hello-world binary and set up CI/CD pipelines.

## Context

mdget is a new CLI tool. The project structure, CI/CD, and code quality gates are modeled after the hyalo project to maintain consistency across our Rust tooling.

## Tasks

- [x] Create workspace Cargo.toml with `crates/mdget-core` and `crates/mdget-cli`
- [x] Implement hello-world in mdget-core (greeting function) and mdget-cli (prints it)
- [x] Add e2e test verifying CLI output
- [x] Set up CI workflow (fmt, clippy, test on Ubuntu/macOS/Windows)
- [x] Set up release workflow (version check, security audit, multi-platform build, crates.io publish)
- [x] Add deny.toml for dependency auditing
- [x] Add .gitignore, LICENSE, CLAUDE.md
- [x] Create kb/ folder with this iteration file
- [ ] Initialize git repo, verify quality gates pass, create initial commit

## Quality Gates

- [ ] `cargo fmt`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace -q`

## Acceptance Criteria

- [ ] `cargo run` prints "Hello from mdget!"
- [ ] All tests pass on workspace level
- [ ] CI workflow is ready for GitHub Actions
- [ ] Release workflow covers multi-platform builds and crates.io publishing
