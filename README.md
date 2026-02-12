# cfg-rs: A Configuration Library for Rust Applications

[![Crates.io](https://img.shields.io/crates/v/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Crates.io](https://img.shields.io/crates/d/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Documentation](https://docs.rs/cfg-rs/badge.svg)](https://docs.rs/cfg-rs)
[![dependency status](https://deps.rs/repo/github/leptonyu/cfg-rs/status.svg)](https://deps.rs/crate/cfg-rs)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/cfg-rs/blob/master/LICENSE)
[![Actions Status](https://github.com/leptonyu/cfg-rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/cfg-rs/actions)
[![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.85+-green.svg)](#minimum-supported-rust-version)

cfg-rs is a lightweight, flexible configuration loader for Rust applications. It composes multiple sources (files, env, inline maps, random, etc.), supports live refresh, placeholder expansion, and derive-based typed configs — all without a serde dependency.

See the [examples](https://github.com/leptonyu/cfg-rs/tree/main/examples) directory for end-to-end demos.

## Documentation map

If you are new to this crate, read in this order:

1. **Quick start** in this README
2. **Examples** for end-to-end usage patterns
3. **docs.rs API reference** for type-level details
4. **CONTRIBUTING.md** for contribution and documentation standards

- Examples: <https://github.com/leptonyu/cfg-rs/tree/main/examples>
- API docs: <https://docs.rs/cfg-rs>
- Contributing guide: [CONTRIBUTING.md](CONTRIBUTING.md)


## Features

- Single call to load typed config: see [Configuration::get](struct.Configuration.html#method.get)
- Derive your config types: see [FromConfig](derive.FromConfig.html)
- Default values via field attributes: see [field attributes](derive.FromConfig.html#field-annotation-attribute)
- Placeholder expansion like `${cfg.key}`: see [ConfigValue](enum.ConfigValue.html#placeholder-expression)
- Random values under the `rand` feature (e.g. `configuration.get::<u8>("random.u8")`)
- Refreshable values via [RefValue](struct.RefValue.html) and refreshable [Configuration](struct.Configuration.html)
- Field-level validation via `#[validate(...)]` rules (range, length, not_empty, custom, regex)
- Pluggable sources with clear priority: see [register_source](struct.Configuration.html#method.register_source)[^priority]
- No serde dependency

[^priority]: Source precedence follows registration order — earlier registrations have higher priority.

## Supported formats and feature flags

Built-in file parsers (enable via Cargo features):

| Feature | File extensions | Notes |
| --- | --- | --- |
| `toml` | `.toml`, `.tml` | Typed values and nested tables |
| `yaml` | `.yaml`, `.yml` | Friendly for hand-written configs |
| `json` | `.json` | Good for generated or machine-edited config |
| `ini` | `.ini` | Flat/simple legacy formats |

Other useful features:

- `validate` (built-in): supports `#[validate(range)]`, `#[validate(length)]`, `#[validate(not_empty)]`, and `#[validate(custom = ...)]`
- `rand`: random value provider (e.g. `random.u8`, `random.string`)
- `log`: minimal logging integration for value parsing
- `coarsetime`: coarse time helpers for time-related values
- `regex`: enables `#[validate(regex = ...)]` validator

Tip: in application crates, define your own feature aliases (e.g. `full-config = ["cfg-rs/full"]`) so downstream users can enable capabilities consistently.

## Installation

Add to your Cargo.toml with the features you need:

```toml
[dependencies]
cfg-rs = { version = "^1.0", features = ["toml"] }
```

For a batteries-included setup, use the convenience feature set:

```toml
cfg-rs = { version = "^1.0", features = ["full"] }
```

## Example guide

This README organizes examples into two tracks:

1. Derive-first typed config (`#[config(...)]` + `#[validate(...)]`)
2. Source composition (from simple to complex)

### 1) Derive-first (recommended)

Use derive for app-facing config structs. This single example covers:

- `#[config(name = "...")]`
- `#[config(default = ...)]`
- Placeholder defaults via `${...}` in values
- `#[validate(...)]`

```rust
#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "app")]
struct AppCfg {
    #[config(name = "service_name", default = "demo")]
    #[validate(length(min = 1, max = 32))]
    name: String,

    #[config(default = "${server.host:127.0.0.1}")]
    host: String,

    #[config(default = "${server.port:8080}")]
    #[validate(range(min = 1, max = 65535))]
    port: u16,

    #[config(default = 4)]
    #[validate(custom = check_workers)]
    workers: usize,
}

fn check_workers(v: &usize) -> Result<(), String> {
    if *v == 0 {
        return Err("workers must be > 0".to_string());
    }
    Ok(())
}

let cfg: AppCfg = cfg_rs::from_static_map!(AppCfg, {
    "service_name" => "api",
    "server.host" => "0.0.0.0",
});
assert_eq!(cfg.name, "api");
assert_eq!(cfg.host, "0.0.0.0");
assert_eq!(cfg.port, 8080);
```

Notes:

- Placeholder expressions are value syntax, not a separate derive attribute.
- Full derive attribute reference: [derive.FromConfig](derive.FromConfig.html)
- Validation rules: `range`, `length`, `not_empty`, `custom`, `regex` (feature = `regex`)

### 2) Source composition (simple -> complex)

#### a) Simplest: predefined stack

```rust
use cfg_rs::*;

let configuration = Configuration::with_predefined().unwrap();
// let port: u16 = configuration.get("app.port").unwrap();
```

#### b) Common: explicit env + file

```rust,no_run
use cfg_rs::*;

let configuration = Configuration::new()
    .register_prefix_env("APP").unwrap()
    .register_file("./app.toml", true).unwrap();
```

#### c) Advanced: layered sources with deterministic priority

```rust,no_run
use cfg_rs::*;
init_cargo_env!();

let mut configuration = Configuration::new()
    // Layer 0 (highest): Cargo env source.
    .register_source(init_cargo_env()).unwrap()
    // Layer 1: Inline key-values.
    .register_kv("inline")
        .set("app.name", "demo")
        .finish()
        .unwrap();

// Layer 2: Random values (feature = "rand").
#[cfg(feature = "rand")]
{
    configuration = configuration.register_random().unwrap();
}

// Layer 3: Environment variables with prefix `APP_`.
configuration = configuration.register_prefix_env("APP").unwrap();

// Layer 4 (lowest): File source.
configuration = configuration.register_file("./app.yaml", true).unwrap();

// Optional: merge inline file content.
#[cfg(feature = "toml")]
{
    let toml = inline_source!("app.toml").unwrap();
    configuration = configuration.register_source(toml).unwrap();
}
```

See [register_kv](struct.Configuration.html#method.register_kv), [register_file](struct.Configuration.html#method.register_file), [register_random](struct.Configuration.html#method.register_random), and [register_prefix_env](struct.Configuration.html#method.register_prefix_env).

## Placeholders, randoms, and refresh

- Placeholder expansion: use `${some.key}` inside string values; see [ConfigValue](enum.ConfigValue.html#placeholder-expression)
- Random values: under `rand`, keys like `random.u8`, `random.string` provide per-read randoms
- Refreshing: `Configuration::refresh()` re-reads sources that allow refresh; `RefValue<T>` updates on refresh

## Runnable examples

- `simple`: minimal setup (full feature set)
- `profile`: working with profiles (requires `toml`)
- `watch`: basic file watching and refresh (requires `yaml`)
- `refresh`: manual refresh and `RefValue`
- `logger`: logging integration (requires `full`)
- `thread_pool`, `salak`, `test_suit`: larger samples and integrations

https://github.com/leptonyu/cfg-rs/tree/main/examples

## License

MIT © contributors. See [LICENSE](https://github.com/leptonyu/cfg-rs/blob/main/LICENSE).

## Minimum supported Rust version
This crate supports Rust **1.85** and newer. Older Rust versions are not guaranteed to compile or be tested.

## Tips and notes

- Source priority is deterministic: earlier registrations override later ones[^priority]
- This crate intentionally does not depend on serde
- Docs.rs builds enable all features for a comprehensive reference


## Open-source documentation improvement checklist

If you maintain this project, these upgrades usually provide the biggest payoff:

- Add a **versioned migration section** in release notes for each breaking/non-trivial change.
- Keep a **"common recipes"** section (env + file layering, validation patterns, refresh loop).
- Add **"troubleshooting"** for frequent errors (missing feature flags, key prefix mismatch, parse failures).
- Keep examples aligned with latest API and feature naming.
- Treat README + examples + doc comments as one documentation surface; update all three together in PRs.

These habits improve first-run success for users and reduce repeated issue triage for maintainers.
