use cfg_rs::*;
use env_logger::{Builder, Logger, Target};
use log::LevelFilter;

#[derive(FromConfig)]
#[config(prefix = "log")]
struct LogEnv {
    #[config(default = "out")]
    target: LogTarget,
    #[config(default = "info")]
    level: LevelFilter,
}

struct LogTarget(Target);

impl_enum!( LogTarget {
    "stdout" | "out" => LogTarget(Target::Stdout)
    "stderr" | "err" => LogTarget(Target::Stderr)
});

impl From<LogEnv> for Logger {
    fn from(le: LogEnv) -> Self {
        Builder::new()
            .target(le.target.0)
            .filter_level(le.level)
            .build()
    }
}

fn main() -> Result<(), ConfigError> {
    let config = Configuration::with_predefined()?;
    let env = config.get_predefined::<LogEnv>()?;
    log::set_max_level(env.level);
    log::set_boxed_logger(Box::new(Logger::from(env))).map_err(ConfigError::from_cause)?;
    let mut i = 0;
    for name in config.source_names() {
        i += 1;
        log::info!("{}: {}", i, name);
    }
    log::info!("hello {}", config.get::<String>("hello.toml").unwrap());
    Ok(())
}
