import { invoke } from "@tauri-apps/api/tauri";
import type { FileEntry } from "./types";

// Pure helper to load directory contents and normalize fields without holding module state
export async function loadFiles(path: string, level: number = 0): Promise<FileEntry[]> {
  try {
    const dirFiles = await invoke("list_files", { dirPath: path }) as FileEntry[];

    for (const file of dirFiles) {
      file.level = level;
      file.parent_dir = path.split("/").slice(-2, -1)[0] || "";
      if (file.is_dir) {
        file.expanded = false;
        file.children = [];
      }
    }

    return dirFiles as FileEntry[];
  } catch (error) {
    console.error("Error listing files:", error);
    return [] as FileEntry[];
  }
}

// Pure helper that flattens a hierarchical file list into a single array
export function updateAllFiles(files: FileEntry[]): FileEntry[] {
  const all: FileEntry[] = [];
  function flattenFiles(list: FileEntry[]) {
    for (const file of list) {
      all.push(file);
      if (file.is_dir && file.expanded && file.children) {
        flattenFiles(file.children);
      }
    }
  }
  flattenFiles(files);
  return all as FileEntry[];
}
