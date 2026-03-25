use rise_lib::{Project, ProjectType, RunConfig};
use std::env;
use std::fs;
use std::path::Path;

fn temp_dir(name: &str) -> String {
    let suffix: u32 = rand::random();
    env::temp_dir()
        .join(format!("rise_project_it_{}_{}", name, suffix))
        .to_string_lossy()
        .to_string()
}

#[test]
fn detect_blank_project() {
    let dir = temp_dir("blank");
    fs::create_dir_all(&dir).unwrap();

    let proj = Project::detect(&dir);

    assert_eq!(proj.project_type, ProjectType::Blank);
    assert_eq!(proj.path, dir);
    // Expect one generic run config for blank projects
    assert_eq!(proj.run_configs.len(), 1);
    let rc = &proj.run_configs[0];
    assert_eq!(rc.id, "blank:run-current-file");
    assert_eq!(rc.command, "run_current_file");

    // cleanup
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_rust_project_and_run_configs() {
    let dir = temp_dir("rust");
    fs::create_dir_all(&dir).unwrap();

    // A Cargo.toml file indicates a Rust project
    let cargo_toml = Path::new(&dir).join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname=\"tmp\"\nversion=\"0.1.0\"\n").unwrap();

    let proj = Project::detect(&dir);
    assert_eq!(proj.project_type, ProjectType::Rust);
    assert_eq!(proj.path, dir);

    // Expect cargo run/build/test run configs
    let ids: Vec<_> = proj.run_configs.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"cargo:run"));
    assert!(ids.contains(&"cargo:build"));
    assert!(ids.contains(&"cargo:test"));

    // Validate commands match ids semantics
    fn find<'a>(v: &'a [RunConfig], id: &str) -> &'a RunConfig {
        v.iter().find(|r| r.id == id).unwrap()
    }
    assert_eq!(find(&proj.run_configs, "cargo:run").command, "cargo run");
    assert_eq!(find(&proj.run_configs, "cargo:build").command, "cargo build");
    assert_eq!(find(&proj.run_configs, "cargo:test").command, "cargo test");

    // cleanup
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn detect_npm_project_and_run_configs() {
    let dir = temp_dir("npm");
    fs::create_dir_all(&dir).unwrap();

    // Create a minimal package.json with scripts
    let pkg = Path::new(&dir).join("package.json");
    let pkg_content = r#"{
      "name": "tmp",
      "version": "0.0.1",
      "scripts": {
        "dev": "vite",
        "build": "vite build"
      }
    }"#;
    fs::write(&pkg, pkg_content).unwrap();

    let proj = Project::detect(&dir);
    assert_eq!(proj.project_type, ProjectType::Npm);
    assert_eq!(proj.path, dir);

    // Should include generic npm install and discovered scripts
    let names: Vec<_> = proj.run_configs.iter().map(|r| r.name.as_str()).collect();
    assert!(names.iter().any(|n| *n == "npm run dev"));
    assert!(names.iter().any(|n| *n == "npm run build"));
    assert!(names.iter().any(|n| *n == "npm install"));

    // cleanup
    let _ = fs::remove_dir_all(&dir);
}
