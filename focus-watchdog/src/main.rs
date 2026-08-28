// Ukryj konsolę w buildzie release na Windows (w debug zostaje - przydaje się do logów).
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
    let config = config::load_config( env::args().nth(1).as_deref());

    // Loadery egui zgłaszają błędy przez `log`. Bez zainicjowanego loggera
    // komunikaty typu "nie znalazłem pliku" znikają w próżni.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let window_size = config
        .as_ref()
        .and_then(|c| c.window_size)
        .unwrap_or((640.0, 240.0));

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(window_size)
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

    let mut worklog = match worklog::make_worklog(config.clone()) {
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
