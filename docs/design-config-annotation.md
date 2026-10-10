# Design document: `#[config(desc = ..., sample = ...)]` annotations, full configuration export, and sample completeness checking

Status: **Proposal (Design only, no source code changed)**
Related code: `cfg-derive/src/lib.rs`, `src/lib.rs`, `src/configuration.rs`

Revision history:

- v1: `desc` / `sample` annotations + metadata model + TOML template export (§1–§4).
- v2: Added §5 "`ConfigSampleCheck` completeness checking extension trait"; accordingly changed the metadata for `sample` / `default` from a **rendered TOML fragment** to a **structured `ConfigLiteral`** (§3.6 explains why), and added §5.8 `undocumented` and §5.9 value probe.

---

## 1. Background and current state

### 1.1 Existing annotation surface

`cfg-derive` currently parses the following annotations (`#[proc_macro_derive(FromConfig, attributes(config, validate))]`):

| Location | Annotation | Current state |
| --- | --- | --- |
| struct | `prefix = "..."` | Implemented, generates `FromConfigWithPrefix` |
| struct | `crate = "..."` | Implemented, rewrites the crate path in the generated code |
| field | `name = "..."` | Implemented, renames the configuration key |
| field | `default = ...` | Implemented, the literal serves as the runtime fallback value |
| field | `desc = "..."` | **Parsed but never used** (`FieldInfo.desc` is a dead field) |
| field | `sample = ...` | **Not supported**, reports `Only support default/name/desc` |
| field | `#[validate(...)]` | Implemented (range/length/not_empty/regex/custom) |

Key conclusions:

- `desc` has in fact already been written into the macro's parsing branch (`cfg-derive/src/lib.rs:192`), but `FieldInfo.desc` is never read by any code, and there is no corresponding trait/metadata exit either. In other words, the annotation surface exists, but the capability surface is zero.
- `sample` needs to be added, and there is **no metadata infrastructure**, so it is not enough to merely add a parsing branch.
- The generated code only implements `FromConfig::from_config` (and optionally `FromConfigWithPrefix`), so at runtime it is impossible to enumerate "which fields this type has and what the fields mean".

### 1.2 The exact meaning of "default configuration" in this crate

This point directly determines the design boundary of the export capability:

- This crate does **not** have serde's `Default` semantics. A field has a fallback value only when `#[config(default = ...)]` is written; without it, the field is **required**, and at runtime `ConfigError::ConfigNotFound` is returned.
- Therefore, strictly speaking, a "complete default configuration" exists only when **every field has a `default`**.
- For ordinary structs, "complete" should be understood as a **complete template**: every field appears in the artifact, those with default values are written as actual key-value pairs, and those without default values are given a sample in the form of a comment (sample).

Based on this, this design provides two export styles (see `ExportStyle` in §4.1).

### 1.3 Goals

1. Make `desc` / `sample` **machine-readable** metadata, rather than merely comments.
2. Provide an exporter that does not depend on serde and does not depend on any file format feature, producing a commented TOML template.
3. **Zero impact** on existing behavior: `desc` / `sample` are used only for documentation and export, and never participate in parsing, default values, or validation.

### 1.4 Non-goals

- Do not introduce dependencies such as serde / schemars.
- Do not build a JSON Schema-style strongly typed validation language.
- Do not turn `sample` into a runtime fallback value (confirmed semantics: documentation only). The value probe in §5.9 only feeds sample to the parser as input during tests, and does not change this.
- Do not attempt to generate real values for `HashMap` with dynamic keys / unknown generics.
- `ConfigSampleCheck` does not pursue 100% coverage (for example dynamic keys of `Map`, deep layers of recursive types) — the report explicitly marks these "not found" regions instead of pretending to be complete.

---

## 2. Annotation design

### 2.1 Field-level annotations

```rust
#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "app", desc = "Application configuration")]
struct AppCfg {
    /// Service name (a doc comment can also serve as the fallback for desc)
    #[config(desc = "Service name", sample = "demo", default = "demo")]
    name: String,

    #[config(desc = "Listen address", sample = "127.0.0.1", default = "0.0.0.0")]
    host: String,

    #[config(desc = "TCP port", sample = 8080, default = 8080)]
    port: u16,

    #[config(desc = "Number of worker threads", sample = 4)]      // required: no default
    workers: usize,

    #[config(desc = "Allowed origins", sample = ["a.example.com", "b.example.com"])]
    origins: Vec<String>,

    #[config(desc = "TLS configuration")]
    tls: TlsCfg,                                    // nested struct, described recursively
}
```

Syntax rules:

| Key | Value type | Semantics | Required |
| --- | --- | --- | --- |
| `name` | `LitStr` | Configuration key renaming (unchanged) | No |
| `default` | `Lit` (keep as is) | Runtime fallback value; also used as the value when exporting | No |
| `desc` | `LitStr` | Meaning of the field, exported as a comment | No |
| `sample` | **`Expr` (literal / array literal / negative number)** | Documentation and export only, never participates at runtime | No |

Why `sample` uses `Expr` instead of using `Lit` like `default`:

- `sample` is never evaluated and only serves **rendering**, so it can safely accept a wider subset: `Lit`, `-1` (`Expr::Unary(Neg, Lit)`), `[a, b, c]` (`Expr::Array`, elements must still be literals).
- `default` keeps using `Lit`: it is converted to `ConfigValue::Str` at runtime and then goes through parsing, and its semantics must not be loosened by this change (for example, negative number literals are currently unsupported, and this design should not quietly change that).
- Writing `sample` as an array is meant to give a `Vec<T>` field a readable example (`["a", "b"]` → TOML `["a", "b"]`).
- Invalid forms (paths, function calls, template expressions, etc.) report an error at **compile time**, with a message of the form:
  `sample must be a literal, a negative number, or an array of literals`.

`sample` and `default` are stored in the metadata as a **structured literal** `ConfigLiteral` (see §3.5 / §3.6), rather than as rendered text: this way the same piece of data can both be rendered into a comment and be fed back to the parser for a validity probe (see §5.9).

### 2.2 Struct-level annotations

Add `desc`, at the same level as `prefix` / `crate`:

```rust
#[config(prefix = "app", desc = "Application configuration")]
```

- Generates `impl FromConfig { fn config_desc() -> Option<&'static str> }`.
- Used as a file header comment when exporting.
- The parser error message is updated from `Only support prefix` to `Only support prefix/desc/crate` (compile-time wording change only).

Struct-level `sample` is **not** introduced: the struct itself is not a value.

### 2.3 Relationship with doc comments (`///`)

Recommendation: the `desc` attribute takes precedence; when it is absent, take the **first paragraph** (before the first blank line) of the field doc comment as `desc`.

- Extremely low cost: `#[doc = "..."]` is already present in `field.attrs`, and the macro is already iterating over them.
- Matches Rust intuition and avoids making users write the meaning twice just for export.
- Purely additive, and does not change the behavior of any existing code.

### 2.4 `skip` (optional extension, does not block the main design)

If certain fields should not participate in export (for example internal caches, `RefValue` handles), `#[config(skip)]` can be added:

- It affects only metadata generation, not `from_config` (unlike `default`, it does not change parsing).
- Recommended for Phase 2; the core annotations should be completed end to end first.

---

## 3. Metadata model

### 3.1 Why the methods must hang off `FromConfig`

This is the most critical and also the most error-prone point in the entire design.

We need to recursively obtain child fields under the premise of **not knowing whether a field type is a nested struct**. The macro can only make syntactic judgments and cannot know what lies behind `type Alias = Vec<Foo>;`, so there must be an entry point that "every type can call and that leaf types return empty for".

Candidate approaches:

1. **A new trait `ConfigMeta: FromConfig` + an `impl<T: FromConfig> ConfigMeta for T` fallback** —— ✗ invalid. Under Rust's coherence rules, this blanket impl would overlap with the concrete `impl ConfigMeta for AppCfg` generated by `#[derive]` (because `AppCfg: FromConfig`), and compilation fails.
2. **A new trait `ConfigMeta` implemented only for derived types, with leaf types implemented by hand** —— ✗ it is impossible to enumerate all leaves. `String`, all integers, `Duration`, `PathBuf`, `FromStrHolder<V>`, `ByteSize`…… missing any one of them would make the user's struct fail to compile.
3. **Add methods with default implementations to `FromConfig`** —— ✓ recommended.

Advantages of approach 3:

- All types (including `Option<V>`, `Vec<V>`, `HashMap<_, V>`, `RefValue<T>`) **already** implement `FromConfig`, so they can naturally be called.
- Methods with default implementations are **source-compatible**: they break neither this crate's internal implementations nor downstream code that hand-writes `impl FromConfig`.
- Nesting and leaves are expressed through the same entry point; leaves go through the default implementation and return an empty slice, which is semantically natural.

Cost: `FromConfig` takes on two responsibilities, "parsing" and "self-description". Given that this crate deliberately does not introduce serde, this is considered an acceptable trade-off; the documentation needs to clearly mark that these methods are for the exporter/tools.

### 3.2 Public types

Add modules `src/meta.rs` (types) and `src/export.rs` (rendering), re-exported by `src/lib.rs`.

```rust
/// Field kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigFieldKind {
    /// Leaf value (String / numeric / bool / enum string / unknown custom scalar)
    Leaf,
    /// Option<T>
    Optional,
    /// Vec<T>
    Sequence,
    /// HashMap<String, V> / BTreeMap<String, V>
    Map,
}

/// Configuration literal: shared by `sample` and `default`.
///
/// Stores a **structured** value rather than a rendered TOML fragment; see §3.6 for the rationale:
/// the same data must both be renderable into a comment and be feedable back to the parser as a validity probe (§5.9).
#[derive(Clone, Copy, Debug)]
pub enum ConfigLiteral {
    Str(&'static str),
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    /// Array literal, e.g. `sample = ["a.io", "b.io"]`
    Array(&'static [ConfigLiteral]),
}

impl ConfigLiteral {
    /// Renders as a TOML value fragment: `"demo"` / `8080` / `true` / `["a.io", "b.io"]`.
    pub fn to_toml_string(&self) -> String;

    /// Flattens into `(key, value)` pairs, for the probe to build a flat configuration.
    /// Arrays expand into `key[0]`, `key[1]`, consistent with how `Vec<V>` is parsed.
    pub fn to_entries(&self, key: &str) -> Vec<(String, ConfigValue<'static>)>;
}

/// Static description of a single configuration field.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive] // more fields will be added later (see undocumented in §5.8, probe in §5.9)
pub struct ConfigFieldMeta {
    /// Key name after renaming (the result of `#[config(name = ...)]` taking effect)
    pub name: &'static str,
    /// `#[config(desc = ...)]`, or the first paragraph of the doc comment
    pub desc: Option<&'static str>,
    /// `#[config(sample = ...)]`
    pub sample: Option<ConfigLiteral>,
    /// `#[config(default = ...)]`
    pub default: Option<ConfigLiteral>,
    /// Field kind
    pub kind: ConfigFieldKind,
    /// Child field metadata factory; None for leaf types, Some(T::config_fields) for nested structs
    pub nested: Option<fn() -> Vec<ConfigFieldMeta>>,
}

impl ConfigFieldMeta {
    /// Gets the child fields; a leaf returns an empty Vec.
    pub fn nested_fields(&self) -> Vec<ConfigFieldMeta> {
        self.nested.map(|f| f()).unwrap_or_default()
    }

    /// Whether this is an expandable table/array-of-tables (nested is non-empty).
    pub fn is_section(&self) -> bool { self.nested.is_some() }
}
```

On the **lazy function pointer** design of `nested` (important):

- Using `Vec<ConfigFieldMeta>` directly would make a recursive structure such as `Tree { children: Vec<Tree> }` recurse infinitely the instant metadata is constructed (the macro-generated `Vec<Tree>::config_fields()` → `Tree::config_fields()` → ...).
- `Option<fn() -> Vec<ConfigFieldMeta>>` defers the recursion until the exporter actually descends; combined with `max_depth` from §4.4, this guarantees that exporting a recursive type does not hang.
- A function pointer is also `const`-friendly (although this design ultimately uses `Vec` rather than a `const` array; see below).
- `#[non_exhaustive]` is deliberate: the checker in §5 will add `undocumented` and `probe` to this struct, and without it every such addition would be a breaking change.

### 3.3 Extension of `FromConfig`

```rust
pub trait FromConfig: Sized {
    fn from_config(
        context: &mut ConfigContext<'_>,
        value: Option<ConfigValue<'_>>,
    ) -> Result<Self, ConfigError>;

    // ---- The following is self-describing metadata, used only for tooling/export, not involved in parsing ----

    /// Field metadata for this type. Leaf types return empty (default implementation).
    fn config_fields() -> Vec<ConfigFieldMeta> { Vec::new() }

    /// Struct-level `#[config(desc = ...)]`.
    fn config_desc() -> Option<&'static str> { None }

    /// Struct-level `#[config(prefix = ...)]`; same source as `FromConfigWithPrefix::prefix`.
    fn config_prefix() -> Option<&'static str> { None }
}
```

Additional notes:

- `config_prefix` coexists with the existing `FromConfigWithPrefix::prefix()`. When a `prefix` is present, the derive generates **both**, and `FromConfigWithPrefix` stays unchanged to preserve backward compatibility. The reason for not simply requiring `T: FromConfigWithPrefix` is that structs without a prefix also need to be exportable.
- The existing impls for `Option<V>` / `Vec<V>` / `HashMap<_, V>` are **not** rewritten; the macro unwraps the child field at the syntax level and points it at the inner type (see §3.4). This avoids introducing extra delegation logic into this crate's generic impls.
- For `RefValue<T>` (`src/value_ref.rs`), it is recommended to delegate `config_fields/desc/prefix` to `T`, so that refreshable fields can also be exported.

Why return `Vec` instead of `&'static [ConfigFieldMeta]`:

- Filling `<FieldTy as FromConfig>::config_fields()` into an array initializer requires it to be a `const fn`, but a trait method cannot be a `const fn` (stable Rust does not support const fn in traits).
- Export is a low-frequency operation, so one `Vec` allocation is acceptable. If performance demands arise in the future, the exporter can memoize internally by `TypeId`/path.

### 3.4 Illustration of derive expansion

```rust
impl ::cfg_rs::FromConfig for AppCfg {
    fn from_config(/* unchanged */) -> Result<Self, ::cfg_rs::ConfigError> { /* ... */ }

    fn config_desc() -> Option<&'static str> {
        ::core::option::Option::Some("Application configuration")
    }

    fn config_prefix() -> Option<&'static str> {
        ::core::option::Option::Some("app")
    }

    fn config_fields() -> Vec<::cfg_rs::ConfigFieldMeta> {
        ::std::vec![
            ::cfg_rs::ConfigFieldMeta {
                name: "name",
                desc: ::core::option::Option::Some("Service name"),
                sample: ::core::option::Option::Some(
                    ::cfg_rs::ConfigLiteral::Str("demo")
                ),
                default: ::core::option::Option::Some(
                    ::cfg_rs::ConfigLiteral::Str("demo")
                ),
                kind: ::cfg_rs::ConfigFieldKind::Leaf,
                nested: ::core::option::Option::Some(
                    <String as ::cfg_rs::FromConfig>::config_fields
                ),
            },
            ::cfg_rs::ConfigFieldMeta {
                name: "port",
                desc: ::core::option::Option::Some("TCP port"),
                sample: ::core::option::Option::Some(
                    ::cfg_rs::ConfigLiteral::Int(8080)
                ),
                default: ::core::option::Option::Some(
                    ::cfg_rs::ConfigLiteral::Int(8080)
                ),
                kind: ::cfg_rs::ConfigFieldKind::Leaf,
                nested: ::core::option::Option::Some(
                    <u16 as ::cfg_rs::FromConfig>::config_fields
                ),
            },
            // Option<T> -> unwrap to T
            ::cfg_rs::ConfigFieldMeta {
                name: "tls",
                desc: ::core::option::Option::Some("TLS configuration"),
                sample: ::core::option::Option::None,
                default: ::core::option::Option::None,
                kind: ::cfg_rs::ConfigFieldKind::Leaf,
                nested: ::core::option::Option::Some(
                    <TlsCfg as ::cfg_rs::FromConfig>::config_fields
                ),
            },
            // Vec<T> -> kind = Sequence, nested points to T
            ::cfg_rs::ConfigFieldMeta {
                name: "origins",
                desc: ::core::option::Option::Some("Allowed origins"),
                sample: ::core::option::Option::Some(
                    ::cfg_rs::ConfigLiteral::Array(&[
                        ::cfg_rs::ConfigLiteral::Str("a.example.com"),
                        ::cfg_rs::ConfigLiteral::Str("b.example.com"),
                    ])
                ),
                default: ::core::option::Option::None,
                kind: ::cfg_rs::ConfigFieldKind::Sequence,
                nested: ::core::option::Option::Some(
                    <String as ::cfg_rs::FromConfig>::config_fields
                ),
            },
            // ...
        ]
    }
}
```

Macro-side shape determination (pure syntax):

| Field type spelling | `kind` | `nested` points to |
| --- | --- | --- |
| `Option<T>` | `Optional` | `T` |
| `Vec<T>` | `Sequence` | `T` |
| `HashMap<String, V>` / `BTreeMap<String, V>` | `Map` | `V` |
| Other (including type aliases) | `Leaf` | The type itself |

Known degradation: aliases such as `type Ports = Vec<u16>;` are judged as `Leaf` and exported as an ordinary scalar line rather than an array. This is an inherent limitation of syntax-level determination and should be stated explicitly in the documentation; if support is needed, an explicit override such as `#[config(kind = "sequence")]` could be introduced in Phase 2.

### 3.5 Literal storage and rendering rules

At expansion time the macro converts the literals in the annotation into `ConfigLiteral` (structured), while **rendering** is delegated to `ConfigLiteral::to_toml_string()`, which the exporter calls at runtime:

| Rust spelling | `ConfigLiteral` | Rendered result | Notes |
| --- | --- | --- | --- |
| `"demo"` | `Str("demo")` | `"demo"` | Uses TOML basic string escaping (`\`, `"`, control characters) |
| `'x'` | `Char('x')` | `"x"` | char is treated as a single-character string |
| `8080` | `Int(8080)` | `8080` | `Lit::Int::base10_digits()`, automatically strips suffixes such as `u16` |
| `-1` | `Int(-1)` | `-1` | `Expr::Unary(Neg, Lit)`, supported only by `sample` |
| `3.5` | `Float(3.5)` | `3.5` | Float kept as is |
| `true` | `Bool(true)` | `true` | Bare boolean |
| `["a", "b"]` | `Array([Str("a"), Str("b")])` | `["a", "b"]` | Arrays are recursive and elements must be literals; supported only by `sample` |
| `"${server.host:127.0.0.1}"` | `Str("${server.host:127.0.0.1}")` | `"${server.host:127.0.0.1}"` | A placeholder expression is an ordinary string and is preserved as is |

Note: the **runtime** semantics of `default` are still "a string, parsed via `ConfigValue::Str`", and they are unrelated to rendering. Rendering only affects how the artifact looks (`default = 8080` is an integer in TOML, whereas at runtime it is first treated as the string `"8080"` and then parsed as a `u16`; the two are equivalent).

### 3.6 Revision note: why `sample` / `default` store structured values instead of pre-rendered fragments

The initial design stored `sample` / `default` directly as **TOML fragment strings rendered at macro expansion time** (such as `"\"demo\""`). After the completeness checker from §5 was introduced, this decision had to be overturned, for two reasons:

1. **A fragment cannot be fed back into the parser.** To verify "whether `sample = "abc"` can really be parsed into a `u16`", the checker must feed the sample as input into the existing parse pipeline. `"\"abc\""` carries TOML quotes, so feeding it in directly yields the wrong value (the string content becomes `"abc"` including quotes), while stripping the quotes would require implementing parsing all over again — which amounts to implementing the TOML grammar twice.
2. **Arrays cannot be expressed.** The probe for `Vec<T>` needs to flatten `["a", "b"]` into `key[0]`, `key[1]`, and a rendered string cannot achieve structured flattening.

The structured storage lets the same metadata serve three consumers at once, each doing its own job:

| Consumer | Usage |
| --- | --- |
| Exporter (§4) | `ConfigLiteral::to_toml_string()` → written into TOML |
| Checker L1 (§5) | Only determines whether the `Option` is `Some`; does not care about the value |
| Checker L2 probe (§5.9) | `ConfigLiteral::to_entries(key)` → flat key/value table → `Configuration` |

The cost is that `ConfigLiteral` must cover the six forms `Str/Int/Float/Bool/Char/Array` and must render recursively in the `Array` case. Compared with "implementing the grammar twice" or "not providing L2", this is the smaller cost.

---

## 4. Export design

### 4.1 API

```rust
/// Export style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportStyle {
    /// Export only fields that have a default; the artifact can be loaded directly. Fields without a default are listed as comments.
    Defaults,
    /// Export all fields: those with a default are written as key/value pairs, and those without are placeholders as sample comments. The default style.
    Template,
}

/// Export options.
#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    pub style: ExportStyle,
    /// Whether to render `#[config(prefix = ...)]` as a TOML table header. Defaults to true.
    pub include_prefix: bool,
    /// Upper limit on the nesting drill-down depth, preventing infinite loops in recursive structures. Defaults to 8.
    pub max_depth: usize,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self { style: ExportStyle::Template, include_prefix: true, max_depth: 8 }
    }
}

/// Generates a TOML template with comments.
pub fn export_toml<T: FromConfig>() -> String;

/// Version with options.
pub fn export_toml_with<T: FromConfig>(options: &ExportOptions) -> String;

/// Writes into an existing buffer to avoid extra allocation.
pub fn export_toml_into<T: FromConfig>(out: &mut String, options: &ExportOptions);
```

Design points:

- **Depends on no feature**: rendering is pure string concatenation and needs no `toml` / `yaml` / `json` feature. This is consistent with the crate's existing style of "enabling formats by feature" (the corresponding feature is only needed when registering a file source).
- Does not depend on a `Configuration` instance; it is purely the result of statically analyzing `T::config_fields()`.
- If YAML is to be added later, this can be extended to `export_yaml`; JSON has no comments and can only output values (separate `ExportStyle::Json` semantics could be defined), which this design does not commit to.

### 4.2 Value sources and precedence

For each leaf field, rendering takes the "example value" according to the following precedence:

1. `default` (if present it is always used, and it cannot be downgraded to a comment — it is the real default value)
2. `sample` (used under the `Template` style; under the `Defaults` style it is only a comment)
3. Neither is present → under `Template` the field is output as the comment form `# name = <placeholder>`, and under `Defaults` it is **omitted entirely**

`desc` is always output as a `# <desc>` comment (multiple lines are split line by line and prefixed with `# `).

Required fields (those without a `default`) are additionally marked `(required)` under the `Template` style:

```toml
# Worker thread count (required)
# sample: 4
# workers = 4
```

### 4.3 Rendering algorithm (TOML)

```
render_root(meta):
    emit header comments: meta.desc, "prefix: app", "generated by cfg-rs"
    path = if include_prefix { split(meta.prefix) } else { [] }
    render_table(out, path, meta.fields, depth = 0)

render_table(out, path, fields, depth):
    # TOML semantics require scalar keys to be written before subtables; otherwise keys after a table header belong to that subtable
    leaves = fields.filter(|f| !f.is_section() || f.nested_fields().is_empty())
    sections = fields.filter(is_section)

    for f in leaves:
        emit_comment(f)                      # desc / (required) / sample
        if f.default.is_some() or (style == Template and f.sample.is_some()):
            emit  key = value
        else:
            emit  # key = <placeholder>

    for f in sections:
        if depth + 1 > max_depth: emit "# <key> ... (max depth reached)"; continue
        sub_path = path + f.name
        emit_comment(f)
        match f.kind:
            Optional  -> first output the whole comment skeleton `# [sub_path]` (an optional table that is not required to appear)
            Sequence  -> array table: if the sample is an array, generate `[[sub_path]]` according to the number of elements;
                         otherwise output a comment skeleton `# [[sub_path]]`
            Map       -> keys cannot be enumerated, output `# [sub_path]  # dynamic keys`
            Leaf      -> ordinary table `[sub_path]`
        render_table(out, sub_path, f.nested_fields(), depth + 1)
```

Three TOML details that must be written into implementation comments:

1. **Within the same level, scalars must precede subtables**. In TOML, if `x = 1` is written after `[a]`, then `x` belongs to `a` rather than the parent table. The algorithm guarantees this through "leaves before sections".
2. **Array tables `[[x]]` and ordinary tables `[x]` must not be mixed**. `Vec<Struct>` always uses `[[x]]`.
3. **String escaping**: `\`, `"`, `\n`, `\r`, `\t` and U+0000–U+001F control characters must be escaped; line breaks appearing in a `desc` comment are split line by line.

### 4.4 Recursion and depth protection

- `max_depth` defaults to 8; when the limit is exceeded, output `# <key> ... (max depth reached)` and stop drilling down.
- There are two purposes: first, to prevent self-referential structures such as `Tree { children: Vec<Tree> }` from expanding infinitely during export; second, to prevent an extremely deep but legal configuration from blowing up the artifact.
- Because `nested` is a lazy function pointer (§3.2), the next level's `config_fields()` is triggered only when drilling down for real, so the depth protection is effective.

### 4.5 Output example

Input:

```rust
#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "app", desc = "Application configuration")]
struct AppCfg {
    #[config(desc = "Service name", sample = "demo", default = "demo")]
    name: String,

    #[config(desc = "Listen address", sample = "0.0.0.0", default = "0.0.0.0")]
    host: String,

    #[config(desc = "TCP port", sample = 8080, default = 8080)]
    port: u16,

    #[config(desc = "Number of worker threads", sample = 4)]
    workers: usize,

    #[config(desc = "Allowed origins", sample = ["a.io", "b.io"])]
    origins: Vec<String>,

    #[config(desc = "TLS configuration")]
    tls: TlsCfg,
}

#[derive(Debug, cfg_rs::FromConfig)]
struct TlsCfg {
    #[config(desc = "Certificate path", sample = "/etc/tls/cert.pem")]
    cert: String,

    #[config(desc = "Whether enabled", default = false)]
    enabled: bool,
}
```

`export_toml::<AppCfg>()` (the `Template` style, `include_prefix = true`) outputs:

```toml
# Application configuration
# prefix: app
# Generated by cfg-rs. Keep the comments as field documentation.

# Service name
# sample: "demo"
name = "demo"

# Listen address
# sample: "0.0.0.0"
host = "0.0.0.0"

# TCP port
# sample: 8080
port = 8080

# Number of worker threads (required)
# sample: 4
# workers = 4

# Allowed origins
# sample: ["a.io", "b.io"]
origins = ["a.io", "b.io"]

# TLS configuration
[tls]

# Certificate path (required)
# sample: "/etc/tls/cert.pem"
# cert = "/etc/tls/cert.pem"

# Whether enabled
enabled = false
```

The same input under `ExportStyle::Defaults`: `workers` and `[tls].cert` are omitted (everything else is unchanged), and the artifact is a configuration that can be **loaded directly with `register_file`**.

---

## 5. Extension: `ConfigSampleCheck` — sample completeness checking

### 5.1 Motivation

The quality of the `Template` style export from §4 depends entirely on the coverage of `desc` / `sample`: miss one field and the template loses a piece of documentation, and there is no signal at all for this problem at compile time. Once there are many fields and deep nesting, a manual review is bound to miss something.

Therefore an automatable **completeness check** that can run in CI is needed: given a `FromConfig` type, answer "does every field have a `desc` and a `sample`?", and locate the gaps down to the full configuration key path.

### 5.2 Key judgment: why a blanket impl is acceptable here

The conclusion in §3.1 is that "the metadata entry point must hang off `FromConfig`", because a newly created `ConfigMeta` trait cannot fight the coherence rules and the exhaustive enumeration of leaf types. But **the trait in this section is a different situation**:

- Every method of `ConfigSampleCheck` can be given a default implementation, and it **only depends on `FromConfig::config_fields()`**; it does not need the derive to generate anything;
- Therefore `impl<T: FromConfig> ConfigSampleCheck for T` is legal: the derive **does not implement** this trait, so there is no overlap as in §3.1.

Conclusion: the checker is an extension trait with **zero derive cost**. The price is that users cannot customize the checking logic for a particular type (it would conflict with the blanket impl); customization is instead expressed through `SampleCheckOptions` and field-level exemption attributes.

> Memory points:
> - Need "the type itself to answer which fields exist" → must be a **default method** of `FromConfig` (overridden by the derive).
> - Need "to apply a generic algorithm to any `FromConfig`" → an **extension trait with a blanket impl** (this section).
>
> The two do not conflict, provided the latter does not require the derive to generate an implementation.

### 5.3 Two layers of responsibility

| Layer | Name | Question answered | Implementation | Phase |
| --- | --- | --- | --- | --- |
| L1 | Static completeness | Does every field have a `desc` / `sample`? | Pure metadata traversal, no parsing | Phase 1 (no derive change) |
| L2 | Value validity | Can `default` / `sample` really be parsed and pass validation? | Metadata builds a probe configuration, goes through the real pipeline | Phase 2 (needs the `probe` pointer) |

The main body of "sample completeness" is L1; L2 is a high-value capability obtained along the way, see §5.9.

### 5.4 API

```rust
/// Extension trait for sample completeness checking.
///
/// There is `impl<T: FromConfig> ConfigSampleCheck for T`,
/// so any `FromConfig` type automatically gains this capability, with no `#[derive]` needed.
pub trait ConfigSampleCheck: FromConfig {
    /// Do a static completeness check with the default strategy.
    fn check_samples() -> SampleReport {
        Self::check_samples_with(&SampleCheckOptions::default())
    }

    /// Do a static completeness check with the given strategy.
    fn check_samples_with(options: &SampleCheckOptions) -> SampleReport;

    /// Test / CI assertion: panic and print the report when incomplete.
    fn assert_samples_complete() {
        let report = Self::check_samples();
        assert!(report.is_complete(), "{report}");
    }
}

impl<T: FromConfig> ConfigSampleCheck for T {
    fn check_samples_with(options: &SampleCheckOptions) -> SampleReport {
        // Reuse the same metadata walker from §4.3
        let mut report = SampleReport::default();
        walk_meta(T::config_prefix(), &T::config_fields(), options, &mut report);
        report
    }
}

/// Convenience free function, equivalent to `T::check_samples()`.
pub fn check_samples<T: FromConfig>() -> SampleReport { T::check_samples() }
```

The strategy and result types:

```rust
/// The required strength for a sample.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SampleRequirement {
/// Every field must have a sample.
    Always,
/// Only fields with no default must have a sample (the default).
/// When a default exists, the default itself can serve as the example.
    #[default]
    WhenNoDefault,
/// Do not check sample, only check desc.
    Never,
}

/// Check policy.
#[derive(Clone, Copy, Debug)]
pub struct SampleCheckOptions {
/// Whether every field is required to have a desc. Defaults to true.
    pub require_desc: bool,
/// The required strength for a sample. Defaults to `WhenNoDefault`.
    pub sample: SampleRequirement,
/// Whether to recursively check nested sections and array element structures. Defaults to true.
    pub recurse: bool,
/// The recursion depth limit, to prevent recursive types. Defaults to 8 (consistent with the exporter).
    pub max_depth: usize,
/// Whether the key path includes `#[config(prefix = ...)]`. Defaults to true.
    pub include_prefix: bool,
/// Whether an empty-string sample counts as missing (`sample = ""` usually means it was not filled in). Defaults to true.
    pub empty_sample_is_missing: bool,
}

/// Issue classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleIssueKind {
/// Missing desc, so the meaning of the field cannot be explained.
    MissingDesc,
/// Missing sample, so no example value can be given.
    MissingSample,
/// The sample is an empty string.
    EmptySample,
/// The depth limit was reached, so nothing below it was checked.
    DepthLimited,
/// Dynamic key (Map), so the value type can only be checked once.
    DynamicKeys,
}

/// A single issue.
#[derive(Clone, Debug)]
pub struct SampleIssue {
/// The full configuration key path, such as `app.tls.cert`, `app.servers[].name`, `app.labels.*`.
    pub path: String,
/// The field name (after `#[config(name = ...)]` takes effect).
    pub field: &'static str,
/// The issue kind.
    pub kind: SampleIssueKind,
/// Whether the field is required (no default).
    pub required: bool,
/// The field kind.
    pub field_kind: ConfigFieldKind,
/// Existing information, to make the report easier to present.
    pub desc: Option<&'static str>,
/// The rendered sample, used only for display.
    pub sample: Option<String>,
}

/// Check report.
#[derive(Clone, Debug, Default)]
pub struct SampleReport {
/// The issues found.
    pub issues: Vec<SampleIssue>,
/// The total number of fields checked.
    pub checked: usize,
/// The number of fields with both desc and sample present.
    pub documented: usize,
/// Whether it was truncated because of `max_depth`.
    pub truncated: bool,
}

impl SampleReport {
/// Complete when there are no issues.
    pub fn is_complete(&self) -> bool { self.issues.is_empty() }

/// Documentation coverage, used for trend statistics.
    pub fn ratio(&self) -> f32 {
        if self.checked == 0 { 1.0 } else { self.documented as f32 / self.checked as f32 }
    }

/// Human-readable text (`impl Display` calls it).
    pub fn render(&self) -> String;

/// A table that can be pasted directly into docs / PR comments.
    pub fn render_markdown(&self) -> String;
}
```

### 5.5 Decision rules

| Field kind | `desc` | `sample` | Notes |
| --- | --- | --- | --- |
| `Leaf` (scalar) | Always required | Per `SampleRequirement` | Exempt from sample by default when there is a `default` |
| `Optional<T>` | Always required | Per policy | Optional fields can usually be exempted at the policy level |
| section (`nested` non-empty) | Always required | **Not applicable** | The struct itself is not a value; recurse only into child fields |
| `Sequence` with scalar elements | Always required | An array sample is recommended | Reported per policy when there is no sample |
| `Sequence` with section elements | Always required | Not applicable | The path gets `[]` appended, and the element structure is recursed |
| `Map<String, V>` | Always required | Not applicable | The path gets `.*` appended, with a `DynamicKeys` hint, and `V` is recursed once |
| `#[config(undocumented)]` | Skip | Skip | Explicit exemption, see §5.8 |
| `max_depth` reached | — | — | Record a `DepthLimited`, `truncated = true` |

Decision order (must be followed in the implementation, otherwise counting will be duplicated):

1. `undocumented` -> skip directly, not counted in `checked`;
2. section (`nested_fields()` non-empty) -> check only `desc`, count it in `checked`, then decide whether to recurse;
3. leaf / Optional / Sequence -> check `desc` and `sample`, count it in `checked`, and if both are present then `documented += 1`.

Fields that are required (no `default`) and missing a sample carry `required: true` in the report to make prioritization easier -- they are exactly the fields that can only appear as comments in the `Template` export and cannot be loaded directly.

### 5.6 Path model (shared with the exporter)

The checker and the exporter must produce **exactly identical** key paths, otherwise the locations in the report will not match the artifacts. Therefore a shared `MetaPath` is extracted:

```rust
/// A configuration key path during metadata traversal.
pub struct MetaPath(String);

impl MetaPath {
    pub fn root(prefix: Option<&str>) -> Self;
    /// `app` + `tls` -> `app.tls`
    pub fn child(&self, name: &str) -> Self;
/// `app.servers` -> `app.servers[]` (array table element, index symbolized)
    pub fn element(&self) -> Self;
/// `app.labels` -> `app.labels.*` (dynamic key)
    pub fn any_key(&self) -> Self;
    pub fn as_str(&self) -> &str;
}
```

The rendering algorithm in `src/export.rs` (§4.3) and the checker in `src/sample_check.rs` share the same walker:

```rust
pub(crate) fn walk_meta<F>(
    prefix: Option<&str>,
    fields: &[ConfigFieldMeta],
    max_depth: usize,
    visit: &mut F,
) where
    F: FnMut(&MetaPath, &ConfigFieldMeta, usize);
```

> Implementation suggestion: `MetaPath` reuses the existing `CacheString` / `PartialKeyCollector` approach (append + truncate) to avoid a `format!` allocation at every level. Export and checking are both low-frequency operations, so a simple implementation is fine at first, but define the interface in a reusable form from the start so the two walkers do not drift apart later.

### 5.7 Report example

```text
sample completeness: 86.7% (13/15 documented)
  app.workers        missing sample   (required, Leaf/usize)   desc = Number of worker threads
  app.tls.cert       missing sample   (required, Leaf/String)  desc = Certificate path
  app.labels.*       dynamic keys     (Map<String, String>)
2 issues, 0 truncated
```

The output of `render_markdown()`:

| Path | Field | Type | Issue | Required |
| --- | --- | --- | --- | --- |
| `app.workers` | `workers` | Leaf | missing sample | yes |
| `app.tls.cert` | `cert` | Leaf | missing sample | yes |
| `app.labels.*` | `labels` | Map | dynamic keys | no |

### 5.8 Field-level exemption: `#[config(undocumented)]`

Some fields genuinely need no documentation (internal handles, purely machine-generated keys). For this reason a marker is added to `ConfigFieldMeta`:

```rust
pub struct ConfigFieldMeta {
    // ...
    /// `#[config(undocumented)]`: explicitly exempts the field from the completeness check and from exported comments.
    /// Affects metadata only, not `from_config`.
    pub undocumented: bool,
}
```

Difference from `skip` in §2.4:

| Attribute | Metadata | Export | from_config | Purpose |
| --- | --- | --- | --- | --- |
| `skip` | Removed entirely | Does not appear | **Unchanged** (`skip` only affects metadata, see §2.4) | Fields that should not appear in the configuration documentation |
| `undocumented` | Kept | No comment output | Unchanged | Fields that appear in the configuration but need no explanation |

Recommended to implement together with the checker (Phase 2), so that a proper exemption channel exists as soon as the checker ships, instead of forcing users to write `sample = "TBD"` to work around it.

### 5.9 Optional: L2 value probe

L1 can only answer "was it written", not "was it written correctly". `sample = "abc"` paired with a `u16` field, a typo in `default`, a `sample` that does not satisfy `#[validate(range)]` - all of these can be caught in CI, and no new dependencies are needed, because the parser is already there.

The approach is to add a derive-generated **probe function pointer** to `ConfigFieldMeta`:

```rust
pub struct ConfigFieldMeta {
    // ...
    /// Validates the field's value using the real parsing pipeline.
    /// Generated by derive: `|cfg, key| cfg.get::<FieldTy>(key).map(|_| ())`
    pub probe: fn(&Configuration, &str) -> Result<(), ConfigError>,
}
```

Probe flow:

1. Materialize the metadata into a flat key→value table using the same precedence as §4.2 (`default` first, then `sample`); arrays are flattened by `ConfigLiteral::to_entries` into `key[0]`, `key[1]`, which exactly matches how `Vec<V>` is parsed (`collect_keys().int_key`); sections are expanded recursively; Optional with no value is skipped entirely.
2. Load this table with `Configuration::register_kv`.
3. Call `probe(&cfg, &path)` for each field to collect **all** errors (not just the first error).
4. Map `ConfigError` (which itself carries the failing key) to `SampleProbeIssue { path, error }`.

Two granularities and their tradeoffs:

| Granularity | Approach | Catches | Misses |
| --- | --- | --- | --- |
| Per-field | Call `cfg.get::<FieldTy>(key)` for each leaf | Type mismatch, typo in `default`/`sample`, array length problems; can collect all errors | The field's `#[validate(...)]` on the parent struct (validation rules are generated in the parent's `from_config`) |
| Whole | Call `cfg.get::<T>(prefix)` directly | All of the above + parent field-level `validate` + placeholder expansion | Can only get the first error (`from_config` returns on the first error) |

Recommended to provide both: `probe_fields()` collects all type errors, `probe_all()` performs one end-to-end confirmation. For the requirement "does the sample really satisfy the validation rules", `probe_all()` is the key complement.

A semantic boundary that must be made explicit: **the probe is test-time behavior, not runtime behavior**. It feeds `sample` to the parser as input, but does **not change** the rule established in §2.1 that "`sample` is never a runtime fallback value" - at runtime, only `default` can provide a fallback for a missing value.

### 5.10 Usage

```rust
#[derive(Debug, cfg_rs::FromConfig)]
#[config(prefix = "app", desc = "Application configuration")]
struct AppCfg { /* ... */ }

#[test]
fn config_docs_are_complete() {
    AppCfg::assert_samples_complete();
}

#[test]
fn config_samples_are_valid() {
    // Phase 2
    let report = AppCfg::probe_samples();
    assert!(report.is_complete(), "{report}");
}
```

CI can additionally produce coverage for trend tracking:

```rust
println!("{}", AppCfg::check_samples().render_markdown());
```

### 5.11 Relationship with the exporter

- The quality upper bound of `Template`-style export is `SampleReport::ratio()`: the gaps in the report are exactly the comment placeholders in the artifact.
- Recommended to make "export the template + print the report" two outputs of the same task, failing CI directly when a field is missing.
- Both share `walk_meta` and `MetaPath` (§5.6), so there is no inconsistency where "the export produces something the check cannot find".

### 5.12 Compatibility and risks

| Item | Conclusion |
| --- | --- |
| New pub trait + blanket impl | Breaks no existing code; after `prelude` re-export, `T::check_samples()` can be called directly |
| Side effect of the blanket impl | Users cannot customize the trait implementation for their own types; customization can only go through `SampleCheckOptions` (which is sufficient) |
| Adding `undocumented` / `probe` to `ConfigFieldMeta` | The struct is already marked `#[non_exhaustive]` (§3.2), so this is a compatible change |
| `assert_samples_complete` panics | Used only for tests/CI, consistent with the `assert_*` naming convention |
| The probe needs to construct a `Configuration` | Only uses `register_kv`, depends on no format feature, consistent with the zero-dependency constraint of §4 |

---

## 6. Edge case list

| Scenario | Behavior |
| --- | --- |
| Field has neither `desc` nor a doc comment | No meaning comment is output, only `sample` / `default` |
| Field has no `default` and is not `Template` | Omitted |
| `default` is a placeholder `${a.b:def}` | Output as-is as a string; takes effect via placeholder expansion after loading |
| `Option<T>`, where T is a scalar | Same as an ordinary leaf; under `Template`, commented out if required |
| `Option<T>`, where T is a struct | Outputs a `# [path]` comment skeleton, not forced to appear in the configuration |
| `Vec<scalar>` with a `sample` array | Outputs the real array |
| `Vec<struct>` | `[[path]]`; if a sample array exists, expand by element count, otherwise output a comment skeleton |
| `HashMap<String, V>` | Keys cannot be enumerated; outputs `# [path]  # dynamic keys` |
| Type alias (`type A = Vec<T>`) | Treated as `Leaf`, degrades to a scalar line (see §3.4) |
| Recursive struct | Truncated by `max_depth` |
| Empty struct (no fields) | Treated as a leaf; exported as a one-line comment |
| Enum field | The current derive does not support enums (`Only support struct`); handled as a scalar string |
| String contains `"` / `\` / newline | Output after escaping |
| `desc` contains multiple lines | Each line is prefixed with `# ` |

---

## 7. Compatibility

| Change | Impact |
| --- | --- |
| `FromConfig` gains 3 methods with default implementations | **Source compatible**. Downstream hand-written `impl FromConfig` is unaffected; object safety is irrelevant (the trait is already `Sized`). |
| The derive macro newly generates `config_fields/desc/prefix` | Compatible; `FromConfigWithPrefix` is retained. |
| New `sample` keyword | Compatible; it previously caused a compile error and is now legal. |
| Struct attribute error message `Only support prefix` → `Only support prefix/desc/crate` | Only the compile error message changes. |
| `desc` changes from "discarded after parsing" to "entering metadata" | Compatible; does not change any runtime behavior. |

Risk: `FromConfig` is a public trait, and the new methods become part of the documented surface. The doc comments must make clear that these methods are **self-describing metadata** for export/tooling use, not part of the parsing path.

---

## 8. Test plan

1. **Compile time (trybuild, following the `cfg-derive/tests_bak/ui` style)**
   - Legal `desc` / `sample` forms compile successfully.
   - `sample = some_fn()`, `sample = a.b` produce the expected errors.
   - Struct `#[config(desc = ...)]` passes; unknown attributes report the new message.
2. **Metadata assertions**
   - Assert the name (including `name` renames), `desc`, the `ConfigLiteral` form of `sample`, `kind`, and nested pointers item by item.
   - A case where a doc comment falls back to `desc`.
3. **Export snapshots**
   - Both `Defaults` / `Template` styles; nesting, `Option`, `Vec<scalar>`, `Vec<struct>`, `HashMap`, `prefix` on/off, escaping, multi-line `desc`.
4. **Round-trip test (the most valuable one)**
   - For a struct where "all fields have a `default`", parse the output of `export_toml_with::<T>(Defaults)` back into `T` with `inline_source!` / `to_toml`, and assert it is exactly equal to the expected default values.
   - This test directly validates the "complete default configuration" promise and is recommended as the primary acceptance criterion.
5. **Termination**
   - Exporting a recursive struct (`Tree { children: Vec<Tree> }`) does not hang, and the output contains `max depth reached`.
6. **Completeness checker (§5)**
   - Each strategy branch of `SampleCheckOptions` (`Always` / `WhenNoDefault` / `Never`, `require_desc` off).
   - Path assertions: nesting, `[]` for `Vec<struct>`, `.*` for `Map`, `prefix` on/off.
   - Count assertions: `checked` / `documented` / `ratio` and `truncated` of `DepthLimited`.
   - Exemption assertion: `#[config(undocumented)]` does not enter `checked`.
   - L2 probe: a deliberately wrong `sample` (`"abc"` with `u16`), a typo'd `default`, and a `sample` that violates `#[validate(range)]` are all asserted to be caught with the correct path.
   - Negative case: for a type with all desc/sample present, `assert_samples_complete()` must pass (to prevent false positives from the checker).

---

## 9. Alternatives and trade-offs

| Option | Conclusion |
| --- | --- |
| Export based on serde reflection | ✗ Violates this crate's core positioning of not depending on serde |
| Write `desc` only into the generated code's `#[doc]` without metadata | ✗ Cannot export, which amounts to keeping the status quo |
| A `const &'static [ConfigFieldMeta]` constant array | ✗ Nested fields require calling `T::config_fields()`, which is not a `const fn`, so it cannot be evaluated in a const context |
| A separate `ConfigMeta` trait + blanket impl | ✗ Conflicts with the concrete impl generated by the derive macro under the coherence rules, causing a compile failure (see §3.1) |
| Construct a `Default` instance first and then serialize | ✗ Fields without a written `default` have no value to take; and it requires serialization capability (serde or hand-written reflection), which costs more |
| Let `sample` also serve as a runtime fallback | ✗ Would quietly turn "documentation" into "behavior", and is explicitly excluded |
| Also make the checker a default method of `FromConfig` | ✗ Would stuff a "generic algorithm" into the parsing trait, and the strategy/report types have nowhere to live; an extension trait with a blanket impl is more suitable (§5.2) |
| Store rendered TOML fragments for `sample` / `default` | ✗ The v1 approach; once the L2 probe is introduced it must be fed back into the parser, and fragments cannot be safely fed back (§3.6) |

---

## 10. Recommended implementation phases

**Phase 1 (core loop + L1 checker)**

1. Add `sample` to `FieldInfo`; wire `desc` into the metadata.
2. Support `desc` on struct attributes.
3. `src/meta.rs`: `ConfigFieldKind` / `ConfigLiteral` / `ConfigFieldMeta` (with `#[non_exhaustive]`); add 3 default methods to `FromConfig`.
4. The derive macro generates `config_fields/config_desc/config_prefix`.
5. `src/export.rs`: `export_toml*` + the two `ExportStyle` variants.
6. Doc comment fallback for `desc`.
7. `src/sample_check.rs`: `ConfigSampleCheck` + `SampleReport` (**L1 static checking, zero derive changes**, see §5.3).
8. Tests: metadata assertions + export snapshots + round-trip test + checker cases.

**Phase 2 (optional)**

- The `#[config(undocumented)]` exemption channel (paired with L1, see §5.8).
- `#[config(skip)]`.
- **L2 value probe**: `ConfigFieldMeta.probe` + `probe_fields()` / `probe_all()` (§5.9).
- Explicit override via `#[config(kind = "...")]` (addresses type alias degradation).
- YAML export.
- Convenience methods on `Configuration` (for example, exporting the currently effective configuration as a commented template).

**Phase 3 (optional)**

- Integration with the CLI / build scripts: `--export-config` prints the template and the completeness report.
- Coverage trend reporting (`SampleReport::ratio()` changing over time).

---

## 11. Open questions

1. Is it acceptable for `config_prefix()` and `FromConfigWithPrefix::prefix()` to coexist, or is unification desired (the latter requires handling the export entry point for types without a prefix).
2. Should the exporter expose a convenience macro in `src/prelude.rs` (for example `export_config!(AppCfg)`).
3. Should the `ExportStyle` default be `Template` (complete template, required items commented) or `Defaults` (directly loadable)? This design recommends `Template`, because it better matches the demand to "explain field meanings + be complete".
4. ~~Whether `sample` needs to be exposed as a structured value~~ — resolved in v2 as a **structured `ConfigLiteral`** (§3.6); this item is closed.
5. Is the checker's default `SampleRequirement` of `WhenNoDefault` (having a default exempts the field from needing a sample) strict enough? If the team wants "every field to have a readable sample", it should be changed to `Always`.
6. Should a required (no default) field missing a sample get its **own issue severity** (distinct from an ordinary `MissingSample`), so that CI fails only on this kind of hard gap and merely warns for the rest?
7. Is the attribute name `undocumented` appropriate? Alternatives: `no_doc`, `doc_skip`, `examples_only`.
8. Should `ConfigSampleCheck` provide default implementations of `probe_*` (depending on the `probe` pointer), or should L2 be left entirely until after Phase 2 and the API decided then?

---

## Appendix A: List of changes this design makes to existing code

| File | Change |
| --- | --- |
| `cfg-derive/src/lib.rs` | Add `sample` to `FieldInfo`; add a `sample` branch to `derive_config_field_attr` (`Expr` form, see §2.1); add `desc` to `derive_config_prefix`; add a "literal → `ConfigLiteral`" conversion; generate 3 metadata methods; parse `desc` at the struct outer level; in Phase 2 additionally generate the `undocumented` marker and the `probe` pointer |
| `src/lib.rs` | `mod meta; mod export; mod sample_check;`, plus re-export the public items; add 3 default methods and docs to `FromConfig` |
| `src/meta.rs` | New: `ConfigFieldKind` / `ConfigLiteral` / `ConfigFieldMeta` |
| `src/export.rs` | New: `ExportStyle` / `ExportOptions` / `export_toml*` / the shared `walk_meta` and `MetaPath` |
| `src/sample_check.rs` | New: `ConfigSampleCheck` (blanket impl) / `SampleCheckOptions` / `SampleRequirement` / `SampleIssue` / `SampleIssueKind` / `SampleReport` |
| `src/value_ref.rs` | `RefValue<T>` delegates metadata to `T` (optional) |
| `README.md` | Add two subsections, "Field annotations and configuration export" and "Completeness checking" |
| `examples/` | Add an `export_config.rs` example (export + check report) |
| `docs/` | This file |

## Appendix B: One-sentence summary of the current state of `desc`

`desc` is currently a half-finished feature that "can be written, can be parsed, and is discarded"; this design wires it together with the new `sample` into the metadata pipeline and lands it as a commented TOML export. The core engineering judgment is: **the metadata entry point must hang off `FromConfig` (with default implementations), not a new trait**, otherwise it would simultaneously hit the coherence rules and the leaf type exhaustiveness problem.
