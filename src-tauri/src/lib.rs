mod commands;
mod highlight;
mod theme;
mod actions;

pub use commands::{
    create_project,
    execute_command,
    execute_command_with_shell,
    get_default_shell,
    get_system_info,
    get_line_count,
    is_directory,
    change_directory,
    list_files,
    open_project,
    read_file,
    write_file,
    open_buffer,
    get_buffer,
    apply_edit,
    apply_full_update,
    undo_last_change,
};

pub use highlight::highlight_html;

pub use actions::get_actions;

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_project,
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
            highlight_html,
            open_buffer,
            get_buffer,
            apply_edit,
            apply_full_update,
            undo_last_change,
            get_actions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}