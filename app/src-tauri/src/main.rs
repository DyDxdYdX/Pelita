#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod media;
mod projection;
mod store;

use std::sync::Mutex;
use tauri::{Manager, WebviewWindow};

type Outcome<T> = Result<T, String>;

fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn operator(window: &WebviewWindow) -> Outcome<()> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Operator only.".into())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(projection::Session::default()))
        .setup(|app| {
            let profile = app.path().app_data_dir()?;
            let profile = if cfg!(debug_assertions) {
                std::env::var_os("PELITA_DEV_PROFILE")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| profile.join("development"))
            } else {
                profile
            };
            app.manage(store::Store::open(profile).map_err(std::io::Error::other)?);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("pelita", media::serve)
        .invoke_handler(tauri::generate_handler![
            store::library,
            store::save_library,
            store::preflight,
            store::begin_import,
            store::write_import,
            store::finish_import,
            projection::displays,
            projection::confirm,
            projection::projector,
            projection::status,
            projection::prepare_take,
            projection::cancel_take,
            projection::take,
            projection::action
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .run(tauri::generate_context!())
        .expect("start Pelita");
}
