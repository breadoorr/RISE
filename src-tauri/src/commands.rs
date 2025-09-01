use std::env;
use std::fs;
use std::path::Path;
use serde::Serialize;
use hostname;
use std::process::Command;
// tree-sitter imports for backend syntax highlighting
use tree_sitter::{Parser, Tree};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use crate::highlight::{get_language_object, collect, escape_html, calculate_edit};

lazy_static! {
    static ref PARSERS: Mutex<HashMap<String, Parser>> = Mutex::new(HashMap::new());
    static ref TREES: Mutex<HashMap<String, Tree>> = Mutex::new(HashMap::new());
    static ref FILE_CONTENTS: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
}

#[derive(Serialize)]
pub struct HighlightResult {
    pub html: String,
}


fn detect_language(param_lang: Option<String>, filename: Option<String>) -> String {
    if let Some(l) = param_lang {
        return l.to_lowercase();
    }
    if let Some(f) = filename {
        let fl = f.to_lowercase();
        if fl.ends_with(".js") || fl.ends_with(".mjs") || fl.ends_with(".cjs") || fl.ends_with(".jsx") {
            return "javascript".to_string();
        }
    }
    "plaintext".to_string()
}

#[tauri::command]
pub fn highlight_ast(code: String, language: String, path: String) -> Result<Vec<(usize, usize, String)>, String> {
    let mut parsers = PARSERS.lock().unwrap();
    let mut trees = TREES.lock().unwrap();
    let mut file_contents = FILE_CONTENTS.lock().unwrap();

    if !parsers.contains_key(&language) {
        let mut parser = Parser::new();
        let lang = get_language_object(&language);
        parser.set_language(&lang).map_err(|e| e.to_string())?;
        parsers.insert(language.clone(), parser);
    }

    let old_code = file_contents.get(&path).cloned().unwrap_or_default();

    if let Some(tree) = trees.get_mut(&path) {
        if let Some(edit) = calculate_edit(&old_code, &code) {
            tree.edit(&edit);
        }
    }

    let parser = parsers.get_mut(&language).unwrap();
    let old_tree_ref = trees.get(&path);
    let new_tree = parser.parse(&code, old_tree_ref);

    if let Some(tree) = new_tree {
        let mut results = Vec::with_capacity(code.len() / 8);
        collect(tree.root_node(), &code, &mut results);
        trees.insert(path.clone(), tree);
        file_contents.insert(path.clone(), code);
        Ok(results)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub fn highlight_html(
    code: String,
    language: String,
    matches: Vec<usize>,
    query_len: usize,
    path: String,
) -> String {
    if code.is_empty() {
        return String::new();
    }
    if code.len() > 500_000 {
        return escape_html(&code).replace("\n\n", "\n<span class=\"empty-line\"> </span>\n");
    }

    let spans = match highlight_ast(code.clone(), language, path) {
        Ok(spans) => spans,
        Err(_) => return escape_html(&code).replace("\n\n", "\n<span class=\"empty-line\"> </span>\n"),
    };

    let mut html = String::with_capacity(code.len() * 2);
    let mut last_index: usize = 0;

    let match_set: HashSet<(usize, usize)> = if query_len > 0 {
        matches.into_iter().map(|m| (m, m + query_len)).collect()
    } else {
        HashSet::new()
    };

    let mut spans_sorted = spans;
    spans_sorted.sort_by_key(|(s, _e, _k)| *s);

    for (start, end, kind) in spans_sorted.into_iter() {
        if start > last_index {
            let plain = &code[last_index..start];
            html.push_str(&escape_html(plain));
        }
        if end <= code.len() && start < end {
            let raw = &code[start..end];
            let escaped = escape_html(raw);
            let is_match = match_set.contains(&(start, end));
            let color_opt = color_for_kind(&kind);
            if let Some(color) = color_opt {
                if is_match {
                    html.push_str(&format!("<span class=\"token find-match\" style=\"color:{}\">{}</span>", color, escaped));
                } else {
                    html.push_str(&format!("<span class=\"token\" style=\"color:{}\">{}</span>", color, escaped));
                }
            } else {
                if is_match {
                    html.push_str(&format!("<span class=\"token find-match\">{}</span>", escaped));
                } else {
                    html.push_str(&format!("<span class=\"token\">{}</span>", escaped));
                }
            }
        }
        last_index = end.min(code.len());
    }

    if last_index < code.len() {
        html.push_str(&escape_html(&code[last_index..]));
    }

    html.replace("\n\n", "\n<span class=\"empty-line\"> </span>\n")
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


// Mapping from token kind to color for inline styling
fn color_for_kind(kind: &str) -> Option<&'static str> {
    match kind {
        // Comments
        "comment" => Some("#7f848e"),
        // Strings (various languages)
        "string" | "string_literal" | "template_string" | "raw_string_literal" | "interpreted_string_literal" | "char_literal" => Some("#54790d"),
        // Numbers
        "number" | "integer" | "float" | "number_literal" | "float_literal" | "decimal_integer_literal" => Some("#d19a66"),
        // Regex
        "regex" => Some("#e06c75"),
        // Types (where available)
        "type_identifier" | "type" => Some("#56b6c2"),
        _ => {
            if is_keyword(kind) { Some("#c678dd") } else { None }
        }
    }
}

fn is_keyword(kind: &str) -> bool {
    match kind {
        // Common JS/TS
        "function" | "return" | "if" | "else" | "for" | "while" | "do" | "switch" | "case" | "default" |
        "break" | "continue" | "const" | "let" | "var" | "class" | "extends" | "new" | "try" | "catch" |
        "finally" | "throw" | "import" | "from" | "export" | "as" | "in" | "of" | "instanceof" | "typeof" |
        "delete" | "void" | "yield" | "await" | "with" | "interface" | "enum" | "implements" | "readonly" |
        "declare" | "namespace" | "public" | "private" | "protected" | "abstract" | "override" | "static" |
        "get" | "set" | "this" | "super" | "true" | "false" | "null" | "undefined" | "debugger" |
        // Rust
        "fn" | "mut" | "pub" | "struct" | "impl" | "trait" | "where" | "use" | "mod" | "crate" | "super" |
        "self" | "Self" | "match" | "loop" | "move" | "async" | "unsafe" | "extern" | "ref" | "type" | "const" |
        // Include shared control flow for Rust too
        "continue" | "break" |
        // Python
        "def" | "elif" | "lambda" | "global" | "nonlocal" | "pass" | "assert" | "del" | "not" | "and" | "or" |
        "is" | "None" | "True" | "False" | "in" | "raise" | "yield" | "from" | "import" | "as" | "with" |
        // C/C++ common
        "int" | "char" | "float" | "double" | "struct" | "union" | "typedef" | "sizeof" | "goto" | "inline" |
        "signed" | "unsigned" | "short" | "long" | "volatile" | "extern" |
        // Java / C# common
        "package" | "throws" | "boolean" | "byte" | "short" | "long" | "native" | "synchronized" | "strictfp" |
        "transient" | "readonly" | "virtual" | "sealed" | "foreach" |
        // More C#
        "namespace" | "using" | "internal" | "dynamic" | "base" | "operator" | "explicit" | "implicit" | "event" |
        "lock" | "fixed" => true,
        _ => false,
    }
}
