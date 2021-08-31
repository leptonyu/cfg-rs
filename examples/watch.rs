use cfg_rs::*;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::time::SystemTime;

fn write_f(f: &str) -> Result<(), ConfigError> {
    std::fs::write(
        f,
        format!(
            "timstamp: {}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
    )?;

    Ok(())
}

fn main() -> Result<(), ConfigError> {
    let p = "target/update.yaml";
    write_f(p)?;
    let conf = Configuration::new().register_file(p, true)?;
    let (tx, rx) = channel();
    let v: RefValue<u128> = conf.get("timstamp")?;

    std::thread::spawn(move || loop {
        write_f(p).unwrap();
        std::thread::sleep(Duration::new(0, 5000000));
        conf.refresh_ref().unwrap();
        tx.send(1u8).unwrap();
    });

    for _ in 0..10 {
        if let Ok(_) = rx.recv() {
            println!("{}", v.get()?);
        }
    }
    std::fs::remove_file(p)?;
    Ok(())
}
