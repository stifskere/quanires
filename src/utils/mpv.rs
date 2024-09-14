use std::{process::{Command, Stdio}, thread::spawn};
use tokio::sync::oneshot::{channel, Receiver};
use which::which;

pub fn check_mpv() -> bool {
    which("mpv").is_ok()
}

pub fn run_mpv(title: String, url: String) -> Receiver<bool> {
    let (tx, rx) = channel();

    spawn(move || {
        let result = Command::new("mpv")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .arg(format!("--title={}", title))
            .arg(url)
            .status();

        match result {
            Ok(result) if result.success() => {
                tx.send(true)
                    .expect("");
            },
            _ => {
                tx.send(false)
                    .expect("")
            }
        }
    });

    rx
}
