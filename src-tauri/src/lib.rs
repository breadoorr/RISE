mod commands;

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
            execute_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");


}