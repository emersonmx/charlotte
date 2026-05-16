use std::{fs, path::Path};
#[allow(unused)]
pub use tracing::{debug, error, info, trace, warn};
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};

pub fn init(log_dir: &Path) -> WorkerGuard {
    let _ = fs::create_dir_all(log_dir);

    let file_appender = RollingFileAppender::new(Rotation::DAILY, log_dir, "tui.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt().with_writer(non_blocking).init();

    if let Ok(entries) = fs::read_dir(log_dir) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
            .collect();
        files.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        while files.len() > 7 {
            if let Some(old) = files.first() {
                let _ = fs::remove_file(old.path());
                files.remove(0);
            }
        }
    }

    guard
}
