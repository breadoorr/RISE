use crate::commands::FileEntry;
use std::fs;
use std::path::Path;

#[derive(Debug)]
enum Actions {
    RenameFile,
    DeleteFile,
    CopyFile,
    MoveFile,
    CopyFilePath,
    CreateNewFile,
    CreateNewFolder,
    RenameFolder,
    DeleteFolder,
    CopyFolder,
    MoveFolder,
    CopyFolderPath,
}
impl From<String> for Actions {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "new file" => Actions::CreateNewFile,
            "new folder" => Actions::CreateNewFolder,
            "delete folder" => Actions::DeleteFolder,
            "copy folder" => Actions::CopyFolder,
            "move folder" => Actions::MoveFolder,
            "copy folder path" => Actions::CopyFolderPath,
            "rename folder" => Actions::RenameFolder,
            "delete file" => Actions::DeleteFile,
            "copy file" => Actions::CopyFile,
            "move file" => Actions::MoveFile,
            "copy file path" => Actions::CopyFilePath,
            "rename file" => Actions::RenameFile,
            _ => Actions::RenameFile,
        }
    }
}

impl From<Actions> for String {
    fn from(s: Actions) -> Self {
        match s {
            Actions::RenameFile => String::from("Rename File"),
            Actions::DeleteFile => String::from("Delete File"),
            Actions::CopyFile => String::from("Copy File"),
            Actions::MoveFile => String::from("Move File"),
            Actions::CopyFilePath => String::from("Copy File Path"),
            Actions::CreateNewFile => String::from("New File"),
            Actions::CreateNewFolder => String::from("New Folder"),
            Actions::DeleteFolder => String::from("Delete Folder"),
            Actions::CopyFolder => String::from("Copy Folder"),
            Actions::MoveFolder => String::from("Move Folder"),
            Actions::CopyFolderPath => String::from("Copy Folder Path"),
            Actions::RenameFolder => String::from("Rename Folder"),
        }
    }
}

const FILE_ACTIONS: [Actions; 5] = [
    Actions::RenameFile,
    Actions::DeleteFile,
    Actions::CopyFile,
    Actions::MoveFile,
    Actions::CopyFilePath,
];

const FOLDER_ACTIONS: [Actions; 7] = [
    Actions::CreateNewFile,
    Actions::CreateNewFolder,
    Actions::RenameFolder,
    Actions::DeleteFolder,
    Actions::CopyFolder,
    Actions::MoveFolder,
    Actions::CopyFolderPath,
];

#[tauri::command]
pub fn get_actions(is_dir: bool) -> Vec<String> {
    if is_dir {
        return FOLDER_ACTIONS.map(|x| x.into()).into();
    }

    FILE_ACTIONS.map(|x| x.into()).into()
}

#[tauri::command]
pub fn perform_action(file: FileEntry, action: String, new_name: String) {
    println!(
        "Performing action: {} on path: {}, type: {}",
        action, file.path, file.is_dir
    );
    let act = Actions::from(action.clone());

    println!("action: {:?}", act);

    match act {
        Actions::CreateNewFile => {
            create_file(&file.path);
            return;
        }
        Actions::CreateNewFolder => {
            create_folder(&file.path);
            return;
        }
        Actions::DeleteFolder => {
            delete_folder(&file.path);
        }
        Actions::CopyFolder => {
            // copy(&file.path);
        }
        Actions::MoveFolder => {
            if !new_name.is_empty() {
                move_folder(&file.path, &new_name);
            }
        }
        Actions::CopyFolderPath => {
            copy_path(&file.path);
        }
        Actions::RenameFolder => {
            if !new_name.is_empty() {
                rename(&file.path, &new_name);
            }
        }
        Actions::DeleteFile => {
            delete_file(&file.path);
        }
        Actions::CopyFile => {
            // copy(&file.path);
        }
        Actions::MoveFile => {
            if !new_name.is_empty() {
                move_file(&file.path, &new_name);
            }
        }
        Actions::CopyFilePath => {
            copy_path(&file.path);
        }
        Actions::RenameFile => {
            if !new_name.is_empty() {
                rename(&file.path, &new_name);
            }
        }
    }
}

fn create_file(path: &String) {
    println!("Creating file: {}", path);
    let new_path = Path::new(path);
    fs::write(new_path, "//Created in RISE")
        .map_err(|e| format!("Failed to create new file: {}", e))
        .expect("Failed to create new file");
}

fn create_folder(path: &String) {
    let new_path = Path::new(path);
    fs::create_dir(new_path)
        .map_err(|e| format!("Failed to create new directory: {}", e))
        .expect("Failed to create new directory");
}

fn delete_folder(path: &String) {
    let new_path = Path::new(path);
    fs::remove_dir_all(new_path)
        .map_err(|e| format!("Failed to delete directory: {}", e))
        .expect("Failed to delete directory files");
    fs::remove_dir(new_path)
        .map_err(|e| format!("Failed to delete directory: {}", e))
        .expect("Failed to remove directory");
}

fn delete_file(path: &String) {
    let new_path = Path::new(path);
    fs::remove_file(new_path)
        .map_err(|e| format!("Failed to delete directory: {}", e))
        .expect("Failed to delete directory");
}

fn copy(path: &String) {
    let new_path = Path::new(path);
    fs::copy(new_path, new_path)
        .map_err(|e| format!("Failed to copy file: {}", e))
        .expect("Failed to copy file");
}

fn move_file(path: &String, n_path: &String) {
    fs::rename(path, n_path)
        .map_err(|e| format!("Failed to rename file: {}", e))
        .expect("Failed to rename file");
    println!("Renamed file: {} to {}", path, n_path);
}

fn move_folder(path: &String, n_path: &String) {
    fs::rename(path, n_path)
        .map_err(|e| format!("Failed to rename file: {}", e))
        .expect("Failed to rename file");
    println!("Renamed folder: {} to {}", path, n_path);
}

fn copy_path(path: &String) {}

fn rename(path: &String, n_path: &String) {
    // let path = Path::new(path);
    // let new_path = Path::new(n_path);
    fs::rename(path, n_path)
        .map_err(|e| format!("Failed to rename file: {}", e))
        .expect("Failed to rename file");
    println!("Renamed file: {} to {}", path, n_path);
}
