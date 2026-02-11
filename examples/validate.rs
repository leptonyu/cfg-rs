use cfg_rs::*;

#[allow(dead_code)]
#[derive(Debug, FromConfig)]
#[config(prefix = "app")]
struct AppCfg {
    #[validate(range(min = 1, max = 65535))]
    port: u16,
    #[validate(length(min = 1, max = 64))]
    host: String,
    #[validate(regex = "^[a-z0-9_]+$")]
    user: String,
    #[validate(regex = "^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")]
    email: String,
    #[validate(custom = check_threads)]
    threads: usize,
}

fn check_threads(v: &usize) -> Result<(), String> {
    if *v == 0 {
        return Err("threads must be > 0".to_string());
    }
    Ok(())
}

fn main() -> Result<(), ConfigError> {
    let cfg = Configuration::new()
        .register_kv("inline")
        .set("app.port", "8080")
        .set("app.host", "localhost")
        .set("app.user", "user_1")
        .set("app.email", "user@example.com")
        .set("app.threads", "2")
        .finish()?;

    let app: AppCfg = cfg.get_predefined()?;
    println!("{:?}", app);
    Ok(())
}
