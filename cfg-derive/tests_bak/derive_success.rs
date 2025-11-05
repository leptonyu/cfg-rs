use cfg_derive::FromConfig;
use cfg_rs::{from_static_map, Configuration};

#[derive(Debug, PartialEq, Eq, FromConfig)]
struct BasicApp {
    #[config(default = "localhost")]
    host: String,
    #[config(default = 8080)]
    port: u16,
    #[config(name = "renamed")]
    value: u8,
}

#[test]
fn test_basic_defaults_and_rename() {
    let app: BasicApp = from_static_map!(BasicApp, {
        // host missing -> default "localhost"
        // port missing -> default 8080
        "renamed" => "7",
    });

    assert_eq!(
        app,
        BasicApp {
            host: "localhost".to_string(),
            port: 8080,
            value: 7,
        }
    );
}

#[derive(Debug, PartialEq, FromConfig)]
#[config(prefix = "cfg.app")] // also generates FromConfigWithPrefix
struct WithPrefix {
    port: u16,
}

#[test]
fn test_struct_prefix_supports_get_and_get_predefined() {
    // Build a configuration with a key under cfg.app
    let cfg = Configuration::new()
        .register_kv("default")
        .set("cfg.app.port", "9000")
        .finish()
        .expect("finish configuration");

    // Explicit prefix
    let v1: WithPrefix = cfg.get("cfg.app").expect("get with explicit prefix");
    // Via predefined prefix trait
    let v2: WithPrefix = cfg.get_predefined().expect("get_predefined");

    assert_eq!(v1, v2);
    assert_eq!(v1.port, 9000);
}

#[derive(Debug, PartialEq, FromConfig)]
struct LitDefaults {
    #[config(default = "s")]
    s: String,
    #[config(default = b"bs")]
    bs: String,
    #[config(default = b'X')]
    bch: String,
    #[config(default = 'Y')]
    ch: String,
    #[config(default = true)]
    bo: bool,
    #[config(default = 3)]
    n: u8,
    #[config(default = 3.5)]
    f: f32,
}

#[test]
fn test_literal_defaults_cover_all_kinds() {
    // No keys provided, all should fallback to defaults
    let v: LitDefaults = from_static_map!(LitDefaults, {});
    assert_eq!(v.s, "s");
    assert_eq!(v.bs, "bs");
    assert_eq!(v.bch, "X");
    assert_eq!(v.ch, "Y");
    assert_eq!(v.bo, true);
    assert_eq!(v.n, 3);
    // Allow small fp tolerance
    assert!((v.f - 3.5).abs() < 1e-6);
}
