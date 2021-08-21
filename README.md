# cfg-rs
cfg-rs provides a layered configuration formed by multi config source for rust applications.

Former crate [salak.rs](https://github.com/leptonyu/salak.rs).

[![Crates.io](https://img.shields.io/crates/v/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Crates.io](https://img.shields.io/crates/d/cfg-rs?style=flat-square)](https://crates.io/crates/cfg-rs)
[![Documentation](https://docs.rs/cfg-rs/badge.svg)](https://docs.rs/cfg-rs)
[![dependency status](https://deps.rs/repo/github/leptonyu/cfg-rs/status.svg)](https://deps.rs/crate/cfg-rs)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/leptonyu/cfg-rs/blob/master/LICENSE-MIT)
[![Actions Status](https://github.com/leptonyu/cfg-rs/workflows/Rust/badge.svg)](https://github.com/leptonyu/cfg-rs/actions)



This lib supports:
* Easy to use, one line to create, [`Configuration::init`].
* Mutiple sources, such as environment variables, toml, yaml and json.
* Easily extends config source by implementing [`crate::source::file::FileConfigSource`].
* Programmatic override config by [`ConfigurationBuilder::set`].
* Auto derive config struct by proc-macro.
* Placeholder parsing with syntax `${config.key}`.
* Using placeholder expresion to get random value by `${random.u64}`, support all integer types.
* With high performance when parsing.

See the [examples](https://github.com/leptonyu/cfg-rs/tree/main/examples) for general usage information.


