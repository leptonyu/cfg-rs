# cfg-rs: A Configuration Library for Rust Applications

[![Crates.io](https://img.shields.io/crates/v/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Crates.io](https://img.shields.io/crates/d/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Documentation](https://docs.rs/cfg-rs/badge.svg)](https://docs.rs/cfg-rs)
[![dependency status](https://deps.rs/repo/github/leptonyu/cfg-rs/status.svg)](https://deps.rs/crate/cfg-rs)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/cfg-rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/cfg-rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/cfg-rs/actions)
[![Minimum supported Rust version](https://img.shields.io/badge/rustc-1.54+-green.svg)](#minimum-supported-rust-version)

## Major Features

* One method to get all config objects, see [get](struct.Configuration.html#method.get).
* Automatic derive config object, see [FromConfig](derive.FromConfig.html).
* Support default value for config object by auto deriving, see [derived attr](derive.FromConfig.html#field-annotation-attribute).
* Config value placeholder parsing, e.g. `${config.key}`, see [placeholder](enum.ConfigValue.html#placeholder-expression).
* Random config value, e.g. `configuration.get::<u8>("random.u8")` will get random `u8` value.
* Support refreshable value type [RefValue](struct.RefValue.html), it can be updated when refreshing.
* Support refresh [Configuration](struct.Configuration.html).
* Easy to use, easy to add new config source, easy to organize configuration, see [register_source](struct.Configuration.html#method.register_source).[^priority]

See the [examples](https://github.com/leptonyu/cfg-rs/tree/main/examples) for general usage information.

[^priority]: Config order is determined by the order of registering sources, register earlier have higher priority.

#### Supported File Format

* Toml: toml, tml
* Yaml: yaml, yml
* Json: json
* Ini: ini

## How to Initialize Configuration

* Use Predefined Source Configuration in One Line

```rust,no_run
use cfg_rs::*;
let configuration = Configuration::with_predefined().unwrap();
// use configuration.
```
See [init](struct.PredefinedConfigurationBuilder.html#method.init) for details.

* Customize Predefined Source Configuration Builder

```rust,no_run
use cfg_rs::*;
init_cargo_env!();
let configuration = Configuration::with_predefined_builder()
    .set_cargo_env(init_cargo_env())
    .init()
    .unwrap();
// use configuration.
```
See [init](struct.PredefinedConfigurationBuilder.html#method.init) for details.

* Organize Your Own Sources

```rust,no_run
use cfg_rs::*;
init_cargo_env!();
let mut configuration = Configuration::new()
    // Layer 0: Register cargo env config source.
    .register_source(init_cargo_env()).unwrap()
    // Layer 1: Register customized config.
    .register_kv("customized_config")
        .set("hello", "world")
        .finish()
        .unwrap();
    // Layer 2: Register random value config.
#[cfg(feature = "rand")]
{
configuration = configuration.register_random().unwrap();
}
    // Layer 3: Register all env variables `CFG_*`.
configuration = configuration.register_prefix_env("CFG").unwrap()
    // Layer 4: Register yaml file(Need feature yaml).
    .register_file("/conf/app.yaml", true).unwrap();

#[cfg(feature = "toml")]
{
    let toml = inline_source!("../app.toml").unwrap();
    configuration = configuration.register_source(toml).unwrap();
}

// use configuration.
```
See [register_kv](struct.Configuration.html#method.register_kv), [register_file](struct.Configuration.html#method.register_file), [register_random](struct.Configuration.html#method.register_random), [register_prefix_env](struct.Configuration.html#method.register_prefix_env) for details.



