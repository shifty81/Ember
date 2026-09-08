mod app;

use macroquad::prelude::Conf;
use std::backtrace::Backtrace;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

fn window_conf() -> Conf {
    app::window_conf()
}

fn install_panic_log_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let logs = log_root();
        let _ = std::fs::create_dir_all(&logs);
        let crash = format!(
            "Havenwild native editor panic\n{info}\n\nBacktrace:\n{}\n",
            Backtrace::force_capture()
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(logs.join("haven_editor_native_crash.log"))
            .and_then(|mut file| file.write_all(crash.as_bytes()));
        append_editor_log(&format!("panic: {info}"));
        default_hook(info);
    }));
}

fn log_root() -> std::path::PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    if current.join("Cargo.toml").is_file() || current.join("content").is_dir() {
        return current.join("logs");
    }
    if let Ok(executable) = std::env::current_exe() {
        for candidate in executable.ancestors().skip(1).take(5) {
            if candidate.join("content").is_dir() || candidate.join("Cargo.toml").is_file() {
                return candidate.join("logs");
            }
        }
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("logs")
}

pub(crate) fn append_editor_log(message: &str) {
    let logs = log_root();
    let _ = std::fs::create_dir_all(&logs);
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(logs.join("haven_editor_native.log"))
    {
        let _ = writeln!(file, "[{}] {}", log_stamp(), message);
        let _ = file.flush();
    }
}

fn log_stamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[macroquad::main(window_conf)]
async fn main() {
    install_panic_log_hook();
    append_editor_log("Havenwild native editor started");
    append_editor_log(&format!("working directory: {:?}", std::env::current_dir()));
    append_editor_log(&format!("executable: {:?}", std::env::current_exe()));
    append_editor_log(&format!(
        "arguments: {:?}",
        std::env::args().collect::<Vec<_>>()
    ));
    app::run().await;
}
