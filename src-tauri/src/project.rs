use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use serde_json::Value as JsonValue;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProjectType {
    #[serde(rename = "NPM")] 
    Npm,
    Rust,
    #[serde(rename = "blank")] 
    Blank,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunConfig {
    pub id: String,
    pub name: String,
    pub command: String,
    pub cwd: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub project_type: ProjectType,
    pub run_configs: Vec<RunConfig>,
}

impl Project {
    pub fn detect(path: &str) -> Project {
        let p = Path::new(path);
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("Project").to_string();
        let (project_type, run_configs) = if has_package_json(p) {
            (ProjectType::Npm, npm_run_configs(p))
        } else if has_cargo_toml(p) {
            (ProjectType::Rust, rust_run_configs(p))
        } else {
            (ProjectType::Blank, blank_run_configs(p))
        };
        Project { name, path: path.to_string(), project_type, run_configs }
    }
}

fn has_package_json(dir: &Path) -> bool {
    dir.join("package.json").exists()
}

fn has_cargo_toml(dir: &Path) -> bool {
    dir.join("Cargo.toml").exists()
}

fn npm_run_configs(dir: &Path) -> Vec<RunConfig> {
    let mut configs: Vec<RunConfig> = Vec::new();
    let pkg_path = dir.join("package.json");
    if let Ok(text) = fs::read_to_string(&pkg_path) {
        if let Ok(json) = serde_json::from_str::<JsonValue>(&text) {
            if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
                for (name, cmd) in scripts.iter() {
                    if let Some(_cmd_str) = cmd.as_str() {
                        configs.push(RunConfig {
                            id: format!("npm:{}", name),
                            name: format!("npm run {}", name),
                            command: format!("npm run {}", name),
                            cwd: dir.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }
    }
    // Always add generic npm commands
    configs.push(RunConfig {
        id: "npm:install".to_string(),
        name: "npm install".to_string(),
        command: "npm install".to_string(),
        cwd: dir.to_string_lossy().to_string(),
    });
    configs
}

fn rust_run_configs(dir: &Path) -> Vec<RunConfig> {
    let cwd = dir.to_string_lossy().to_string();
    vec![
        RunConfig { id: "cargo:run".to_string(), name: "cargo run".to_string(), command: "cargo run".to_string(), cwd: cwd.clone() },
        RunConfig { id: "cargo:build".to_string(), name: "cargo build".to_string(), command: "cargo build".to_string(), cwd: cwd.clone() },
        RunConfig { id: "cargo:test".to_string(), name: "cargo test".to_string(), command: "cargo test".to_string(), cwd },
    ]
}

fn blank_run_configs(dir: &Path) -> Vec<RunConfig> {
    vec![RunConfig {
        id: "blank:run-current-file".to_string(),
        name: "Run current file".to_string(),
        // The frontend can interpret this special command and choose proper runtime
        command: "run_current_file".to_string(),
        cwd: dir.to_string_lossy().to_string(),
    }]
}