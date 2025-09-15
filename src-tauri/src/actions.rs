enum FileActions {
    Rename,
    Delete,
    Copy,
    Move,
    CopyPath,
}

enum FolderActions {
    CreateNewFile,
    CreateNewFolder,
    Rename,
    Delete,
    Copy,
    Move,
    CopyPath,
}

#[tauri::command]
pub fn get_actions(is_dir: bool) -> Vec<String> {
    if is_dir {
        return vec![String::from("New File"), String::from("New Folder")];
    }
    vec![String::from("Rename"), String::from("Delete"), String::from("Copy"), String::from("Move"), String::from("Copy Path")]
}