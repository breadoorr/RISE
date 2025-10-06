use std::fs;
use std::path::Path;

enum FileActions {
    Rename,
    Delete,
    Copy,
    Move,
    CopyPath,
}

#[derive(Debug)]
enum FolderActions {
    CreateNewFile,
    CreateNewFolder,
    Rename,
    Delete,
    Copy,
    Move,
    CopyPath,
}
impl From<String> for FolderActions {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "new file" => FolderActions::CreateNewFile,
            "new folder" => FolderActions::CreateNewFolder,
            "delete" => FolderActions::Delete,
            "copy" => FolderActions::Copy,
            "move" => FolderActions::Move,
            "copy path" => FolderActions::CopyPath,
            "rename" => FolderActions::Rename,
            _ => FolderActions::CreateNewFile,
        }
    }
}

impl From<String> for FileActions {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "delete" => FileActions::Delete,
            "copy" => FileActions::Copy,
            "move" => FileActions::Move,
            "copy path" => FileActions::CopyPath,
            "rename" => FileActions::Rename,
            _ => FileActions::Copy,
        }
    }
}

impl From<FileActions> for String {
    fn from(s: FileActions) -> Self {
        match s {
            FileActions::Rename => String::from("Rename"),
            FileActions::Delete => String::from("Delete"),
            FileActions::Copy => String::from("Copy"),
            FileActions::Move => String::from("Move"),
            FileActions::CopyPath => String::from("Copy Path"),
        }
    }
}

impl From<FolderActions> for String {
    fn from(s: FolderActions) -> Self {
        match s {
            FolderActions::CreateNewFile => String::from("New File"),
            FolderActions::CreateNewFolder => String::from("New Folder"),
            FolderActions::Delete => String::from("Delete"),
            FolderActions::Copy => String::from("Copy"),
            FolderActions::Move => String::from("Move"),
            FolderActions::CopyPath => String::from("Copy Path"),
            FolderActions::Rename => String::from("Rename"),
        }
    }
}

const FILE_ACTIONS: [FileActions; 5] = [FileActions::Rename, FileActions::Delete, FileActions::Copy, FileActions::Move, FileActions::CopyPath];
const FOLDER_ACTIONS: [FolderActions; 6] = [FolderActions::CreateNewFile, FolderActions::CreateNewFolder, FolderActions::Delete, FolderActions::Copy, FolderActions::Move, FolderActions::CopyPath];

#[tauri::command]
pub fn get_actions(is_dir: bool) -> Vec<String> {
    if is_dir {
        return FOLDER_ACTIONS.map(|x| x.into()).into();
    }

    FILE_ACTIONS.map(|x| x.into()).into()
}

#[tauri::command]
pub fn perform_action(is_dir: bool, action: String, path: String) {
    println!("Performing action: {} on path: {}, type: {}", action, path, is_dir);
    if is_dir {
        let act = FolderActions::from(action.clone());

        println!("action: {:?}", act);

        match act {
            FolderActions::CreateNewFile => {
                create_file(&path);
            }
            FolderActions::CreateNewFolder => {
                create_folder(&path);
            }
            FolderActions::Delete => {
                delete_folder(&path);
            }
            FolderActions::Copy => {
                copy(&path);
            }
            FolderActions::Move => {
                move_folder(&path);
            }
            FolderActions::CopyPath => {
                copy_path(&path);
            }
            FolderActions::Rename => {
                rename(&path);
            }
        }
    }

    let act = FileActions::from(action);
    match act {
        FileActions::Delete => {
            delete_file(&path);
        }
        FileActions::Copy => {
            copy(&path);
        }
        FileActions::Move => {
            move_file(&path);
        }
        FileActions::CopyPath => {
            copy_path(&path);
        }
        FileActions::Rename => {
            rename(&path);
        }
    }
}

fn create_file(path: &String) {
    let new_path = Path::new(path).join("NewFile.js");
    fs::write(new_path, "//Created in RISE").map_err(|e| format!("Failed to create new file: {}", e)).expect("Failed to create new file");
}

fn create_folder(path: &String) {
    let new_path = Path::new(path).join("New Folder");
    fs::create_dir(new_path).map_err(|e| format!("Failed to create new directory: {}", e)).expect("Failed to create new directory");
}

fn delete_folder(path: &String) {
    let new_path = Path::new(path);
    fs::remove_dir_all(new_path).map_err(|e| format!("Failed to delete directory: {}", e)).expect("Failed to delete directory files");
    fs::remove_dir(new_path).map_err(|e| format!("Failed to delete directory: {}", e)).expect("Failed to remove directory");
}

fn delete_file(path: &String) {
    let new_path = Path::new(path);
    fs::remove_file(new_path).map_err(|e| format!("Failed to delete directory: {}", e)).expect("Failed to delete directory");
}

fn copy(path: &String) {
    let new_path = Path::new(path);
    fs::copy(new_path, new_path).map_err(|e| format!("Failed to copy file: {}", e)).expect("Failed to copy file");
}

fn move_file(path: &String) {}

fn move_folder(path: &String) {}

fn copy_path(path: &String) {}

fn rename(path: &String) {}

