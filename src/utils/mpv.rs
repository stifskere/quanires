use std::{process::Command, sync::mpsc::{channel, Receiver}, thread::spawn};
use rustc_hash::FxHashSet;
use which::which;

pub fn check_mpv() -> bool {
    which("mpv").is_ok()
}

pub fn run_mpv(title: String, chapter: i32, urls: FxHashSet<String>) -> Receiver<bool> {
    let (tx, rx) = channel();

    spawn(move || {
        let mut could_play = false;

        for url in urls {
            let success = Command::new("mpv")
                .args([
                    &format!("--title=\"{} | Capitulo {}\"", &title, &chapter),
                    "--no-terminal",
                    &url
                ])
                .status()
                .map(|status|
                    status.success()
                    || status.code()
                        .map(|code| code == -9)
                        .unwrap_or(false)
                )
                .unwrap_or(false);

            if success {
                could_play = true;
                break;
            }
        }

        tx
            .send(could_play)
            .expect("Error de comunicacion de hilos interno.");
    });

    rx
}

pub fn close_mpv() -> Result<(), ()> {
    #[cfg(target_os = "windows")] {
        Command::new("taskkil")
            .args(["/F", "/IM", "mpv.exe"])
            .status()
    }

    #[cfg(target_os = "linux")] {
        Command::new("pkill")
            .args(["-9", "mpv"])
            .status()
    }

    .ok()
    .take_if(|status| status.success())
    .map(|_| ())
    .ok_or(())
}
