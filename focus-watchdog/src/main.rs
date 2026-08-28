// Ukryj konsolę w buildzie release na Windows (w debug zostaje - przydaje się do logów).
// to robi build windowsowy, bez okienka - nie da się zrobić println! - ale tylko w release - w debug jest!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod worklog;
mod utils;
mod resource;
mod app;
mod task;

use eframe::egui;
use std::env;
use app::App;
    
static APP_NAME: &str = "Focus Watchdog";
static APP_TITLE: &str = "Nope, Finish This First!";

fn main() -> eframe::Result {
    init_logging();
    log::info!("Starging {APP_NAME}");
    let config = config::load_config(env::args().nth(1).as_deref());

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(config.as_ref().unwrap().window_size)
        .with_resizable(false)
        .with_maximize_button(false)
        .with_title(APP_TITLE);

    if let Some(icon) = resource::load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let worklog = match worklog::make_worklog(config.clone()) {
        Ok(w) => w,
        Err(err) => {
            log::error!("Nie udało się utworzyć workloga: {err}");
            std::process::exit(1);
        }
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            // Bez tego obrazki się nie załadują.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx, worklog, config.unwrap_or_default())))
        }),
    )
}

/// Konfiguruje `log`. Poziom sterowany zmienną RUST_LOG, domyślnie `info`.
///
/// W debug piszemy na stderr (konsola jest widoczna).
/// W release konsoli nie ma - `windows_subsystem = "windows"` ją odcina,
/// więc stderr trafiałby w próżnię. Dlatego log idzie do pliku obok exe.
fn init_logging() {

    let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    builder.format_timestamp_secs();
    #[cfg(not(debug_assertions))]
    if let Some(path) = log_file_path() {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                builder.target(env_logger::Target::Pipe(Box::new(file)));
            }
            Err(err) => {
                // Nie ma gdzie tego zgłosić - logger jeszcze nie działa.
                eprintln!("nie mogę otworzyć logu {}: {err}", path.display());
            }
        }
    }
    builder.init();
}
/// Plik logu obok pliku wykonywalnego.
#[cfg(not(debug_assertions))]
fn log_file_path() -> Option<PathBuf> {
    Some(env::current_exe().ok()?.with_extension("log"))
}
