mod commands;
mod terminal;

pub use commands::{
    create_project,
    execute_command,
    get_system_info,
    is_directory,
    list_files,
    open_project,
    read_file,
    write_file,
};
pub use terminal::open_system_terminal;

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
            execute_command,
            open_system_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}