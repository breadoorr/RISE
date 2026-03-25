use rise_lib::{perform_action, get_actions, FileEntry};
use std::env;
use std::fs;
use std::path::Path;

fn temp_path(name: &str) -> String {
    let suffix: u32 = rand::random();
    env::temp_dir()
        .join(format!("rise_actions_it_{}_{}", name, suffix))
        .to_string_lossy()
        .to_string()
}

fn make_file_entry(path: &str, is_dir: bool) -> FileEntry {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    FileEntry {
        path: path.to_string(),
        name,
        is_dir,
    }
}

#[test]
fn get_actions_for_file_and_folder() {
    let file_actions = get_actions(false);
    assert!(file_actions.contains(&"Rename File".to_string()));
    assert!(file_actions.contains(&"Delete File".to_string()));
    assert!(file_actions.contains(&"Copy File".to_string()));
    assert!(file_actions.contains(&"Copy File Path".to_string()));

    let folder_actions = get_actions(true);
    assert!(folder_actions.contains(&"New File".to_string()));
    assert!(folder_actions.contains(&"New Folder".to_string()));
    assert!(folder_actions.contains(&"Rename Folder".to_string()));
    assert!(folder_actions.contains(&"Delete Folder".to_string()));
    assert!(folder_actions.contains(&"Copy Folder".to_string()));
    assert!(folder_actions.contains(&"Copy Folder Path".to_string()));
}

#[test]
fn create_and_delete_file_via_perform_action() {
    let file_path = temp_path("file");
    let entry = make_file_entry(&file_path, false);

    perform_action(entry, "New File".into(), String::new());
    assert!(Path::new(&file_path).exists());

    let entry = make_file_entry(&file_path, false);
    perform_action(entry, "Delete File".into(), String::new());
    assert!(!Path::new(&file_path).exists());
}

#[test]
fn create_and_delete_folder_via_perform_action() {
    let dir_path = temp_path("dir");
    let entry = make_file_entry(&dir_path, true);

    perform_action(entry, "New Folder".into(), String::new());
    assert!(Path::new(&dir_path).exists());
    assert!(Path::new(&dir_path).is_dir());

    let entry = make_file_entry(&dir_path, true);
    perform_action(entry, "Delete Folder".into(), String::new());
    assert!(!Path::new(&dir_path).exists());
}

#[test]
fn move_and_rename_file_via_perform_action() {
    let src = temp_path("move_src");
    // create the source file first
    {
        let entry = make_file_entry(&src, false);
        perform_action(entry, "New File".into(), String::new());
        assert!(Path::new(&src).exists());
    }

    // Move the file to a new path
    let dst = temp_path("move_dst");
    let entry = make_file_entry(&src, false);
    perform_action(entry, "Move File".into(), dst.clone());
    assert!(!Path::new(&src).exists());
    assert!(Path::new(&dst).exists());

    // Rename the moved file again
    let final_path = temp_path("renamed");
    let entry = make_file_entry(&dst, false);
    perform_action(entry, "Rename File".into(), final_path.clone());
    assert!(!Path::new(&dst).exists());
    assert!(Path::new(&final_path).exists());

    // cleanup
    let _ = fs::remove_file(final_path);
}

#[test]
fn move_folder_via_perform_action() {
    let src_dir = temp_path("dir_move_src");
    // create the source folder first
    {
        let entry = make_file_entry(&src_dir, true);
        perform_action(entry, "New Folder".into(), String::new());
        assert!(Path::new(&src_dir).exists());
        // create a file inside to ensure recursive move
        let inner_file = Path::new(&src_dir).join("inner.txt");
        fs::write(&inner_file, "hi").unwrap();
        assert!(inner_file.exists());
    }

    let dst_dir = temp_path("dir_move_dst");
    let entry = make_file_entry(&src_dir, true);
    perform_action(entry, "Move Folder".into(), dst_dir.clone());

    assert!(!Path::new(&src_dir).exists());
    assert!(Path::new(&dst_dir).exists());
    assert!(Path::new(&dst_dir).join("inner.txt").exists());

    // cleanup
    let _ = fs::remove_dir_all(dst_dir);
}
