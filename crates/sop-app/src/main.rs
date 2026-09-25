//! The field-sop desktop application.
//!
//! A thin shell: it owns a window, a working copy, and the commands the frontend calls.
//! Every rule lives in `sop-core`, every write goes through `sop-repo`, and the frontend
//! renders what it is given (`docs/DESIGN.md` section 6.6).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod commands;
mod state;

use state::AppState;

fn main() {
    let state = match AppState::load() {
        Ok(state) => state,
        Err(error) => {
            eprintln!("field-sop could not start: {error}");
            std::process::exit(1);
        }
    };
    println!("field-sop: working copy {}", state.root().unwrap_or_default().display());

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::manifest_json,
            commands::status,
            commands::validation_report,
            commands::procedure_choices,
            commands::settings_rows,
            commands::settings_path,
            commands::settings_set,
            commands::settings_unset,
            commands::open_repository,
            commands::remote_url,
            commands::remote_set,
            commands::project_fields,
            commands::project_set,
            commands::step_add,
            commands::step_update,
            commands::step_remove,
            commands::item_move,
            commands::capture_set,
            commands::capture_remove,
            commands::include_add,
            commands::include_remove,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not start");
}
