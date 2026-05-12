use crate::ui::app::MediaApp;

mod core;
mod data;
mod infra;
mod ui;
mod utils;

fn main() -> eframe::Result {
    #[cfg(windows)]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(|s| s.as_str()) == Some(infra::updater::APPLY_UPDATE_ARG) {
            match (args.get(2), args.get(3), args.get(4)) {
                (Some(pid_str), Some(pending), Some(target)) => {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        infra::updater::run_apply_update(pid, pending, target);
                    }
                }
                _ => {}
            }
            return Ok(());
        }
    }

    #[cfg(windows)]
    let _singleton = infra::singleton::acquire();

    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
        use windows::core::HSTRING;
        unsafe {
            let _ = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from("Nexa"));
        }
    }

    let app_icon = {
        let png_bytes = include_bytes!("../assets/icons/icon.png");

        let img = image::load_from_memory(png_bytes)
            .expect("assets/icons/icon.png could not be decoded")
            .into_rgba8();

        let (width, height) = img.dimensions();

        std::sync::Arc::new(egui::IconData {
            rgba: img.into_raw(),
            width,
            height,
        })
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1920.0, 1060.0])
        .with_min_inner_size([1024.0, 600.0])
        .with_title("Nexa")
        .with_decorations(false)
        .with_transparent(true)
        .with_icon(app_icon);

    #[cfg(windows)]
    {
        viewport = viewport.with_transparent(false);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Nexa",
        native_options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
