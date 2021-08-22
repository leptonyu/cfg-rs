# cfg-rs: A Configuration Library for Rust Applications

[![Crates.io](https://img.shields.io/crates/v/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Crates.io](https://img.shields.io/crates/d/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Documentation](https://docs.rs/cfg-rs/badge.svg)](https://docs.rs/cfg-rs)
[![dependency status](https://deps.rs/repo/github/leptonyu/cfg-rs/status.svg)](https://deps.rs/crate/cfg-rs)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/cfg-rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/cfg-rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/cfg-rs/actions)

## Features

* One line to create configuration by [`Configuration::init`].
* Customizing creating configuration by [`Configuration::builder`].
* Provide random value source, e.g. `${random.u64}`.
* Support multiple source formats: toml, yaml and json.
* Easy to add new source format.
* Automatic derive config object by [`FromConfig`].
* Support config value placeholder expression, e.g. `${config.key}`.

See the [examples](https://github.com/leptonyu/cfg-rs/tree/main/examples) for general usage information.


