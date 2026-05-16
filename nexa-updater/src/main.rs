#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod protocol;
mod ui;
mod verify;
mod worker;

use crate::ui::app::UpdaterApp;
use directories::ProjectDirs;
use std::{fs, path::PathBuf};
use tracing::Level;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{filter::Targets, fmt, prelude::*};

fn main() {
    let args = match protocol::parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nexa-updater: bad arguments: {e}");
            std::process::exit(1);
        }
    };

    let _guard = init_logging();
    tracing::info!(
        current = %args.current_version,
        next    = %args.new_version,
        target  = %args.target_exe.display(),
        "nexa-updater starting"
    );

    let app_icon = {
        let png = include_bytes!("../../nexa/assets/icons/icon.png");
        image::load_from_memory(png)
            .expect("icon.png decode failed")
            .into_rgba8()
    };
    let (iw, ih) = app_icon.dimensions();
    let icon_data = std::sync::Arc::new(egui::IconData {
        rgba: app_icon.into_raw(),
        width: iw,
        height: ih,
    });

    let viewport = egui::ViewportBuilder::default()
        .with_title("Nexa Updater")
        .with_inner_size([440.0, 300.0])
        .with_min_inner_size([440.0, 300.0])
        .with_max_inner_size([440.0, 300.0])
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(false)
        .with_always_on_top()
        .with_icon(icon_data);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Nexa Updater",
        native_options,
        Box::new(move |cc| Ok(Box::new(UpdaterApp::new(cc, args)))),
    )
    .expect("eframe failed to start");
}

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = ProjectDirs::from("com", "Envo1d", "Nexa")
        .map(|d| d.data_local_dir().join("logs"))
        .unwrap_or_else(|| PathBuf::from("."));

    let _ = fs::create_dir_all(&log_dir);

    let appender = rolling::RollingFileAppender::builder()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix("nexa-updater")
        .build(&log_dir)
        .expect("log appender failed");

    let (writer, guard) = non_blocking(appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(false)
                .with_target(false)
                .with_writer(writer),
        )
        .with(Targets::new().with_target("nexa_updater", Level::INFO))
        .init();

    guard
}
