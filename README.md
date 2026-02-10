# cfg-rs: A Configuration Library for Rust Applications

[![Crates.io](https://img.shields.io/crates/v/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Crates.io](https://img.shields.io/crates/d/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Documentation](https://docs.rs/cfg-rs/badge.svg)](https://docs.rs/cfg-rs)
[![dependency status](https://deps.rs/repo/github/leptonyu/cfg-rs/status.svg)](https://deps.rs/crate/cfg-rs)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/cfg-rs/blob/master/LICENSE)
[![Actions Status](https://github.com/leptonyu/cfg-rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/cfg-rs/actions)
[![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.81+-green.svg)](#minimum-supported-rust-version)

cfg-rs is a lightweight, flexible configuration loader for Rust applications. It composes multiple sources (files, env, inline maps, random, etc.), supports live refresh, placeholder expansion, and derive-based typed configs — all without a serde dependency.

See the examples directory for end-to-end demos: https://github.com/leptonyu/cfg-rs/tree/main/examples

## Features

- Single call to load typed config: see [Configuration::get](struct.Configuration.html#method.get)
- Derive your config types: see [FromConfig](derive.FromConfig.html)
- Default values via field attributes: see [field attributes](derive.FromConfig.html#field-annotation-attribute)
- Placeholder expansion like `${cfg.key}`: see [ConfigValue](enum.ConfigValue.html#placeholder-expression)
- Random values under the `rand` feature (e.g. `configuration.get::<u8>("random.u8")`)
- Refreshable values via [RefValue](struct.RefValue.html) and refreshable [Configuration](struct.Configuration.html)
- Pluggable sources with clear priority: see [register_source](struct.Configuration.html#method.register_source)[^priority]
- No serde dependency

[^priority]: Source precedence follows registration order — earlier registrations have higher priority.

## Supported formats and feature flags

Built-in file parsers (enable via Cargo features):

- `toml`: extensions `.toml`, `.tml`
- `yaml`: extensions `.yaml`, `.yml`
- `json`: extension `.json`
- `ini`: extension `.ini`

Other useful features:

- `rand`: random value provider (e.g. `random.u8`, `random.string`)
- `log`: minimal logging integration for value parsing
- `coarsetime`: coarse time helpers for time-related values
- `regex`: regex validation support for `#[validate(regex = ...)]` and `#[validate(email)]`

## Installation

Add to your Cargo.toml with the features you need:

```toml
[dependencies]
cfg-rs = { version = "^0.6", features = ["toml"] }
```

For a batteries-included setup, use the convenience feature set:

```toml
cfg-rs = { version = "^0.6", features = ["full"] }
```

## Quick start

### 1) One-liner with predefined sources

```rust
use cfg_rs::*;

let configuration = Configuration::with_predefined().unwrap();
// use configuration.get::<T>("your.key") or derive types (see below)
```

See [PredefinedConfigurationBuilder::init](struct.PredefinedConfigurationBuilder.html#method.init) for details.

### 2) Customize predefined builder

```rust,no_run
use cfg_rs::*;
init_cargo_env!();

let configuration = Configuration::with_predefined_builder()
    .set_cargo_env(init_cargo_env())
    .init()
    .unwrap();
```

### 3) Compose your own sources (priority = registration order)

```rust,no_run
use cfg_rs::*;
init_cargo_env!();

let mut configuration = Configuration::new()
    // Layer 0: Cargo env source.
    .register_source(init_cargo_env()).unwrap()
    // Layer 1: Inline key-values.
    .register_kv("inline")
        .set("hello", "world")
        .finish()
        .unwrap();

// Layer 2: Random values (feature = "rand").
#[cfg(feature = "rand")]
{
    configuration = configuration.register_random().unwrap();
}

// Layer 3: All environment variables with prefix `CFG_`.
configuration = configuration.register_prefix_env("CFG").unwrap();

// Layer 4: File(s) — extension inferred by feature (e.g. yaml).
configuration = configuration.register_file("/conf/app.yaml", true).unwrap();

// Optional: register an inline file content (e.g. TOML) and merge.
#[cfg(feature = "toml")]
{
    let toml = inline_source!("../app.toml").unwrap();
    configuration = configuration.register_source(toml).unwrap();
}

// Finally use it.
// let port: u16 = configuration.get("server.port").unwrap();
```

See [register_kv](struct.Configuration.html#method.register_kv), [register_file](struct.Configuration.html#method.register_file), [register_random](struct.Configuration.html#method.register_random), and [register_prefix_env](struct.Configuration.html#method.register_prefix_env).

### 4) Handy helpers for tests and small apps

- From inline map (macro):

```rust
#[derive(Debug, cfg_rs::FromConfig)]
struct AppCfg { port: u16, host: String }

let cfg: AppCfg = cfg_rs::from_static_map!(AppCfg, {
    "port" => "8080",
    "host" => "localhost",
});
```

- From environment variables:

```rust

#[derive(Debug, cfg_rs::FromConfig)]
struct AppCfg { port: u16, host: String }

std::env::set_var("CFG_APP_PORT", "8080");
std::env::set_var("CFG_APP_HOST", "localhost");
let cfg: AppCfg = cfg_rs::from_env("CFG_APP").unwrap();
```

## Derive typed configs

Implement strong-typed configs via derive:

```rust,no_run

#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "cfg.app")] // optional, implements FromConfigWithPrefix
struct AppCfg {
    port: u16,              // required
    #[config(default = true)]
    enabled: bool,          // has default value
    #[config(name = "ip")] // remap field name
    host: String,
}
```

Attributes summary:

- `#[config(prefix = "cfg.app")]` on struct: implement `FromConfigWithPrefix`
- `#[config(name = "...")]` on field: rename field key
- `#[config(default = <expr>)]` on field: default value when missing

See the full reference in [derive.FromConfig](derive.FromConfig.html).

## Validation

The derive macro supports field-level validation via `#[validate(...)]`.
The rules are implemented in [src/validate.rs](src/validate.rs) and are
invoked after parsing field values.

Available validators:

- `range(min = <expr>, max = <expr>)` for comparable values
- `length(min = <usize>, max = <usize>)` for string/collection/path length
- `not_empty` for any type implementing `ValidateLength`
- `validate_not_empty(field, value)` helper for any type implementing `ValidateLength`
- `regex = "..."` (feature = `regex`) for regex matching on strings
- `email` (feature = `regex`) for basic email format checks
- `custom = "path::to::fn"` for user-defined validation

Example:

```rust,no_run
#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "app")]
struct AppCfg {
    #[validate(range(min = 1, max = 65535))]
    port: u16,
    #[validate(length(min = 1, max = 32))]
    name: String,
    #[validate(custom = "check_threads")]
    threads: usize,
    #[cfg(feature = "regex")]
    #[validate(regex = "^u[a-z]+$")]
    user: String,
    #[cfg(feature = "regex")]
    #[validate(email)]
    email: String,
}

fn check_threads(v: &usize) -> Result<(), cfg_rs::ConfigError> {
    if *v == 0 {
        return Err(cfg_rs::ConfigError::ConfigParseError(
            "app.threads".to_string(),
            "threads must be > 0".to_string(),
        ));
    }
    Ok(())
}
```

## Placeholders, randoms, and refresh

- Placeholder expansion: use `${some.key}` inside string values; see [ConfigValue](enum.ConfigValue.html#placeholder-expression)
- Random values: under `rand`, keys like `random.u8`, `random.string` provide per-read randoms
- Refreshing: `Configuration::refresh()` re-reads sources that allow refresh; `RefValue<T>` updates on refresh

## Examples

Browse runnable examples covering common patterns:

- `simple`: minimal setup (full feature set)
- `profile`: working with profiles (requires `toml`)
- `watch`: basic file watching and refresh (requires `yaml`)
- `refresh`: manual refresh and `RefValue`
- `logger`: logging integration (requires `full`)
- `thread_pool`, `salak`, `test_suit`: larger samples and integrations

https://github.com/leptonyu/cfg-rs/tree/main/examples

## Minimum supported Rust version

rustc 1.81+

## License

MIT © contributors. See [LICENSE](https://github.com/leptonyu/cfg-rs/blob/main/LICENSE).

## Tips and notes

- Source priority is deterministic: earlier registrations override later ones[^priority]
- This crate intentionally does not depend on serde
- Docs.rs builds enable all features for a comprehensive reference



