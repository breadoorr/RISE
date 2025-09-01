mod commands;
mod highlight;

pub use commands::{
    create_project,
    execute_command,
    execute_command_with_shell,
    get_default_shell,
    get_system_info,
    is_directory,
    change_directory,
    list_files,
    open_project,
    read_file,
    write_file,
    highlight_ast,
    highlight_html,
};

pub fn run() {

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_project,
            create_project,
            read_file,
            write_file,
            list_files,
            get_system_info,
            is_directory,
            change_directory,
            execute_command,
            execute_command_with_shell,
            get_default_shell,
            highlight_ast,
            highlight_html,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");


}