use std::collections::HashMap;

use crate::{macros::impl_default, ConfigError};

use super::{ConfigSource, ConfigSourceBuilder};

#[doc(hidden)]
#[derive(Debug)]
#[allow(unreachable_pub)]
pub struct Cargo(HashMap<String, String>);

impl_default!(Cargo);

#[allow(dead_code, unreachable_pub)]
impl Cargo {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    #[doc(hidden)]
    pub fn set<K: Into<String>, V: Into<String>>(&mut self, k: K, v: V) {
        self.0.insert(k.into(), v.into());
    }
}

impl ConfigSource for Cargo {
    fn name(&self) -> &str {
        "cargo_env"
    }

    fn load(&self, builder: &mut ConfigSourceBuilder<'_>) -> Result<(), ConfigError> {
        for (k, v) in &self.0 {
            builder.set(k.as_str(), v.to_string());
        }
        Ok(())
    }
}

/// Collect all `CARGO_PKG_*` env variables, and `CARGO_BIN_NAME` into configuration.
///
/// Please refer to [set_cargo_env](struct.PredefinedConfigurationBuilder.html#method.set_cargo_env) for usage.
#[macro_export]
macro_rules! init_cargo_env {
    () => {
        fn init_cargo_env() -> $crate::Cargo {
            let mut builder = $crate::Cargo::new();
init_cargo_env!(builder: "CARGO_PKG_NAME");
init_cargo_env!(builder: "CARGO_PKG_VERSION");
init_cargo_env!(builder: "CARGO_PKG_VERSION_MAJOR");
init_cargo_env!(builder: "CARGO_PKG_VERSION_MINOR");
init_cargo_env!(builder: "CARGO_PKG_VERSION_PATCH");
init_cargo_env!(builder:? "CARGO_PKG_VERSION_PRE");
init_cargo_env!(builder:? "CARGO_PKG_AUTHORS");
init_cargo_env!(builder:? "CARGO_PKG_DESCRIPTION");
init_cargo_env!(builder:? "CARGO_PKG_HOMEPAGE");
init_cargo_env!(builder:? "CARGO_PKG_REPOSITORY");
init_cargo_env!(builder:? "CARGO_PKG_LICENSE");
init_cargo_env!(builder:? "CARGO_PKG_LICENSE_FILE");
init_cargo_env!(builder:? "CARGO_BIN_NAME");
            builder
        }

    };

    ($b:ident:? $x:literal) => {
        if let Some(v) = option_env!($x) {
            $b.set(&$x.to_lowercase().replace("_", "."), v);
        }
    };

    ($b:ident: $x:literal) => {
        $b.set(&$x.to_lowercase().replace("_", "."), env!($x));
    };
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod test {

    use crate::test::*;
    use crate::*;

    #[derive(FromConfig)]
    #[config(crate = "crate")]
    struct CargoPkg {
        name: String,
        version: String,
        description: Option<String>,
    }

    #[derive(FromConfig)]
    #[config(crate = "crate")]
    struct CargoBin {
        #[allow(dead_code)]
        name: String,
    }

    #[derive(FromConfig)]
    #[config(prefix = "cargo", crate = "crate")]
    struct CargoEnv {
        pkg: CargoPkg,
        bin: Option<CargoBin>,
    }

    #[test]
    fn cargo_test() {
        init_cargo_env!();
        let c = init_cargo_env().new_config();
        let cargo = c.get_predefined::<CargoEnv>().unwrap();
        assert_eq!(env!("CARGO_PKG_NAME"), cargo.pkg.name);
        assert_eq!(env!("CARGO_PKG_VERSION"), cargo.pkg.version);
        assert_eq!(
            env!("CARGO_PKG_DESCRIPTION"),
            cargo.pkg.description.unwrap()
        );
        assert!(cargo.bin.is_none());
    }
}
