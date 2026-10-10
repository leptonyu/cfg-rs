# AGENTS.md

Guidance for AI coding agents working in this repository.

## Language: English only

Everything that lands in this repository must be written in English:

- source code, identifiers, comments, doc comments, panic and error messages
- `README.md`, design docs, changelogs, commit messages
- pull request titles and descriptions, issue bodies, review comments

Never commit text in Chinese or any other non-English language, even when the
conversation with the maintainer happens in that language. Translate before the
change lands; existing non-English text is a bug and should be fixed when it is
touched.

## No subagents

Do not delegate repository work to subagents, parallel agents, or any other
spawned agent. Every read, edit, build, test and verification step must happen
directly in this working copy, in the main session.

The repository is small enough to work on directly. A delegated agent shares
neither the review context nor the file ownership of this checkout, and a
subagent has already overwritten a source file here by using it as scratch
space.

## Project layout

- `src/` — `cfg-rs`, the configuration library (edition 2024, `rust-version = "1.85.0"`).
- `cfg-derive/` — `cfg-derive`, the `FromConfig` derive macro (edition 2018, proc-macro).
- `examples/`, `benches/core.rs` (divan), and the `app.{toml,yaml,json,ini}` fixtures.

## Build, test, lint

```bash
cargo build --features full
cargo test --workspace --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
bash local_test.sh
```

`local_test.sh` mirrors `.github/workflows/rust.yaml`: the feature matrix
(`no-default-features` plus each parser feature), the test suites and the
examples. CI runs Rust 1.85 / stable / beta / nightly on Linux, macOS and
Windows, so changes must keep `rust-version` honest.

## Dependency policy

- No open-ended requirements: a bare `>= x.y.z` is not allowed. Every dependency
  must cap its major version via the caret requirement of the version that is
  actually verified, e.g. `toml = "1.1.8"`, `serde_json = "1.0.151"`, `syn = '3.0.7'`.
- Prefer moving to the newest major over preserving compatibility with old
  majors. Tightening a requirement on purpose drops the old majors, and the
  accompanying crate `version` bump belongs in the same commit.
- Optional parsers stay behind their feature flag; keep `[features] full` in
  sync with the optional dependencies.

## Versioning and release

- Repo convention: every user-visible or dependency change bumps the crate
  `version` in the same commit.
- Publish `cfg-derive` before `cfg-rs`. `cargo package` for `cfg-rs` verifies
  against the *published* `cfg-derive`, so the new `cfg-derive` must already be
  on crates.io.

## Style

- Public items need doc comments and at least one usage example; see
  `CONTRIBUTING.md`.
- Behavior changes must be reflected in `README.md`.
- Keep the tree clean for `cargo clippy --all-targets --all-features -- -D warnings`
  and `cargo fmt --all -- --check`.
