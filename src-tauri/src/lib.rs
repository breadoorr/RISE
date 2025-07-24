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
async fn create_project(path: Option<String>, project_name: Option<String>) -> Result<String, String> {
    match path {
        Some(p) => {
            let base_path = Path::new(&p);

            // Create the base directory if it doesn't exist
            if !base_path.exists() {
                match fs::create_dir_all(base_path) {
                    Ok(_) => {},
                    Err(e) => return Err(format!("Failed to create base directory: {}", e))
                }
            }

            // Use provided project name or default to "rise-project"
            // Sanitize the project name to ensure it's valid for a directory name
            let folder_name = project_name
                .unwrap_or_else(|| "rise-project".to_string())
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                .collect::<String>()
                .trim()
                .to_string();

            // If the sanitized name is empty, use the default
            let folder_name = if folder_name.is_empty() { "rise-project".to_string() } else { folder_name };

            // Create the project directory inside the selected path
            let project_path = base_path.join(&folder_name);
            match fs::create_dir_all(&project_path) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to create project directory: {}", e))
            }

            // Create a src directory
            let src_path = project_path.join("src");
            if !src_path.exists() {
                match fs::create_dir_all(&src_path) {
                    Ok(_) => {},
                    Err(e) => return Err(format!("Failed to create src directory: {}", e))
                }
            }

            // Create a basic index file
            let index_path = src_path.join("index.js");
            let index_content = "// Main entry point for your project\n\nconsole.log('Hello from RISE project!');\n";
            match fs::write(&index_path, index_content) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to create index.js file: {}", e))
            }

            // Create a README.md file
            let readme_path = project_path.join("README.md");
            let readme_content = format!("# {} Project\n\nThis project was created with RISE.\n", folder_name);
            match fs::write(&readme_path, readme_content) {
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to create README.md file: {}", e))
            }

            // Return the path to the project directory
            match project_path.to_str() {
                Some(path_str) => Ok(path_str.to_string()),
                None => Err("Failed to convert project path to string".to_string())
            }
        },
        None => Err("No path provided".to_string())
    }
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
            open_project, 
            create_project,
            read_file, 
            write_file, 
            list_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
