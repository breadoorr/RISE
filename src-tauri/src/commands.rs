use std::env;
use std::fs;
use std::path::Path;
use serde::Serialize;
use hostname;
use std::process::Command;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use streaming_iterator::StreamingIterator;
use crate::highlight::{escape_html, calculate_edit, highlight_ast};

#[derive(Clone, Debug)]
struct EditEntry {
    prev_content: String,
}

#[derive(Clone, Debug)]
pub struct EditorBuffer {
    pub(crate) content: String,
    undo_stack: Vec<EditEntry>,
}

lazy_static! {
    pub static ref EDITOR_BUFFERS: Mutex<HashMap<String, EditorBuffer>> = Mutex::new(HashMap::new());
}

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

// ===== Editor buffer and undo commands =====
#[tauri::command]
pub async fn open_buffer(path: String) -> Result<String, String> {
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    map.insert(path.clone(), EditorBuffer { content: content.clone(), undo_stack: Vec::new() });
    Ok(content)
}

#[tauri::command]
pub async fn get_buffer(path: String) -> Result<String, String> {
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    if let Some(buf) = map.get(&path) {
        return Ok(buf.content.clone());
    }
    // initialize empty if missing
    map.insert(path.clone(), EditorBuffer { content: String::new(), undo_stack: Vec::new() });
    Ok(String::new())
}

fn is_boundary(s: &str, idx: usize) -> bool {
    idx <= s.len() && s.is_char_boundary(idx)
}

#[tauri::command]
pub async fn apply_edit(path: String, start: usize, end: usize, new_text: String) -> Result<String, String> {
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    let buf = map.entry(path.clone()).or_insert(EditorBuffer { content: String::new(), undo_stack: Vec::new() });
    let len = buf.content.len();
    let s = start.min(len);
    let e = end.min(len);
    if s > e { return Err("start greater than end".to_string()); }
    if !is_boundary(&buf.content, s) || !is_boundary(&buf.content, e) {
        return Err("start/end are not at UTF-8 character boundaries".to_string());
    }
    let old_text = &buf.content[s..e];
    // push undo entry (snapshot of full content before edit)
    let entry = EditEntry { prev_content: buf.content.clone() };
    if buf.undo_stack.len() >= 50 { buf.undo_stack.remove(0); }
    buf.undo_stack.push(entry);

    // apply edit
    let mut new_content = String::with_capacity(buf.content.len() - old_text.len() + new_text.len());
    new_content.push_str(&buf.content[..s]);
    new_content.push_str(&new_text);
    new_content.push_str(&buf.content[e..]);
    buf.content = new_content;
    Ok(buf.content.clone())
}

#[tauri::command]
pub async fn undo_last_change(path: String) -> Result<String, String> {
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    if let Some(buf) = map.get_mut(&path) {
        if let Some(entry) = buf.undo_stack.pop() {
            buf.content = entry.prev_content;
        }
        return Ok(buf.content.clone());
    }
    Ok(String::new())
}

#[tauri::command]
pub async fn apply_full_update(path: String, new_content: String) -> Result<String, String> {
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    let buf = map.entry(path.clone()).or_insert(EditorBuffer { content: String::new(), undo_stack: Vec::new() });

    // If unchanged, return current content
    if buf.content == new_content { return Ok(buf.content.clone()); }

    // Push undo snapshot
    let entry = EditEntry { prev_content: buf.content.clone() };
    if buf.undo_stack.len() >= 50 { buf.undo_stack.remove(0); }
    buf.undo_stack.push(entry);

    // Compute a minimal edit using backend calculate_edit for consistency
    if let Some(edit) = calculate_edit(&buf.content, &new_content) {
        // Apply single-span replacement based on calculated byte range
        let s = edit.start_byte.min(buf.content.len());
        let e = edit.old_end_byte.min(buf.content.len());
        if s > e { return Err("calculated start greater than end".to_string()); }
        // UTF-8 boundary check
        if !is_boundary(&buf.content, s) || !is_boundary(&buf.content, e) {
            return Err("calculated start/end are not at UTF-8 character boundaries".to_string()); }
        let mut updated = String::with_capacity(buf.content.len() - (e - s) + (edit.new_end_byte - edit.start_byte));
        updated.push_str(&buf.content[..s]);
        updated.push_str(&new_content[edit.start_byte..edit.new_end_byte]);
        updated.push_str(&buf.content[e..]);
        buf.content = updated;
    } else {
        // No edit detected; fall back to assigning new content (safety)
        buf.content = new_content;
    }

    Ok(buf.content.clone())
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

    // Sort entries: directories first, then files; alphabetical by name (case-insensitive)
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

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

#[tauri::command]
pub async fn get_line_count(path: String) -> Result<usize, String> {
    let mut map = EDITOR_BUFFERS.lock().map_err(|_| "lock poisoned".to_string())?;
    let content = if let Some(buf) = map.get(&path) {
        buf.content.clone()
    } else {
        drop(map);
        fs::read_to_string(&path).unwrap_or_default()
    };
    let count = if content.is_empty() {
        1
    } else {
        content.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1
    };
    Ok(count)
}

#[tauri::command]
pub async fn change_directory(cwd: String, target: String) -> Result<String, String> {
    // Resolve new working directory on the backend for cross-platform correctness
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_else(|_| cwd.clone());
    let trimmed = target.trim().to_string();

    let new_path_buf = if trimmed.is_empty() || trimmed == "~" {
        Path::new(&home).to_path_buf()
    } else if trimmed.starts_with("~/") {
        let rest = trimmed.trim_start_matches("~/");
        Path::new(&home).join(rest)
    } else {
        let t = Path::new(&trimmed);
        if t.is_absolute() {
            t.to_path_buf()
        } else {
            Path::new(&cwd).join(&trimmed)
        }
    };

    if !new_path_buf.exists() {
        return Err(format!("No such directory: {}", new_path_buf.to_string_lossy()));
    }
    if !new_path_buf.is_dir() {
        return Err(format!("Not a directory: {}", new_path_buf.to_string_lossy()));
    }

    let canonical = new_path_buf.canonicalize().unwrap_or(new_path_buf);
    canonical
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to convert path to string".to_string())
}


#[tauri::command]
pub fn execute_command(command: String, cwd: String) -> Result<String, String> {
    // Backward-compatible wrapper that uses the system default shell
    execute_command_with_shell(command, cwd, Some("system".to_string()))
}

fn which_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[cfg(target_family = "unix")]
fn resolve_unix_shell(shell_opt: Option<String>) -> (String, Vec<String>) {
    let choice = shell_opt
        .unwrap_or_else(|| "system".to_string())
        .to_lowercase();

    let (shell_path, args_prefix) = match choice.as_str() {
        "zsh" => {
            let p = if which_exists("/bin/zsh") { "/bin/zsh" } else { "zsh" };
            (p.to_string(), vec!["-c".to_string()])
        }
        "bash" => {
            let p = if which_exists("/bin/bash") { "/bin/bash" } else { "bash" };
            (p.to_string(), vec!["-c".to_string()])
        }
        "sh" => {
            let p = if which_exists("/bin/sh") { "/bin/sh" } else { "sh" };
            (p.to_string(), vec!["-c".to_string()])
        }
        _ => {
            // system default
            let env_shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            let p = if which_exists(&env_shell) { env_shell } else if which_exists("/bin/zsh") {
                "/bin/zsh".to_string()
            } else if which_exists("/bin/bash") {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            };
            (p, vec!["-c".to_string()])
        }
    };

    (shell_path, args_prefix)
}

#[cfg(target_os = "windows")]
fn resolve_windows_shell(shell_opt: Option<String>) -> (String, Vec<String>) {
    let choice = shell_opt
        .unwrap_or_else(|| "system".to_string())
        .to_lowercase();

    match choice.as_str() {
        "powershell" => ("powershell".to_string(), vec!["-NoLogo".to_string(), "-NoProfile".to_string(), "-Command".to_string()]),
        "cmd" => ("cmd".to_string(), vec!["/C".to_string()]),
        _ => {
            // system default
            let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\system32\cmd.exe".to_string());
            // If COMSPEC ends with powershell.exe (unlikely), use powershell semantics
            if comspec.to_lowercase().contains("powershell.exe") {
                (comspec, vec!["-NoLogo".to_string(), "-NoProfile".to_string(), "-Command".to_string()])
            } else {
                (comspec, vec!["/C".to_string()])
            }
        }
    }
}

#[tauri::command]
pub fn execute_command_with_shell(command: String, cwd: String, shell: Option<String>) -> Result<String, String> {
    let path = Path::new(&cwd);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Invalid working directory: {}", cwd));
    }

    #[cfg(target_family = "unix")]
    let (prog, mut args) = resolve_unix_shell(shell);

    #[cfg(target_os = "windows")]
    let (prog, mut args) = resolve_windows_shell(shell);

    args.push(command);

    let output = Command::new(&prog)
        .current_dir(&cwd)
        .args(&args)
        .output()
        .map_err(|e| format!("Command execution failed: {}", e))?;

    let mut stdout = String::from_utf8(output.stdout).unwrap_or_default();
    let mut stderr = String::from_utf8(output.stderr).unwrap_or_default();

    if !stdout.ends_with('\n') && !stdout.is_empty() { stdout.push('\n'); }
    if !stderr.ends_with('\n') && !stderr.is_empty() { stderr.push('\n'); }

    Ok(format!("{}{}", stdout, stderr))
}

#[tauri::command]
pub fn get_default_shell() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // Requirement: on Windows default is cmd
        return Ok("cmd".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        // Requirement: on macOS default is zsh
        return Ok("zsh".to_string());
    }

    #[cfg(target_os = "linux")]
    {
        // Requirement: on Linux default is bash
        return Ok("bash".to_string());
    }
}
