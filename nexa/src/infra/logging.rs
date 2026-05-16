use crate::infra::config::AppConfig;
use std::{fs, path::PathBuf, sync::OnceLock};

use tracing::Level;
use tracing_appender::{
    non_blocking,
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};

use tracing_subscriber::{filter::Targets, fmt, prelude::*};

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn init() {
    let log_dir = get_log_dir();

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("nexa")
        .build(&log_dir)
        .expect("failed to create log appender");

    let (file_writer, guard) = non_blocking(file_appender);

    LOG_GUARD.set(guard).ok();

    let file_layer = fmt::layer()
        .compact()
        .with_ansi(false)
        .with_target(false)
        .with_writer(file_writer);

    #[cfg(debug_assertions)]
    let console_layer = fmt::layer().pretty().with_writer(std::io::stderr);

    #[cfg(debug_assertions)]
    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .with(Targets::new().with_target("Nexa", Level::DEBUG))
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::registry()
        .with(file_layer)
        .with(Targets::new().with_target("Nexa", Level::INFO))
        .init();

    tracing::info!("Nexa {} logging initialized", env!("CARGO_PKG_VERSION"));
    tracing::info!(path = %log_dir.display(), "Log directory");
}

pub(crate) fn get_log_dir() -> PathBuf {
    let dir = AppConfig::get_proj_dirs().data_local_dir().join("logs");

    let _ = fs::create_dir_all(&dir);

    dir
}
