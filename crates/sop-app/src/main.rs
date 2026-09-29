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
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::manifest_json,
            commands::test_plans,
            commands::duplicate_test_case,
            commands::delete_test_case,
            commands::import_test_case,
            commands::create_test_plan,
            commands::sync_pull,
            commands::testcase_push,
            commands::status,
            commands::validation_report,
            commands::render_markdown,
            commands::procedure_choices,
            commands::settings_rows,
            commands::settings_path,
            commands::settings_set,
            commands::settings_unset,
            commands::settings_repo_path,
            commands::config_load,
            commands::config_save,
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
            commands::run_start,
            commands::run_record,
            commands::run_state,
            commands::run_drift,
            commands::run_end,
            commands::run_attach,
            commands::run_export,
            commands::run_export_to,
            commands::run_summary,
            commands::run_delete,
            commands::run_delete_all,
            commands::load_external_md,
            commands::repo_push,
        ])
        .run(tauri::generate_context!())
        .expect("the window could not start");
}
