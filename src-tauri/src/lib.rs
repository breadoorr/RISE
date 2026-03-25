// Main entry point of the program
mod commands;
pub mod highlight;
pub mod theme;
mod actions;
mod file_watcher;
mod project;

pub use commands::{
    get_recent_projects,
    create_project,
    execute_command,
    execute_command_with_shell,
    get_default_shell,
    start_process,
    write_to_process,
    kill_process,
    get_system_info,
    get_line_count,
    is_directory,
    change_directory,
    list_files,
    open_project,
    get_project_info,
    get_selected_run_config,
    set_selected_run_config,
    read_file,
    write_file,
    open_buffer,
    get_buffer,
    apply_edit,
    apply_full_update,
    undo_last_change,
    process_key_event,
    get_app_theme,
    update_app_theme,
    search_in_project,
    search_paths_in_project,
    replace_in_project,
    flush_app_config,
};

pub use highlight::highlight_html;
pub use actions::{get_actions, perform_action};
pub use file_watcher::set_watched_path;
pub use commands::FileEntry;
pub use project::{Project, ProjectType, RunConfig};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_recent_projects,
            open_project,
            get_project_info,
            get_selected_run_config,
            set_selected_run_config,
            create_project,
            read_file,
            write_file,
            list_files,
            get_system_info,
            get_line_count,
            is_directory,
            change_directory,
            execute_command,
            execute_command_with_shell,
            get_default_shell,
            start_process,
            write_to_process,
            kill_process,
            highlight_html,
            open_buffer,
            get_buffer,
            apply_edit,
            apply_full_update,
            undo_last_change,
            process_key_event,
            get_actions,
            perform_action,
            get_app_theme,
            update_app_theme,
            search_in_project,
            search_paths_in_project,
            replace_in_project,
        ])
        .setup(|app| {
            let win_builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                    .title("")
                    .inner_size(800.0, 600.0);

            let window = win_builder.build().unwrap();

            // Hook save-on-exit: flush app config when window is closing
            if let Some(win) = app.get_webview_window("main") {
                win.on_window_event(|e| {
                    if let WindowEvent::CloseRequested { .. } = e {
                        let _ = flush_app_config();
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}