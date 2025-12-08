mod commands;
mod highlight;
mod theme;
mod actions;

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSWindowStyleMask, NSWindowTitleVisibility};
#[cfg(target_os = "macos")]
use cocoa::base::YES;

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
    process_key_event,
};

pub use highlight::highlight_html;
pub use actions::{get_actions, perform_action};

use tauri::{Manager, TitleBarStyle, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
            process_key_event,
            get_actions,
            perform_action,
        ])
        .setup(|app| {
            let win_builder =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                    .title("")
                    .inner_size(800.0, 600.0);

            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(TitleBarStyle::Transparent);

            let window = win_builder.build().unwrap();

            #[cfg(target_os = "macos")]
            unsafe {
                use cocoa::appkit::{NSColor, NSWindow};
                use cocoa::base::{id, nil};

                let ns_window = window.ns_window().unwrap() as id;
                ns_window.setTitleVisibility_(NSWindowTitleVisibility::NSWindowTitleHidden);
                ns_window.setTitlebarAppearsTransparent_(YES);
                ns_window.setStyleMask_(
                    ns_window.styleMask()
                        | NSWindowStyleMask::NSFullSizeContentViewWindowMask,
                );
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}