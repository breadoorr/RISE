use std::env;
use std::fs;
use std::path::Path;
use serde::Serialize;
use hostname;
use std::process::Command;

#[derive(Serialize)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Serialize)]
pub struct SystemInfo {
    pub user: String,
    pub host: String,
    pub home: String,
}

#[tauri::command]
pub async fn create_project(path: Option<String>, project_name: Option<String>) -> Result<String, String> {
    match path {
        Some(p) => {
            let base_path = Path::new(&p);
            if !base_path.exists() {
                fs::create_dir_all(base_path).map_err(|e| format!("Failed to create base directory: {}", e))?;
            }
            let folder_name = project_name
                .unwrap_or_else(|| "rise-project".to_string())
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
                .collect::<String>()
                .trim()
                .to_string();
            let folder_name = if folder_name.is_empty() { "rise-project".to_string() } else { folder_name };
            let project_path = base_path.join(&folder_name);
            fs::create_dir_all(&project_path).map_err(|e| format!("Failed to create project directory: {}", e))?;
            let src_path = project_path.join("src");
            if !src_path.exists() {
                fs::create_dir_all(&src_path).map_err(|e| format!("Failed to create src directory: {}", e))?;
            }
            let index_path = src_path.join("index.js");
            let index_content = "// Main entry point for your project\n\nconsole.log('Hello from RISE project!');\n";
            fs::write(&index_path, index_content).map_err(|e| format!("Failed to create index.js file: {}", e))?;
            let readme_path = project_path.join("README.md");
            let readme_content = format!("# {} Project\n\nThis project was created with RISE.\n", folder_name);
            fs::write(&readme_path, readme_content).map_err(|e| format!("Failed to create README.md file: {}", e))?;
            project_path.to_str().map(|s| s.to_string()).ok_or_else(|| "Failed to convert project path to string".to_string())
        },
        None => Err("No path provided".to_string())
    }
}

#[tauri::command]
pub async fn open_project(path: Option<String>) -> Result<String, String> {
    match path {
        Some(p) => Ok(p),
        None => Err("No path provided".to_string())
    }
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))
}

#[tauri::command]
pub async fn list_files(dir_path: String) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(&dir_path);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", dir_path));
    }
    let mut entries = Vec::new();
    let dir_entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;
    for entry_result in dir_entries {
        let entry = entry_result.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if let Some(path_str) = path.to_str() {
            let is_dir = path.is_dir();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();
            entries.push(FileEntry { path: path_str.to_string(), name, is_dir });
        }
    }
    Ok(entries)
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    let user = env::var("USER").or_else(|_| env::var("USERNAME")).unwrap_or_else(|_| "user".to_string());
    let host = hostname::get().map_err(|e| format!("Failed to get hostname: {}", e))?.to_string_lossy().into_owned();
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_else(|_| "/".to_string());
    Ok(SystemInfo { user, host, home })
}

#[tauri::command]
pub async fn is_directory(path: String) -> Result<bool, String> {
    let path = Path::new(&path);
    Ok(path.exists() && path.is_dir())
}

#[cfg(target_family = "unix")]
fn run_unix(cmd: &str, cwd: &str) -> std::io::Result<std::process::Output> {
    use std::process::Command;
    let mut c = Command::new("/bin/sh");
    c.current_dir(cwd)
        .arg("-c")
        .arg(cmd);

    let existing = std::env::var("PATH").unwrap_or_default();
    let defaults = "/usr/local/bin:/usr/local/sbin:/opt/homebrew/bin:/opt/homebrew/sbin:/opt/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";
    let merged = if existing.trim().is_empty() {
        defaults.to_string()
    } else {
        format!("{}:{}", existing, defaults)
    };
    eprintln!("Using PATH: {}", merged);
    c.env("PATH", merged);
    c.env("SHELL", "/bin/sh");

    c.output()
}


#[tauri::command]
pub fn execute_command(command: String, cwd: String) -> Result<String, String> {
    let path = Path::new(&cwd);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid working directory: {}", cwd));
    }

    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .current_dir(&cwd)
        .args(&["/C", &command])
        .output()
        .map_err(|e| format!("Command execution failed: {}", e))?;

    #[cfg(target_family = "unix")]
    let output = run_unix(&command, &cwd)
        .map_err(|e| format!("Command execution failed: {}", e))?;

    let mut stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let mut stderr = String::from_utf8(output.stderr).unwrap_or_default();

    if !stdout.ends_with('\n') && !stdout.is_empty() { stdout.push('\n'); }
    if !stderr.ends_with('\n') && !stderr.is_empty() { stderr.push('\n'); }

    Ok(format!("{}{}", stdout, stderr))
}
