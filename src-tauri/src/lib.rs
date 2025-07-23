// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::result::Result;
use std::fs;
use std::path::Path;
use serde::Serialize;

#[derive(Serialize)]
struct FileEntry {
    path: String,
    name: String,
    is_dir: bool,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn open_project(path: Option<String>) -> Result<String, String> {
    match path {
        Some(p) => Ok(p),
        None => Err("No path provided".to_string())
    }
}

#[tauri::command]
async fn read_file(path: String) -> Result<String, String> {
    match fs::read_to_string(&path) {
        Ok(content) => Ok(content),
        Err(e) => Err(format!("Failed to read file: {}", e))
    }
}

#[tauri::command]
async fn write_file(path: String, content: String) -> Result<(), String> {
    match fs::write(&path, content) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write file: {}", e))
    }
}

#[tauri::command]
async fn list_files(dir_path: String) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(&dir_path);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path));
    }

    let mut entries = Vec::new();
    match fs::read_dir(path) {
        Ok(dir_entries) => {
            for entry_result in dir_entries {
                match entry_result {
                    Ok(entry) => {
                        let path = entry.path();
                        if let Some(path_str) = path.to_str() {
                            let is_dir = path.is_dir();
                            let name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();

                            entries.push(FileEntry {
                                path: path_str.to_string(),
                                name,
                                is_dir,
                            });
                        }
                    },
                    Err(e) => return Err(format!("Failed to read directory entry: {}", e))
                }
            }
            Ok(entries)
        },
        Err(e) => Err(format!("Failed to read directory: {}", e))
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet, 
            open_project, 
            read_file, 
            write_file, 
            list_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
