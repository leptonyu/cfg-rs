# Contributing to cfg-rs

Thanks for your interest in improving `cfg-rs`! This document explains how to contribute code and documentation effectively.

## Quick workflow

1. Fork and create a feature branch.
2. Make small, focused changes.
3. Run formatting/lint/tests locally.
4. Open a PR with motivation, design notes, and verification steps.

## Local development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test
```

If your change touches docs examples, ensure doctests still pass via `cargo test`.

## Pull request checklist

Before opening a PR, please make sure:

- [ ] The change is scoped and explained in the PR description.
- [ ] Public behavior changes are documented in `README.md` and/or doc comments.
- [ ] New APIs include at least one usage example.
- [ ] Error messages are actionable and user-friendly.
- [ ] `cargo fmt`, `cargo clippy`, and `cargo test` pass.

## Documentation guidelines

For an open-source crate, docs are part of the product. Prefer:

- **Task-first writing**: start with "how to use" before internals.
- **Copy-paste examples**: runnable snippets with realistic defaults.
- **Feature clarity**: state required crate feature flags next to examples.
- **Migration notes**: call out breaking changes and alternatives.
- **Discoverability**: cross-link README, examples, and API docs.

### Recommended structure for new docs

When adding a new capability, include:

1. What problem it solves.
2. Minimal example.
3. Common failure modes / gotchas.
4. Advanced options.
5. Links to source tests/examples.

## Commit message suggestions

Use short, descriptive messages, e.g.:

- `docs: add feature flag matrix`
- `feat: support custom validator path`
- `fix: preserve source priority on refresh`

## Reporting issues

Please include:

- Rust version (`rustc -V`)
- `cfg-rs` version and enabled features
- Minimal reproducible config/input
- Expected vs actual behavior

Thanks again for helping make `cfg-rs` better.
