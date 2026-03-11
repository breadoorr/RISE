use std::env;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use hostname;
use std::process::Command;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::fs::File;
use std::sync::Mutex;
use streaming_iterator::StreamingIterator;
use crate::highlight::{escape_html, calculate_edit, highlight_ast};
use serde_json;
use crate::theme::reload_theme;
use tauri::State;
use tauri::Emitter;
use std::ffi::OsStr;

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
    pub static ref CONFIG_FILE: String = Path::new("/Users/ddorabble/RISE").to_string_lossy().into_owned();
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppConfig {
    recent_projects: Vec<(String, String)>,
    theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig { recent_projects: Vec::new(), theme: "default".to_string() }
    }
}

fn load_config() -> AppConfig {
    let path = Path::new(&*CONFIG_FILE);
    if !path.exists() {
        // create with defaults
        fs::create_dir(path).expect("Failed to create config directory");
        let cfg = AppConfig::default();
        let _ = save_config(&cfg);
        return cfg;
    }
    let content = fs::read_to_string(path.join("config.json")).unwrap_or_else(|_| String::new());
    if content.trim().is_empty() {
        let cfg = AppConfig::default();
        let _ = save_config(&cfg);
        return cfg;
    }
    match serde_json::from_str::<AppConfig>(&content) {
        Ok(mut cfg) => {
            // Backfill defaults if fields are missing
            if cfg.theme.is_empty() { cfg.theme = "default".to_string(); }
            if cfg.recent_projects.len() > 10 { cfg.recent_projects.truncate(10); }
            cfg
        }
        Err(_) => {
            // If existing file is not valid JSON (from previous versions), reset to defaults
            let cfg = AppConfig::default();
            let _ = save_config(&cfg);
            cfg
        }
    }
}

fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let json = serde_json::to_string_pretty(cfg).map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(Path::new(&*CONFIG_FILE).join("config.json"), json).map_err(|e| format!("Failed to write config file: {}", e))
}

fn update_recent_project(project_name: &str, project_path: &str) -> Result<(), String> {
    let mut cfg = load_config();
    // remove existing occurrences
    cfg.recent_projects.retain(|(p, _)| p != project_path);
    // insert at front
    cfg.recent_projects.insert(0, (project_path.to_string(), project_name.to_string()));
    // cap at 10
    if cfg.recent_projects.len() > 10 { cfg.recent_projects.truncate(10); }
    save_config(&cfg)
}

#[tauri::command]
pub fn get_recent_projects() -> Result<Vec<(String, String)>, String> {
    let cfg = load_config();
    Ok(cfg.recent_projects)
}

#[tauri::command]
pub fn get_app_theme() -> String {
    let cfg = load_config();
    cfg.theme.clone()
}

#[tauri::command]
pub fn update_app_theme(app: tauri::AppHandle, new_theme: String) {
    let mut cfg = load_config();
    cfg.theme = new_theme;
    save_config(&cfg).expect("failed to change theme");
    reload_theme();
    // Emit a global event so the frontend can re-render highlighted text
    let _ = app.emit("theme-changed", cfg.theme.clone());
    cfg = load_config();
    println!("Theme changed to {}", cfg.theme);
}

#[derive(Serialize, Deserialize)]
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
pub async fn create_project(path: Option<String>, project_name: Option<String>, template: Option<String>) -> Result<String, String> {
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
            let chosen_template = template.unwrap_or_else(|| "Blank".to_string());
            println!("Creating project with template: {}", chosen_template);
            let project_path = base_path.join(&folder_name);
            fs::create_dir_all(&project_path).map_err(|e| format!("Failed to create project directory: {}", e))?;
            if chosen_template == "NPM" {
                let src_path = project_path.join("src");
                if !src_path.exists() {
                    fs::create_dir_all(&src_path).map_err(|e| format!("Failed to create src directory: {}", e))?;
                }
                let index_path = src_path.join("index.js");
                let index_content = "// Main entry point for your project\n\nconsole.log('Hello from RISE project!');\n";
                fs::write(&index_path, index_content).map_err(|e| format!("Failed to create index.js file: {}", e))?;
                let readme_path = project_path.join("README.md");
                let readme_content = format!("# {} Project\n\nThis project was created with RISE.\n\nTemplate (mock): {}\n", folder_name, chosen_template);
                fs::write(&readme_path, readme_content).map_err(|e| format!("Failed to create README.md file: {}", e))?;
            } else if chosen_template == "Rust" {
                let src_path = project_path.join("src");
                if !src_path.exists() {
                    fs::create_dir_all(&src_path).map_err(|e| format!("Failed to create src directory: {}", e))?;
                }
                let main_path = src_path.join("main.rs");
                let main_content = "// Main entry point for your project\n\nfn main() {\n    println!(\"Hello from RISE project!\");\n}\n";
                fs::write(&main_path, main_content).map_err(|e| format!("Failed to create main.rs file: {}", e))?;
            }
            // Ensure config exists and update recent projects list
            update_recent_project(folder_name.as_str(), project_path.to_str().unwrap_or_default())?;
            project_path.to_str().map(|s| s.to_string()).ok_or_else(|| "Failed to convert project path to string".to_string())
        },
        None => Err("No path provided".to_string())
    }
}

#[tauri::command]
pub async fn open_project(app: tauri::AppHandle, path: Option<String>) -> Result<String, String> {
    match path {
        Some(p) => {
            // Ensure config exists and update the recent projects list
            update_recent_project(Path::new(&p).file_name().unwrap().to_str().unwrap_or("Unknown"), &p)?;
            // Start/point the file watcher to this project path
            crate::file_watcher::set_watched_path(app, p.clone());
            Ok(p)
        },
        None => Err("No path provided".to_string())
    }
}

#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {}", e))
}

#[tauri::command]
pub async fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("Failed to write file: {}", e))?;
    // mark this as a self-generated change to prevent watcher echo
    crate::file_watcher::mark_self_write(&path);
    Ok(())
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchResultItem {
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub line_text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PathSearchItem {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
}

fn should_ignore_path(p: &Path) -> bool {
    if let Some(s) = p.to_str() {
        let low = s.to_lowercase();
        if low.contains("/node_modules/") || low.ends_with("/node_modules") { return true; }
        if low.contains("/.git/") || low.ends_with("/.git") { return true; }
        if low.contains("/target/") || low.ends_with("/target") { return true; }
    }
    false
}

fn is_probably_text(path: &Path) -> bool {
    // simple heuristic: limit size and ensure no NUL bytes in first chunk
    if let Ok(meta) = fs::metadata(path) {
        if !meta.is_file() { return false; }
        if meta.len() > 2 * 1024 * 1024 { // 2MB cap
            return false;
        }
    } else { return false; }
    if let Ok(mut f) = File::open(path) {
        let mut buf = [0u8; 1024];
        if let Ok(n) = f.read(&mut buf) {
            if buf[..n].iter().any(|&b| b == 0) { return false; }
        }
        return true;
    }
    false
}

#[tauri::command]
pub async fn search_paths_in_project(
    root_path: String,
    query: String,
    case_sensitive: bool,
    include_dirs: bool,
    include_files: bool,
    max_results: Option<usize>,
) -> Result<Vec<PathSearchItem>, String> {
    if query.is_empty() { return Ok(vec![]); }
    let root = Path::new(&root_path);
    if !root.exists() || !root.is_dir() { return Err("Invalid project root".to_string()); }
    if !include_dirs && !include_files { return Ok(vec![]); }

    let mut out: Vec<PathSearchItem> = Vec::new();
    let limit = max_results.unwrap_or(5000);
    let qcmp = if case_sensitive { query.clone() } else { query.to_lowercase() };

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if should_ignore_path(&dir) { continue; }
        let rd = match fs::read_dir(&dir) { Ok(d) => d, Err(_) => continue };
        for ent in rd {
            if out.len() >= limit { break; }
            if let Ok(e) = ent {
                let p = e.path();
                if should_ignore_path(&p) { continue; }
                let is_dir = p.is_dir();
                if is_dir { stack.push(p.clone()); }
                let include = (is_dir && include_dirs) || (!is_dir && include_files);
                if !include { continue; }
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                let ncmp = if case_sensitive { name.clone() } else { name.to_lowercase() };
                if ncmp.contains(&qcmp) {
                    out.push(PathSearchItem { path: p.to_string_lossy().into_owned(), name, is_dir });
                }
            }
        }
        if out.len() >= limit { break; }
    }
    Ok(out)
}

#[tauri::command]
pub async fn search_in_project(root_path: String, query: String, case_sensitive: bool, regex: bool, max_results: Option<usize>) -> Result<Vec<SearchResultItem>, String> {
    if query.is_empty() { return Ok(vec![]); }
    let root = Path::new(&root_path);
    if !root.exists() || !root.is_dir() {
        return Err("Invalid project root".to_string());
    }
    let mut out: Vec<SearchResultItem> = Vec::new();
    let limit = max_results.unwrap_or(5000);

    let query_cmp = if case_sensitive { query.clone() } else { query.to_lowercase() };

    fn scan_file(path: &Path, query_cmp: &str, orig_query: &str, case_sensitive: bool, out: &mut Vec<SearchResultItem>, limit: usize) {
        if out.len() >= limit { return; }
        let content = match fs::read_to_string(path) { Ok(s) => s, Err(_) => return };
        let haystack = if case_sensitive { content.as_str().to_string() } else { content.to_lowercase() };
        let mut idx = 0usize;
        while out.len() < limit {
            if let Some(pos) = haystack[idx..].find(query_cmp) {
                let abs = idx + pos;
                // compute line and col
                let prefix = &content[..abs.min(content.len())];
                let line = prefix.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1;
                let line_start = {
                    let mut i = abs.min(content.len());
                    while i > 0 && content.as_bytes()[i-1] != b'\n' { i -= 1; }
                    i
                };
                let column = abs - line_start + 1;
                let line_end = {
                    let mut i = abs.min(content.len());
                    while i < content.len() && content.as_bytes()[i] != b'\n' { i += 1; }
                    i
                };
                let mut line_text = content[line_start..line_end].to_string();
                if line_text.len() > 400 { line_text.truncate(400); }
                out.push(SearchResultItem { path: path.to_string_lossy().into_owned(), line, column, line_text });
                idx = abs + orig_query.len();
                if idx >= haystack.len() { break; }
            } else { break; }
        }
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if should_ignore_path(&dir) { continue; }
        let rd = match fs::read_dir(&dir) { Ok(d) => d, Err(_) => continue };
        for ent in rd {
            if out.len() >= limit { break; }
            if let Ok(e) = ent {
                let p = e.path();
                if should_ignore_path(&p) { continue; }
                if p.is_dir() {
                    stack.push(p);
                } else if is_probably_text(&p) {
                    scan_file(&p, &query_cmp, &query, case_sensitive, &mut out, limit);
                }
            }
        }
        if out.len() >= limit { break; }
    }
    Ok(out)
}

#[tauri::command]
pub async fn replace_in_project(root_path: String, query: String, replacement: String, case_sensitive: bool, regex: bool, dry_run: bool, max_files: Option<usize>) -> Result<usize, String> {
    if query.is_empty() { return Ok(0); }
    let root = Path::new(&root_path);
    if !root.exists() || !root.is_dir() {
        return Err("Invalid project root".to_string());
    }
    let mut changed_files = 0usize;
    let mut processed = 0usize;
    let limit_files = max_files.unwrap_or(500);

    let cmp_query = if case_sensitive { query.clone() } else { query.to_lowercase() };

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if should_ignore_path(&dir) { continue; }
        let rd = match fs::read_dir(&dir) { Ok(d) => d, Err(_) => continue };
        for ent in rd {
            if processed >= limit_files { break; }
            if let Ok(e) = ent {
                let p = e.path();
                if should_ignore_path(&p) { continue; }
                if p.is_dir() {
                    stack.push(p);
                } else if is_probably_text(&p) {
                    processed += 1;
                    let content = match fs::read_to_string(&p) { Ok(s) => s, Err(_) => continue };
                    let hay = if case_sensitive { content.clone() } else { content.to_lowercase() };
                    if !hay.contains(&cmp_query) { continue; }
                    let new_content = if case_sensitive { content.replace(&query, &replacement) } else {
                        // naive case-insensitive replace: iterate manually
                        let mut out = String::with_capacity(content.len());
                        let mut i = 0usize;
                        let lower = hay.as_str();
                        let ql = cmp_query.len();
                        while i < content.len() {
                            if i + ql <= content.len() && &lower[i..i+ql] == cmp_query {
                                out.push_str(&replacement);
                                i += ql;
                            } else {
                                // push next char from original to keep casing intact
                                let ch = content[i..].chars().next().unwrap();
                                out.push(ch);
                                i += ch.len_utf8();
                            }
                        }
                        out
                    };
                    if new_content != content {
                        if !dry_run {
                            // Update open buffer if present, else write to disk
                            let path_str = p.to_string_lossy().into_owned();
                            {
                                let mut map = EDITOR_BUFFERS.lock().unwrap();
                                if let Some(buf) = map.get_mut(&path_str) {
                                    // push undo snapshot
                                    let entry = EditEntry { prev_content: buf.content.clone() };
                                    if buf.undo_stack.len() >= 50 { buf.undo_stack.remove(0); }
                                    buf.undo_stack.push(entry);
                                    buf.content = new_content.clone();
                                }
                            }
                            if let Err(e) = fs::write(&p, &new_content) {
                                eprintln!("Failed to write {}: {}", p.to_string_lossy(), e);
                            } else {
                                crate::file_watcher::mark_self_write(&p);
                                changed_files += 1;
                            }
                        } else {
                            changed_files += 1;
                        }
                    }
                }
            }
        }
        if processed >= limit_files { break; }
    }
    Ok(changed_files)
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


#[derive(Serialize)]
pub struct KeyEventResult {
    pub content: String,
    pub selection_start: usize,
    pub selection_end: usize,
}

fn detect_language_from_filename(path: &str) -> &'static str {
    let p = path.to_lowercase();
    if p.ends_with(".rs") { "rust" }
    else if p.ends_with(".py") { "python" }
    else if p.ends_with(".c") || p.ends_with(".h") { "c" }
    else if p.ends_with(".java") { "java" }
    else if p.ends_with(".cs") { "c_sharp" }
    else if p.ends_with(".sql") { "sequel" }
    else if p.ends_with(".ts") || p.ends_with(".tsx") || p.ends_with(".js") || p.ends_with(".jsx") { "typescript" }
    else { "typescript" }
}

fn line_comment_for_language(lang: &str) -> &'static str {
    match lang {
        "python" => "#",
        "sequel" => "--",
        _ => "//",
    }
}

fn clamp_index(s: &str, idx: usize) -> usize {
    idx.min(s.len())
}

fn line_start_at(s: &str, mut idx: usize) -> usize {
    let bytes = s.as_bytes();
    if idx > bytes.len() { idx = bytes.len(); }
    while idx > 0 {
        if bytes[idx - 1] == b'\n' { break; }
        idx -= 1;
    }
    idx
}

fn line_end_at(s: &str, mut idx: usize) -> usize {
    let bytes = s.as_bytes();
    if idx > bytes.len() { idx = bytes.len(); }
    while idx < bytes.len() {
        if bytes[idx] == b'\n' { break; }
        idx += 1;
    }
    idx
}

fn prev_non_ws_char_before(s: &str, idx: usize) -> Option<char> {
    let mut i = idx.min(s.len());
    while i > 0 {
        let ch = s[..i].chars().rev().next()?; // expensive but fine for small spans
        let ch_len = ch.len_utf8();
        i -= ch_len;
        if ch.is_ascii() { return Some(ch); }
    }
    None
}

fn whitespace_prefix_at(s: &str, line_start: usize) -> String {
    let mut out = String::new();
    let mut i = line_start;
    let bytes = s.as_bytes();
    while i < s.len() {
        let b = bytes[i];
        if b == b' ' { out.push(' '); i += 1; }
        else if b == b'\t' { out.push('\t'); i += 1; }
        else { break; }
    }
    out
}

fn apply_with_undo(path: &str, new_content: String) -> Result<String, String> {
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    let buf = map.entry(path.to_string()).or_insert(EditorBuffer { content: String::new(), undo_stack: Vec::new() });
    let entry = EditEntry { prev_content: buf.content.clone() };
    if buf.undo_stack.len() >= 50 { buf.undo_stack.remove(0); }
    buf.undo_stack.push(entry);
    buf.content = new_content;
    Ok(buf.content.clone())
}

#[tauri::command]
pub async fn process_key_event(
    path: String,
    key: String,
    selection_start: usize,
    selection_end: usize,
    shift: bool,
    ctrl: bool,
    meta: bool,
    _alt: bool,
) -> Result<KeyEventResult, String> {
    // get current content
    let mut map = EDITOR_BUFFERS.lock().unwrap();
    let buf = map.entry(path.clone()).or_insert(EditorBuffer { content: String::new(), undo_stack: Vec::new() });
    let content = buf.content.clone();
    drop(map);

    let mut start = clamp_index(&content, selection_start);
    let mut end = clamp_index(&content, selection_end);
    if start > end { std::mem::swap(&mut start, &mut end); }

    let lang = detect_language_from_filename(&path);
    let indent_unit = "    ";

    let mut new_content = content.clone();
    let mut new_sel_start = start;
    let mut new_sel_end = end;

    let handled = if key == "Tab" && !shift {
        if start == end {
            // insert indent at caret
            new_content = format!("{}{}{}", &content[..start], indent_unit, &content[start..]);
            new_sel_start = start + indent_unit.len();
            new_sel_end = new_sel_start;
        } else {
            // indent all selected lines
            let ls = line_start_at(&content, start);
            let le = line_end_at(&content, end);
            let mut out = String::with_capacity(content.len() + 8);
            out.push_str(&content[..ls]);
            let mut i = ls;
            let mut first_line = true;
            let mut added_lines = 0usize;
            while i < le {
                out.push_str(indent_unit);
                added_lines += 1;
                let next_nl = {
                    let mut j = i;
                    while j < content.len() && content.as_bytes()[j] != b'\n' { j += 1; }
                    j
                };
                out.push_str(&content[i..next_nl]);
                if next_nl < content.len() && content.as_bytes()[next_nl] == b'\n' {
                    out.push('\n');
                    i = next_nl + 1;
                } else {
                    i = next_nl;
                }
                if first_line { first_line = false; }
            }
            out.push_str(&content[le..]);
            new_content = out;
            new_sel_start = start + indent_unit.len();
            new_sel_end = end + indent_unit.len() * added_lines;
        }
        true
    } else if key == "Tab" && shift {
        // outdent selected lines
        let ls = line_start_at(&content, start);
        let le = line_end_at(&content, end);
        let mut out = String::with_capacity(content.len());
        out.push_str(&content[..ls]);
        let mut i = ls;
        let mut changed_lines = 0usize;
        let mut removed_sum = 0usize;
        while i < le {
            // remove up to 4 leading spaces or one tab
            let mut j = i;
            let mut removed = 0usize;
            while j < content.len() && removed < 4 {
                let b = content.as_bytes()[j];
                if b == b' ' { j += 1; removed += 1; }
                else if b == b'\t' { j += 1; removed = 1; break; }
                else { break; }
            }
            out.push_str(&content[j..{
                let mut k = j;
                while k < content.len() && content.as_bytes()[k] != b'\n' { k += 1; }
                k
            }]);
            let line_end = {
                let mut k = j;
                while k < content.len() && content.as_bytes()[k] != b'\n' { k += 1; }
                k
            };
            if line_end < content.len() && content.as_bytes()[line_end] == b'\n' {
                // out.push('\n');
            }
            if removed > 0 { changed_lines += 1; removed_sum += removed; }
            i = if line_end < content.len() { line_end + 1 } else { line_end };
        }
        out.push_str(&content[le..]);
        new_content = out;
        // Adjust selection: start moves back by removed on first line if caret after removal point
        let first_line_removed = {
            let mut r = 0usize;
            let mut j = ls; let mut removed = 0usize;
            while j < content.len() && removed < 4 {
                let b = content.as_bytes()[j];
                if b == b' ' { j += 1; removed += 1; }
                else if b == b'\t' { j += 1; removed = 1; break; }
                else { break; }
            }
            r = removed;
            r
        };
        new_sel_start = start.saturating_sub(first_line_removed);
        // total removed across all lines in selection
        new_sel_end = end.saturating_sub(removed_sum);
        true
    } else if key == "(" || key == "{" || key == "[" {
        // auto insert matching closing bracket, but don't be persistent if user deletes it
        let (open, close) = match key.as_str() {
            "(" => ('(', ')'),
            "{" => ('{', '}'),
            "[" => ('[', ']'),
            _ => (' ', ' '),
        };
        let next_ch_opt = content[end..].chars().next();
        if start == end {
            // no selection
            if let Some(nc) = next_ch_opt {
                if nc == close {
                    // If the next char is already the expected closing, just insert the opening and move caret
                    new_content = format!("{}{}{}", &content[..start], open, &content[end..]);
                    new_sel_start = start + open.len_utf8();
                    new_sel_end = new_sel_start;
                    true
                } else {
                    // Insert pair and place caret between them
                    new_content = format!("{}{}{}{}", &content[..start], open, close, &content[end..]);
                    new_sel_start = start + open.len_utf8();
                    new_sel_end = new_sel_start;
                    true
                }
            } else {
                // end of file, just insert pair
                new_content = format!("{}{}{}{}", &content[..start], open, close, &content[end..]);
                new_sel_start = start + open.len_utf8();
                new_sel_end = new_sel_start;
                true
            }
        } else {
            // wrap selection with pair; keep selection inside the brackets
            new_content = format!("{}{}{}{}{}", &content[..start], open, &content[start..end], close, &content[end..]);
            new_sel_start = start + open.len_utf8();
            new_sel_end = end + open.len_utf8();
            true
        }
    } else if key == "Enter" {
        // auto-indent
        let ls = line_start_at(&content, start);
        let prefix = whitespace_prefix_at(&content, ls);
        let mut add_indent = false;
        if let Some(ch) = prev_non_ws_char_before(&content, start) {
            if matches!(lang, "rust" | "typescript" | "c" | "java" | "c_sharp") && ch == '{' { add_indent = true; }
            if lang == "python" && ch == ':' { add_indent = true; }
        }
        let mut insert = String::from("\n");
        insert.push_str(&prefix);
        if add_indent { insert.push_str(indent_unit); }
        new_content = format!("{}{}{}", &content[..start], insert, &content[end..]);
        new_sel_start = start + insert.len();
        new_sel_end = new_sel_start;
        true
    } else if (ctrl || meta) && key == "/" {
        // toggle comment on selected lines
        let token = line_comment_for_language(lang);
        let ls = line_start_at(&content, start);
        let le = line_end_at(&content, end);
        // Detect if all selected lines already commented
        let mut i = ls;
        let bytes = content.as_bytes();
        let mut all_commented = true;
        while i < le {
            // skip leading whitespace
            let mut j = i;
            while j < content.len() && (bytes[j] == b' ' || bytes[j] == b'\t') { j += 1; }
            if content[j..].starts_with(token) {
                // ok
            } else {
                all_commented = false;
                break;
            }
            // move to next line
            while j < content.len() && bytes[j] != b'\n' { j += 1; }
            i = if j < content.len() { j + 1 } else { j };
        }
        // Build output
        let mut out = String::with_capacity(content.len() + 8);
        out.push_str(&content[..ls]);
        i = ls;
        let mut delta: isize = 0;
        while i < le {
            // find line leading whitespace
            let mut j = i;
            while j < content.len() && (bytes[j] == b' ' || bytes[j] == b'\t') { j += 1; }
            if all_commented {
                // remove one token instance
                if content[j..].starts_with(token) {
                    out.push_str(&content[i..j]);
                    out.push_str(&content[(j + token.len())..{
                        let mut k = j + token.len();
                        while k < content.len() && bytes[k] != b'\n' { k += 1; }
                        k
                    }]);
                    delta -= token.len() as isize;
                } else {
                    out.push_str(&content[i..{
                        let mut k = i;
                        while k < content.len() && bytes[k] != b'\n' { k += 1; }
                        k
                    }]);
                }
            } else {
                // insert token after leading whitespace
                out.push_str(&content[i..j]);
                out.push_str(token);
                out.push_str(&content[j..{
                    let mut k = j;
                    while k < content.len() && bytes[k] != b'\n' { k += 1; }
                    k
                }]);
                delta += token.len() as isize;
            }
            // copy newline if present
            let mut line_end = j;
            while line_end < content.len() && bytes[line_end] != b'\n' { line_end += 1; }
            // if line_end < content.len() && bytes[line_end] == b'\n' { out.push('\n'); }
            i = if line_end < content.len() { line_end + 1 } else { line_end };
        }
        out.push_str(&content[le..]);
        new_content = out;
        if all_commented {
            // selection shrinks
            new_sel_start = start.saturating_sub(token.len());
            new_sel_end = (end as isize + delta) as usize;
        } else {
            new_sel_start = start + token.len();
            new_sel_end = (end as isize + delta) as usize;
        }
        true
    } else if key == "Backspace" {

        //TODO
        false
    } else {
        false
    };

    if handled {
        let updated = apply_with_undo(&path, new_content)?;
        Ok(KeyEventResult { content: updated, selection_start: new_sel_start, selection_end: new_sel_end })
    } else {
        // return unchanged
        Ok(KeyEventResult { content, selection_start: start, selection_end: end })
    }
}
