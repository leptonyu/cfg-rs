use cfg_rs::*;
use env_logger::{Builder, Logger, Target};
use log::LevelFilter;

#[derive(FromConfig)]
#[config(prefix = "log")]
struct LogEnv {
    #[config(default = "out")]
    target: LogTarget,
    #[config(default = "info")]
    level: LLevel,
}

struct LogTarget(Target);

impl_enum!( LogTarget {
    "stdout" | "out" => LogTarget(Target::Stdout)
    "stderr" | "err" => LogTarget(Target::Stderr)
});

struct LLevel(LevelFilter);

impl_enum!(LLevel {
    "off" => LLevel(LevelFilter::Off)
    "trace" => LLevel(LevelFilter::Trace)
    "debug" => LLevel(LevelFilter::Debug)
    "info" => LLevel(LevelFilter::Info)
    "warn" => LLevel(LevelFilter::Warn)
    "error" => LLevel(LevelFilter::Error)
});

impl From<LogEnv> for Logger {
    fn from(le: LogEnv) -> Self {
        Builder::new()
            .target(le.target.0)
            .filter_level(le.level.0)
            .build()
    }
}

fn main() -> Result<(), ConfigError> {
    let config = Configuration::new();
    let env = config.get_predefined::<LogEnv>()?;
    log::set_max_level(env.level.0);
    log::set_boxed_logger(Box::new(Logger::from(env)))?;
    log::info!("hello");
    Ok(())
}
