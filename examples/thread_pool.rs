use std::{path::PathBuf, time::Duration};

use cfg_rs::*;

/// Generic Pool Configuration.
#[derive(FromConfig, Debug)]
pub struct PoolConfig {
    #[config(default = "${pool.max_size:5}")]
    max_size: Option<u32>,
    #[config(default = "${pool.min_idle:1}")]
    min_idle: Option<u32>,
    #[config(default = "${pool.thread_name:}")]
    thread_name: Option<String>,
    #[config(default = "${pool.thread_nums:}")]
    thread_nums: Option<usize>,
    #[config(default = "${pool.test_on_check_out:}")]
    test_on_check_out: Option<bool>,
    #[config(default = "${pool.max_lifetime:}")]
    max_lifetime: Option<Duration>,
    #[config(default = "${pool.idle_timeout:}")]
    idle_timeout: Option<Duration>,
    #[config(default = "${pool.connection_timeout:1s}")]
    connection_timeout: Option<Duration>,
    #[config(default = "${pool.wait_for_init:false}")]
    wait_for_init: bool,
}

#[derive(FromConfig, Debug)]
#[config(prefix = "postgresql")]
pub struct PostgresConfig {
    host: String,
    port: Option<u16>,
    #[config(default = "postgres")]
    user: String,
    password: Option<String>,
    dbname: Option<String>,
    options: Option<String>,
    #[config(default = "${app.name}")]
    application_name: Option<String>,
    #[config(default = "500ms")]
    connect_timeout: Option<Duration>,
    keepalives: Option<bool>,
    keepalives_idle: Option<Duration>,
    #[config(default = "true")]
    must_allow_write: bool,
    ssl: Option<PostgresSslConfig>,
    pool: PoolConfig,
}

#[derive(FromConfig, Debug)]
pub struct PostgresSslConfig {
    cert_path: PathBuf,
}

fn main() -> Result<(), ConfigError> {
    let config = Configuration::with_defaults_builder()
        .set("postgresql.host", "10.10.0.1")
        .set("postgresql.application_name", "primary")
        .set("postgresql.secondary.host", "10.10.0.2")
        .set("postgresql.secondary.application_name", "secondary")
        .init()?;

    // Equal to key "postgresql".
    let pool = config.get_predefined::<PostgresConfig>()?;
    assert_eq!(Some("primary"), pool.application_name.as_deref());
    assert_eq!("10.10.0.1", pool.host);
    println!("{:?}", pool);

    let pool2 = config.get::<PostgresConfig>("postgresql.secondary")?;
    assert_eq!(Some("secondary"), pool2.application_name.as_deref());
    assert_eq!("10.10.0.2", pool2.host);
    println!("{:?}", pool2);
    Ok(())
}
